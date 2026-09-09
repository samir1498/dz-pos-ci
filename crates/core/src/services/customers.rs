//! The customer fiche (features.md §2): the party a facture is made out to,
//! and the party a debt belongs to.
//!
//! A name is required because the buyer block prints it and the list is
//! ordered by it. The identifiers are kept as typed, trimmed, and an emptied
//! one is stored as nothing, the way the store block's are.
//!
//! `party_kind` is asked for, never inferred from whether an RC was typed in:
//! loi 04-02 art. 10 decides ticket against facture by who the buyer is, and
//! `facture_requires_party_ids` asks a different set of fields of a company
//! than of a consumer.

use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::customer::CustomerRowWrite;
use crate::models::debt::{DebtKind, NewDebtEntry};
use crate::money::Money;
use crate::repos::customers as repo;
use crate::services::{audit, bounded_field, debt, optional_field};

pub use crate::models::customer::{Customer, NewCustomer, PartyKind};

pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<Customer>, CoreError> {
    repo::list(conn, shop_id)
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
pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: i32,
    fields: NewCustomer,
) -> Result<Customer, CoreError> {
    let write = validate(shop_id, &fields)?;
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        let after = repo::update(conn, shop_id, id, &write)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_UPDATE,
                entity: "customer",
                entity_id: Some(id),
                before: Some(as_json(&before, None)),
                after: Some(as_json(&after, None)),
            },
        )?;
        Ok(after)
    })
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
