//! The supplier fiche (features.md §1, Supplier): who the shop buys from,
//! and the party a supplier debt belongs to.
//!
//! The mirror of `services::customers`, minus what only a customer has. There
//! is no `party_kind`, because nothing the shop hands a supplier is a fiscal
//! document it issues, and no credit limit or warning threshold, because
//! those are what a shop grants somebody and here it is the shop being
//! granted.
//!
//! What a customer does not have and a supplier does: the name is unique
//! inside the shop, which features.md §1 asks for and the file enforces. Two
//! fiches under one name would read at a counter as one party carrying two
//! balances.

use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::supplier::SupplierRowWrite;
use crate::models::supplier_debt::{NewSupplierEntry, SupplierDebtKind};
use crate::money::Money;
use crate::repos::suppliers as repo;
use crate::services::{audit, bounded_field, optional_field, supplier_debt};

pub use crate::models::supplier::{NewSupplier, Supplier};

/// The shop's suppliers, the ones it still buys from first. `search` is a
/// piece of a name or of a phone number; blank is no filter at all, so a
/// search box that has been emptied reads the whole list rather than nothing.
///
/// It is bounded at 200 characters like the fields that are stored, under the
/// name the caller sends it as: a search box nobody bounded builds a LIKE
/// pattern the length of whatever was pasted into it.
pub fn list(
    conn: &mut SqliteConnection,
    shop_id: i32,
    search: Option<&str>,
) -> Result<Vec<Supplier>, CoreError> {
    let search = optional_field("q", search)?;
    repo::list(conn, shop_id, search.as_deref())
}

/// A fiche and what its ledger sums to. The two travel together because the
/// list screen shows the debt beside the name, and reading them apart would
/// be one query per row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplierWithBalance {
    pub supplier: Supplier,
    pub balance: Money,
}

/// The list, each fiche carrying its balance. Two queries whatever the number
/// of suppliers: the fiches, and the ledger summed per supplier.
pub fn list_with_balance(
    conn: &mut SqliteConnection,
    shop_id: i32,
    search: Option<&str>,
) -> Result<Vec<SupplierWithBalance>, CoreError> {
    let found = list(conn, shop_id, search)?;
    let balances = supplier_debt::balances(conn, shop_id)?;
    Ok(found
        .into_iter()
        .map(|supplier| {
            // A supplier with no movement is not in the sums, and is owed
            // nothing.
            let balance = balances.get(&supplier.id).copied().unwrap_or(Money::ZERO);
            SupplierWithBalance { supplier, balance }
        })
        .collect())
}

