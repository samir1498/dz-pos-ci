//! The cash position (features.md §1, Dashboard). The one place it is
//! computed, and it is computed from the ledgers every time: no column
//! anywhere stores it, so there is nothing to keep in step and nothing that
//! can be right on one screen and wrong on another. The dashboard reads this
//! function; so does the expenses screen.
//!
//! It is a net movement over a stretch of days, not the money in the drawer:
//! the app knows of no opening float and of no count at close, so a day that
//! spent more than it took answers a figure below zero and that is the honest
//! answer rather than a floor.
//!
//! What comes in
//! - Cash sales: a ticket or a facture, still standing, paid cash, at its
//!   `total_ttc`.
//! - Cash against a customer's debt: a `payment` row of `debt_ledger` whose
//!   mode is cash.
//!
//! What goes out
//! - Refunds: nothing. An avoir credits the customer's ledger and brings the
//!   goods back on `return` movements (`services::avoir`); no row anywhere
//!   says the drawer opened for it, and an avoir issued by a cancellation
//!   reverses a sale the sales column has already dropped. The field is here
//!   and reads zero so the day an avoir does pay somebody back in cash, there
//!   is one place to fill in.
//! - Cash to suppliers: a `payment` row of `supplier_ledger` whose mode is
//!   cash.
//! - Expenses: everything the month or the day was filed under, whatever the
//!   category.
//!
//! The card figure has the same shape on the way in, because a shop counting
//! its drawer wants to know what the TPE took that the drawer will never
//! hold. There is no card figure on the way out: what the shop pays a
//! supplier by card is a movement of the bank account, and this function is
//! about the till.
//!
//! An open question, and the only one: the drawer physically holds a cash
//! facture's `net_to_pay`, which is its `total_ttc` plus the droit de timbre.
//! A 10 000,00 cash facture with a stamp of 100,00 leaves 10 100,00 in the
//! till and 10 000,00 in this figure. The sales column reads `total_ttc` on
//! the task brief's instruction, which reads the position as the shop's own
//! takings with the tax it collects for the state held out. To confirm with
//! the comptable.

use chrono::NaiveDate;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::debt::PaymentMethod;
use crate::money::{Money, PaymentMode};
use crate::repos::{cash as repo, expenses as expenses_repo};
use crate::services::clock::Period;

/// Money that came in over the period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Takings {
    /// Sales paid on the spot, at `total_ttc`.
    pub sales: Money,
    /// Money handed over against a debt.
    pub customer_payments: Money,
}

impl Takings {
    pub fn total(&self) -> Result<Money, CoreError> {
        Ok(self.sales.checked_add(self.customer_payments)?)
    }
}

/// Cash that left over the period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outgoings {
    /// Money given back to a customer. Always zero today; see the module
    /// comment.
    pub refunds: Money,
    pub supplier_payments: Money,
    pub expenses: Money,
}

impl Outgoings {
    pub fn total(&self) -> Result<Money, CoreError> {
        Ok(self
            .refunds
            .checked_add(self.supplier_payments)?
            .checked_add(self.expenses)?)
    }
}

/// What the till took and paid out over a day or a month, and the net of the
/// two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CashPosition {
    /// The first and the last day the figure covers, both included, so a
    /// screen shows the range it is reading rather than the one it asked for.
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub cash_in: Takings,
    pub cash_out: Outgoings,
    /// `cash_in` less `cash_out`, checked.
    pub cash: Money,
    pub card_in: Takings,
}

/// The cash position over a day or a month on the shop's calendar.
///
/// Five sums and one subtraction. Nothing here reads a stored figure, and the
/// arithmetic is checked, so a file large enough to overflow answers an error
/// rather than a number.
pub fn position(
    conn: &mut SqliteConnection,
    shop_id: i32,
    period: Period,
) -> Result<CashPosition, CoreError> {
    let (from, to) = period.days();
    // Half open at the top: every moment of `to` counts, and the first moment
    // of the day after does not. `succ_opt` is None only at the end of
    // chrono's calendar, where the day itself is the honest bound.
    let first_moment = from.and_hms_opt(0, 0, 0);
    let after = to.succ_opt().and_then(|d| d.and_hms_opt(0, 0, 0));
    let (Some(first_moment), Some(after)) = (first_moment, after) else {
        return Err(CoreError::validation(
            "day",
            "that day is outside the calendar the shop keeps",
        ));
    };
    let day_format = "%Y-%m-%d";
    let (first_text, last_text) = (
        from.format(day_format).to_string(),
        to.format(day_format).to_string(),
    );

    let cash_in = Takings {
        sales: repo::sales(conn, shop_id, first_moment, after, PaymentMode::Cash)?,
        customer_payments: repo::customer_payments(
            conn,
            shop_id,
            first_moment,
            after,
            PaymentMethod::Cash,
        )?,
    };
    let cash_out = Outgoings {
        // See the module comment: no avoir moves cash in this app.
        refunds: Money::ZERO,
        supplier_payments: repo::supplier_payments(
            conn,
            shop_id,
            first_moment,
            after,
            PaymentMethod::Cash,
        )?,
        expenses: expenses_repo::total_between(conn, shop_id, &first_text, &last_text)?,
    };
    let card_in = Takings {
        sales: repo::sales(conn, shop_id, first_moment, after, PaymentMode::Card)?,
        customer_payments: repo::customer_payments(
            conn,
            shop_id,
            first_moment,
            after,
            PaymentMethod::Card,
        )?,
    };
    let cash = cash_in.total()?.checked_sub(cash_out.total()?)?;
    Ok(CashPosition {
        from,
        to,
        cash_in,
        cash_out,
        cash,
        card_in,
    })
}
