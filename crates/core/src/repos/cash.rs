//! The sums the cash position is made of. Four ledgers, one query each, every
//! one scoped by `shop_id` (rule 3) and every one coalesced to zero, so a day
//! nothing happened on answers zeros rather than nothing.
//!
//! Its own module rather than a handful of functions added to `documents`,
//! `debt` and `supplier_debt`: the four are read together, by one caller, over
//! one range of days, and reading them together is what makes the figure a
//! single answer instead of four that were taken at different moments.
//!
//! The bounds are half open, `>= the first moment of the first day` and
//! `< the first moment of the day after the last`. Both columns are stamped
//! by the shop's clock (`clock::now`, and `debt::append_at` for a movement
//! dated by the document it belongs to), so the comparison is on the same
//! calendar the range was asked on.

use chrono::NaiveDateTime;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Nullable};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::debt::{DebtKind, PaymentMethod};
use crate::models::document::payment_mode_stored;
use crate::models::sql_types::{DocumentKind, DocumentStatus, SupplierDebtKind};
use crate::money::{Money, PaymentMode};
use crate::schema::{debt_ledger, documents, supplier_ledger};

/// What the shop sold and was paid for on the spot, at `total_ttc`.
///
/// Only a ticket and a facture: a proforma is a quotation nobody paid, an
/// avoir is a credit note, and the papers the supply side writes are not
/// sales. Only a document that still stands: an annulled ticket is money that
/// did not stay in the drawer, and the avoir a cancellation issues is not
/// counted either, so the reversal is felt once.
///
/// `total_ttc` and not `net_to_pay`: the droit de timbre a cash facture
/// carries is money the customer hands over too, so the drawer really holds
/// the larger figure. The tax is held out of this column on the task brief's
/// instruction, and the open question is in `services::cash`.
pub fn sales(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
    mode: PaymentMode,
) -> Result<Money, CoreError> {
    let total: Option<i64> = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .filter(documents::kind.eq_any([DocumentKind::Ticket, DocumentKind::Facture]))
        .filter(documents::status.eq(DocumentStatus::Issued))
        .filter(documents::payment_mode.eq(payment_mode_stored(mode)))
        .filter(documents::issued_at.ge(from))
        .filter(documents::issued_at.lt(until))
        .select(sql::<Nullable<BigInt>>("SUM(total_ttc_centimes)"))
        .first(conn)?;
    Ok(Money::centimes(total.unwrap_or(0)))
}

/// Money a customer handed over against what they owed. A payment lowers the
/// debt, so it is the credit column that carries the amount.
pub fn customer_payments(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
    method: PaymentMethod,
) -> Result<Money, CoreError> {
    let total: Option<i64> = debt_ledger::table
        .filter(debt_ledger::shop_id.eq(shop_id))
        .filter(debt_ledger::kind.eq(DebtKind::Payment))
        .filter(debt_ledger::payment_mode.eq(method))
        .filter(debt_ledger::created_at.ge(from))
        .filter(debt_ledger::created_at.lt(until))
        .select(sql::<Nullable<BigInt>>("SUM(credit_centimes)"))
        .first(conn)?;
    Ok(Money::centimes(total.unwrap_or(0)))
}

/// Money the shop handed over to a supplier. The mirror of the above: a
/// payment lowers what the shop owes, so it is a credit row here too.
pub fn supplier_payments(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
    method: PaymentMethod,
) -> Result<Money, CoreError> {
    let total: Option<i64> = supplier_ledger::table
        .filter(supplier_ledger::shop_id.eq(shop_id))
        .filter(supplier_ledger::kind.eq(SupplierDebtKind::Payment))
        .filter(supplier_ledger::payment_mode.eq(method))
        .filter(supplier_ledger::created_at.ge(from))
        .filter(supplier_ledger::created_at.lt(until))
        .select(sql::<Nullable<BigInt>>("SUM(credit_centimes)"))
        .first(conn)?;
    Ok(Money::centimes(total.unwrap_or(0)))
}
