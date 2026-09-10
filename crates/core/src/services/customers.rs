//! The customer fiche (features.md §2): the party a facture is made out to,
//! and the party a debt belongs to.
//!
//! A name is required because the buyer block prints it and the list is
//! ordered by it. The identifiers are kept as typed, trimmed, and an emptied
//! one is stored as nothing, the way the store block's are.
//!
//! `party_kind` is asked for, never inferred from whether an RC was typed in:
//! loi 04-02 art. 10 decides ticket against facture by who the buyer is, and
//! `a_company_buyer_without_a_nis_refuses_the_facture_and_burns_no_number` and `a_facture_to_a_consumer_asks_for_a_name_and_an_address_and_nothing_else` ask a different set of fields of a company
//! than of a consumer.

use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::customer::CustomerRowWrite;
use crate::models::debt::{DebtKind, NewDebtEntry};
use crate::money::Money;
use crate::repos::customers as repo;
use crate::repos::documents as documents_repo;
use crate::services::{audit, bounded_field, debt, optional_field};

pub use crate::models::customer::{Customer, NewCustomer, PartyKind};

/// The shop's customers, the active ones first. `search` is a piece of a name
/// or of a phone number; blank is no filter at all, so a search box that has
/// been emptied reads the whole list rather than nothing.
///
/// It is bounded at 200 characters like the fields that are stored, under the
/// name the caller sends it as: a search box nobody bounded builds a LIKE
/// pattern the length of whatever was pasted into it, and no name it could
/// match is that long anyway.
pub fn list(
    conn: &mut SqliteConnection,
    shop_id: i32,
    search: Option<&str>,
) -> Result<Vec<Customer>, CoreError> {
    let search = optional_field("q", search)?;
    repo::list(conn, shop_id, search.as_deref())
}

/// A fiche and what its ledger sums to. The two travel together because the
/// list screen shows the debt beside the limit, and reading them apart would
/// be one query per row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomerWithBalance {
    pub customer: Customer,
    pub balance: Money,
}

/// The list, each fiche carrying its balance. Two queries whatever the number
/// of customers: the fiches, and the ledger summed per customer.
pub fn list_with_balance(
    conn: &mut SqliteConnection,
    shop_id: i32,
    search: Option<&str>,
) -> Result<Vec<CustomerWithBalance>, CoreError> {
    let found = list(conn, shop_id, search)?;
    let balances = debt::balances(conn, shop_id)?;
    Ok(found
        .into_iter()
        .map(|customer| {
            // A customer with no movement is not in the sums, and owes
            // nothing.
            let balance = balances.get(&customer.id).copied().unwrap_or(Money::ZERO);
            CustomerWithBalance { customer, balance }
        })
        .collect())
}

pub fn get_with_balance(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
) -> Result<CustomerWithBalance, CoreError> {
    let customer = repo::get(conn, shop_id, id)?;
    let balance = debt::balance(conn, shop_id, id)?;
    Ok(CustomerWithBalance { customer, balance })
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Customer, CoreError> {
    repo::get(conn, shop_id, id)
}

/// Whether the customer is one of this shop's, mirroring the check a document
/// line makes on its product. A caller that has an id and no fiche asks this
/// before it writes the id anywhere.
pub fn customer_belongs_to_shop(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<bool, CoreError> {
    repo::belongs_to_shop(conn, shop_id, customer_id)
}

/// Opens the fiche, and with it the first row of its ledger when the shop is
/// carrying the customer's debt over from whatever it kept before.
///
/// The opening debt is a movement, not a column: the balance then has one
/// source, and correcting the figure later is an `adjustment` row a comptable
/// can read rather than an edit nobody can see. `update` below never touches
/// it.
pub fn create(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    fields: NewCustomer,
    opening_debt: Option<Money>,
) -> Result<Customer, CoreError> {
    let write = validate(shop_id, &fields)?;
    if let Some(debt) = opening_debt {
        if debt.is_negative() {
            return Err(CoreError::validation(
                "opening_debt",
                "an opening balance the shop owes the customer is not a debt to carry over",
            ));
        }
    }
    // The fiche, its opening movement and the audit entry are one
    // transaction: an opening debt without its customer, or a customer
    // nobody can trace, is the failure this log exists to prevent
    // (features.md §5).
    conn.transaction(|conn| {
        let created = repo::insert(conn, &write)?;
        // Zero is not written: a movement of nothing would sit in every
        // statement the customer is ever handed.
        if let Some(amount) = opening_debt.filter(|d| *d != Money::ZERO) {
            debt::append(
                conn,
                shop_id,
                NewDebtEntry {
                    customer_id: created.id,
                    document_id: None,
                    kind: DebtKind::Opening,
                    debit: amount,
                    credit: Money::ZERO,
                    user_id,
                    note: Some("solde de départ".to_string()),
                },
            )?;
        }
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_CREATE,
                entity: "customer",
                entity_id: Some(created.id),
                before: None,
                after: Some(as_json(&created, opening_debt)),
            },
        )?;
        Ok(created)
    })
}