pub fn get_with_balance(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
) -> Result<SupplierWithBalance, CoreError> {
    let supplier = repo::get(conn, shop_id, id)?;
    let balance = supplier_debt::balance(conn, shop_id, id)?;
    Ok(SupplierWithBalance { supplier, balance })
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Supplier, CoreError> {
    repo::get(conn, shop_id, id)
}

/// Whether the supplier is one of this shop's. A caller that has an id and no
/// fiche asks this before it writes the id anywhere (rule 3).
pub fn supplier_belongs_to_shop(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<bool, CoreError> {
    repo::belongs_to_shop(conn, shop_id, supplier_id)
}

/// Opens the fiche, and with it the first row of its ledger when the shop is
/// carrying a debt to this supplier over from whatever it kept before.
///
/// The opening debt is a movement, not a column: the balance then has one
/// source, and correcting the figure later is an `adjustment` row a comptable
/// can read rather than an edit nobody can see. `update` below never touches
/// it.
pub fn create(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    fields: NewSupplier,
    opening_debt: Option<Money>,
) -> Result<Supplier, CoreError> {
    let write = validate(shop_id, &fields)?;
    if opening_debt.is_some_and(|d| d.is_negative()) {
        return Err(CoreError::validation(
            "opening_debt",
            "an opening balance the supplier owes the shop is not a debt to carry over",
        ));
    }
    // The fiche, its opening movement and the audit entry are one
    // transaction: an opening debt without its supplier, or a supplier nobody
    // can trace, is the failure this log exists to prevent (features.md §5).
    conn.transaction(|conn| {
        let created = repo::insert(conn, &write)?;
        // Zero is not written: a movement of nothing would sit in every
        // statement the supplier is ever sent.
        if let Some(amount) = opening_debt.filter(|d| *d != Money::ZERO) {
            supplier_debt::append(
                conn,
                shop_id,
                NewSupplierEntry {
                    supplier_id: created.id,
                    purchase_id: None,
                    kind: SupplierDebtKind::Opening,
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
                entity: "supplier",
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
///
/// `close_reason` is asked for only when the update closes a fiche whose
/// account is still open: a balance either way, or an order still asking to
/// be paid. Closing says the shop has stopped buying from that supplier, and
/// doing it over an open account is either a mistake or a decision somebody
/// took, so the log has to say which. On every other update it is ignored.
pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: i32,
    fields: NewSupplier,
    close_reason: Option<String>,
) -> Result<Supplier, CoreError> {
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
                        "this supplier still has an account open; say why it is being closed",
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
                audit::ACTION_CLOSE_SUPPLIER,
                Some(serde_json::json!({
                    "close_reason": reason,
                    "balance_centimes": account.balance.as_centimes(),
                    "open_purchases": account.open_purchases,
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
                entity: "supplier",
                entity_id: Some(id),
                before: Some(as_json(&before, None)),
                after: Some(with_extra(as_json(&after, None), extra)),
            },
        )?;
        Ok(after)
    })
}

/// Stops the shop buying from this supplier, with the reason the log needs
/// when the account is still open.
///
/// The fiche it writes is the one that is stored, so closing changes exactly
/// one thing. The rule itself lives in `update`: the close route and the
/// fiche's own save reach it through the same check rather than through two
/// that could part company.
pub fn close(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: i32,
    reason: Option<String>,
) -> Result<Supplier, CoreError> {
    let fiche = repo::get(conn, shop_id, id)?;
    if !fiche.active {
        return Err(CoreError::validation(
            "active",
            "this supplier is already closed",
        ));
    }
    update(
        conn,
        shop_id,
        user_id,
        id,
        NewSupplier {
            name: fiche.name,
            phone: fiche.phone,
            address: fiche.address,
            rc: fiche.rc,
            nif: fiche.nif,
            nis: fiche.nis,
            ai: fiche.ai,
            notes: fiche.notes,
            active: false,
        },
        reason,
    )
}

/// What a fiche is still carrying: the balance either way, and how many of
/// its orders are still asking to be paid.
struct OpenAccount {
    balance: Money,
    open_purchases: usize,
}

impl OpenAccount {
    /// Money owed, money advanced, or an order not paid for. A balance of
    /// zero with an order still open is possible and counts: money that
    /// settled no paper leaves the order asking.
    const fn is_open(&self) -> bool {
        !matches!(self.balance.as_centimes(), 0) || self.open_purchases > 0
    }
}

fn open_account(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<OpenAccount, CoreError> {
    Ok(OpenAccount {
        balance: supplier_debt::balance(conn, shop_id, supplier_id)?,
        open_purchases: supplier_debt::open_purchases(conn, shop_id, supplier_id)?.len(),
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
fn as_json(supplier: &Supplier, opening_debt: Option<Money>) -> String {
    let mut value = serde_json::json!({
        "name": supplier.name,
        "phone": supplier.phone,
        "address": supplier.address,
        "rc": supplier.rc,
        "nif": supplier.nif,
        "nis": supplier.nis,
        "ai": supplier.ai,
        "notes": supplier.notes,
        "active": supplier.active,
    });
    if let (Some(object), Some(debt)) = (value.as_object_mut(), opening_debt) {
        object.insert(
            "opening_debt_centimes".to_string(),
            serde_json::json!(debt.as_centimes()),
        );
    }
    value.to_string()
}

fn validate(shop_id: i32, fields: &NewSupplier) -> Result<SupplierRowWrite, CoreError> {
    let name = fields.name.trim();
    if name.is_empty() {
        return Err(CoreError::validation(
            "name",
            "the supplier needs a name; the list is ordered by it and it is unique in the shop",
        ));
    }
    bounded_field("name", name)?;
    Ok(SupplierRowWrite {
        shop_id,
        name: name.to_string(),
        phone: optional_field("phone", fields.phone.as_deref())?,
        address: optional_field("address", fields.address.as_deref())?,
        rc: optional_field("rc", fields.rc.as_deref())?,
        nif: optional_field("nif", fields.nif.as_deref())?,
        nis: optional_field("nis", fields.nis.as_deref())?,
        ai: optional_field("ai", fields.ai.as_deref())?,
        notes: optional_field("notes", fields.notes.as_deref())?,
        active: fields.active,
        // UTC, not the shop's calendar: `created_at` is SQLite's
        // CURRENT_TIMESTAMP, which is UTC, and a fiche whose `updated_at`
        // reads an hour after its own `created_at` is a wrong figure in a row
        // nobody would think to doubt.
        updated_at: chrono::Utc::now().naive_utc(),
    })
}
