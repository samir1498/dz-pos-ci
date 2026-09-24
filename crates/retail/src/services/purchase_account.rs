//! Where one order stands with its supplier: what it is worth, what of it has
//! arrived, what has been paid on it and what is still owed (plan
//! shop-manual-test-findings T46). The order page showed none of the four,
//! and a shop could only read "30 672,00 with 1 836,00 paid" out of the
//! database.
//!
//! Nothing here is stored. Every figure is read off what already decides it,
//! the way features.md §1 asks of what is still owed on one order: the lines'
//! landed costs for the total, the supplier's ledger for what arrived, the
//! allocations for what was paid. A column holding any of them would be a
//! second answer the day a delivery or a payment lands without it.

use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::purchase::{Purchase, PurchaseLine};
use crate::money::Money;
use crate::services::purchase_landed::running_value;
use crate::services::supplier_debt;

/// One order's four figures, in centimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PurchaseAccount {
    /// What the whole order is worth at the cost it landed at: every line's
    /// landed unit cost times what was ordered, each rounded once the way the
    /// delivery that finishes the line rounds it. What the supplier's account
    /// takes if all of it arrives and none goes back.
    pub total: Money,
    /// What the ledger holds against this order: the deliveries' value less
    /// the returns'. Debt rises on receipt and never on save (features.md
    /// §1), so an order nothing has arrived against is worth nothing here.
    pub received: Money,
    /// What payments have placed on this order, from the allocations. Money
    /// handed over on an order whose goods have not arrived sits on the
    /// balance as credit and lands here with the delivery.
    pub paid: Money,
    /// `received - paid`. Below zero when goods went back after the order was
    /// paid: that is credit the supplier holds, and the screen names it as
    /// one rather than printing a minus.
    pub owed: Money,
}

/// The four figures of `purchase`, whose lines the caller has already read.
pub fn of(
    conn: &mut SqliteConnection,
    shop_id: i32,
    purchase: &Purchase,
    lines: &[PurchaseLine],
) -> Result<PurchaseAccount, CoreError> {
    let mut total = Money::ZERO;
    for line in lines {
        total = total.checked_add(running_value(line, line.qty_ordered_milli)?)?;
    }

    // Only the rows citing this order: an opening balance, a payment and a
    // correction cite none, and a payment's share of this order is the
    // allocation read below, not a row here.
    let mut received = Money::ZERO;
    for entry in supplier_debt::ledger(conn, shop_id, purchase.supplier_id)? {
        if entry.purchase_id == Some(purchase.id) {
            received = received.checked_add(entry.signed()?)?;
        }
    }

    let mut paid = Money::ZERO;
    for allocation in supplier_debt::allocations(conn, shop_id, purchase.id)? {
        paid = paid.checked_add(allocation.amount)?;
    }

    Ok(PurchaseAccount {
        total,
        received,
        paid,
        owed: received.checked_sub(paid)?,
    })
}