/// Replaces the fiche. The ledger is untouched: a wrong opening debt is
/// corrected by an `adjustment` movement, never by editing the fiche.
/// `close_reason` is asked for only when the update closes a fiche that is
/// still carrying something: a balance either way, or a document still asking
/// to be paid. Closing says the shop has stopped trading with that customer
/// (features.md §2), and doing it over an open account is either a mistake or
/// a decision somebody took, so the log has to say which. On every other
/// update it is ignored.
pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: i32,
    fields: NewCustomer,
    close_reason: Option<String>,
) -> Result<Customer, CoreError> {
    let write = validate(shop_id, &fields)?;
    let close_reason = optional_field("reason", close_reason.as_deref())?;
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        // Read before the write, because it is what the refusal is about and
        // what the log has to carry: after the update the fiche is closed
        // either way and the figure would say nothing about the decision.
        let closing = before.active && !fields.active;
        let account = closing
            .then(|| open_account(conn, shop_id, id))
            .transpose()?;
        let reason = match &account {
            Some(account) if account.is_open() => match &close_reason {
                Some(reason) => Some(reason.clone()),
                None => {
                    return Err(CoreError::validation(
                        "reason",
                        "this customer still has an account open; say why it is being closed",
                    ))
                }
            },
            _ => None,
        };
        let after = repo::update(conn, shop_id, id, &write)?;
        // One row per call, named for what happened: a close over an open
        // account is not the same event as a change of phone number, and a
        // comptable reading the log for a written-off balance wants to find
        // it by the action rather than by diffing two fiches.
        let (action, extra) = match (&account, &reason) {
            (Some(account), Some(reason)) => (
                audit::ACTION_CLOSE_CUSTOMER,
                Some(serde_json::json!({
                    "close_reason": reason,
                    "balance_centimes": account.balance.as_centimes(),
                    "open_documents": account.open_documents,
                })),
            ),
            _ => (audit::ACTION_UPDATE, None),
        };
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action,
                entity: "customer",
                entity_id: Some(id),
                before: Some(as_json(&before, None)),
                after: Some(with_extra(as_json(&after, None), extra)),
            },
        )?;
        Ok(after)
    })
}

/// What a fiche is still carrying: the balance either way, and how many of
/// its documents are still asking to be paid.
struct OpenAccount {
    balance: Money,
    open_documents: usize,
}

impl OpenAccount {
    /// Money owed, money held, or paper outstanding. A balance of zero with a
    /// document still open is possible and counts: the paper is what the
    /// customer holds in their hand.
    const fn is_open(&self) -> bool {
        !matches!(self.balance.as_centimes(), 0) || self.open_documents > 0
    }
}

fn open_account(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<OpenAccount, CoreError> {
    Ok(OpenAccount {
        balance: debt::balance(conn, shop_id, customer_id)?,
        open_documents: documents_repo::unpaid_of_customer(conn, shop_id, customer_id)?.len(),
    })
}

/// The audited fiche with the close block merged in beside it, so the row
/// reads as one object rather than as a fiche and a note that have to be
/// lined up by whoever opens the log.
fn with_extra(fiche: String, extra: Option<serde_json::Value>) -> String {
    let Some(extra) = extra else { return fiche };
    let (Ok(serde_json::Value::Object(mut fiche)), serde_json::Value::Object(extra)) =
        (serde_json::from_str::<serde_json::Value>(&fiche), extra)
    else {
        return fiche;
    };
    fiche.extend(extra);
    serde_json::Value::Object(fiche).to_string()
}

/// The fiche as the audit log stores it. `opening_debt` is only on a create,
/// and only when there was one: on an update there is no ledger movement to
/// report and the field would say the debt had been set to nothing.
fn as_json(customer: &Customer, opening_debt: Option<Money>) -> String {
    let mut value = serde_json::json!({
        "name": customer.name,
        "party_kind": customer.party_kind.as_str(),
        "phone": customer.phone,
        "address": customer.address,
        "rc": customer.rc,
        "nif": customer.nif,
        "nis": customer.nis,
        "ai": customer.ai,
        "credit_limit_centimes": customer.credit_limit.map(Money::as_centimes),
        "warn_threshold_centimes": customer.warn_threshold.map(Money::as_centimes),
        "notes": customer.notes,
        "active": customer.active,
    });
    if let (Some(object), Some(debt)) = (value.as_object_mut(), opening_debt) {
        object.insert(
            "opening_debt_centimes".to_string(),
            serde_json::json!(debt.as_centimes()),
        );
    }
    value.to_string()
}

fn validate(shop_id: i32, fields: &NewCustomer) -> Result<CustomerRowWrite, CoreError> {
    let name = fields.name.trim();
    if name.is_empty() {
        return Err(CoreError::validation(
            "name",
            "the customer needs a name; it prints in the buyer block",
        ));
    }
    bounded_field("name", name)?;
    for (field, amount) in [
        ("credit_limit", fields.credit_limit),
        ("warn_threshold", fields.warn_threshold),
    ] {
        if amount.is_some_and(Money::is_negative) {
            return Err(CoreError::validation(
                field,
                "an amount a customer may owe is never below zero",
            ));
        }
    }
    Ok(CustomerRowWrite {
        shop_id,
        name: name.to_string(),
        party_kind: fields.party_kind,
        phone: optional_field("phone", fields.phone.as_deref())?,
        address: optional_field("address", fields.address.as_deref())?,
        rc: optional_field("rc", fields.rc.as_deref())?,
        nif: optional_field("nif", fields.nif.as_deref())?,
        nis: optional_field("nis", fields.nis.as_deref())?,
        ai: optional_field("ai", fields.ai.as_deref())?,
        credit_limit_centimes: fields.credit_limit.map(Money::as_centimes),
        warn_threshold_centimes: fields.warn_threshold.map(Money::as_centimes),
        notes: optional_field("notes", fields.notes.as_deref())?,
        active: fields.active,
        // UTC, not the shop's calendar: `created_at` is SQLite's
        // CURRENT_TIMESTAMP, which is UTC, and a fiche whose
        // `updated_at` reads an hour after its own `created_at` is a
        // wrong figure in a row nobody would think to doubt.
        updated_at: chrono::Utc::now().naive_utc(),
    })
}
