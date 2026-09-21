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
//!   `net_to_pay`. That is what the customer handed over, the droit de timbre
//!   included, and the drawer holds all of it. The stamp inside that figure
//!   comes back beside it as `stamp`, so a screen that wants the shop's own
//!   takings can subtract the tax it is holding for the state; it is a part of
//!   `sales` and never a second figure to add.
//! - Cash against a customer's debt: a `payment` row of `debt_ledger` whose
//!   mode is cash.
//!
//! What goes out
//! - Refunds: nothing. An avoir credits the customer's ledger and brings the
//!   goods back on `return` movements (`services::avoir`); no row anywhere
//!   says the drawer opened for it, and an avoir issued by a cancellation
//!   reverses a sale the sales column has already dropped. The field is here
//!   and reads zero so the day an avoir does pay somebody back in cash, there
//!   is one place to fill in. Until such a row exists this figure is off by
//!   whatever cash a shop actually handed back over the counter, and no test
//!   here can catch that: the file has nothing to compare against.
//!
//! A cancelled sale leaves the day it was sold on and appears on no other. A
//! ticket rung up on Monday and annulled on Wednesday is out of Monday's
//! takings, which is right for Monday's own figure and wrong for the drawer
//! on Wednesday, where the money physically went back; nothing marks the day
//! it left.
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

use chrono::{NaiveDate, NaiveDateTime};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::debt::PaymentMethod;
use crate::money::{Money, PaymentMode};
use crate::repos::cash as repo;
use crate::services::clock::Period;
use crate::services::expenses;

/// Money that came in over the period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Takings {
    /// Sales paid on the spot, at `net_to_pay`: what the customer handed
    /// over, the droit de timbre included.
    pub sales: Money,
    /// The droit de timbre inside `sales`, so a screen can show takings net
    /// of the tax the shop collects for the state. Never added to `sales`:
    /// it is already in it. Zero on the card side, where the app writes no
    /// stamp at all.
    pub stamp: Money,
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
    // Half open at the top, and worked out by the period itself so the
    // dashboard reading the same days compares against the same two moments.
    let (first_moment, after) = period.moments()?;

    let (cash_sales, cash_stamp) =
        repo::sales(conn, shop_id, first_moment, after, PaymentMode::Cash)?;
    let cash_in = Takings {
        sales: cash_sales,
        stamp: cash_stamp,
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
        expenses: expenses::total_between(conn, shop_id, from, to)?,
    };
    let (card_sales, card_stamp) =
        repo::sales(conn, shop_id, first_moment, after, PaymentMode::Card)?;
    let card_in = Takings {
        sales: card_sales,
        stamp: card_stamp,
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

/// The cash one person took over one stretch of the clock: their own cash
/// sales at `net_to_pay`, and the cash they were handed against a customer's
/// debt. The window is half open, `from <= the moment < until`, so a sale rung
/// at the second a shift opened is in it and one rung at the second it closed
/// is not.
///
/// A second, narrower question put to this module and never a new input to
/// `position` above. `position` keeps summing the whole shop from sales, both
/// ledgers and expenses; a shift asks what one cashier should be holding. That
/// is what stops a 15 000 DA handover from reading as the shop losing 3 000 on
/// a day it took 12 000, and it is why the dashboard reads no shift at all.
///
/// What is deliberately not in it:
///
/// - Another cashier's takings. Two people hold overlapping shifts on purpose
///   (each drawer is physically their own), so the filter is the point of the
///   function rather than a refinement of it.
/// - Card sales. A card sale never reaches a drawer.
/// - Expenses and supplier payments. Both are `commit_money` work, which a
///   cashier does not hold: they are the shop's money going out, not this
///   drawer's. `repos::expenses::total_between` also filters `expense_date`,
///   a date and not a moment, so there is no shift-sized slice of it to ask
///   for.
///
/// `Takings` and not a type of its own: the same three figures, about one
/// person instead of the shop, and `total()` is the same checked addition.
/// `stamp` travels for the same reason it does above, so a close screen can
/// say how much of the drawer is tax being held for the state.
pub fn takings_for(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Takings, CoreError> {
    let (sales, stamp) =
        repo::sales_by_user(conn, shop_id, user_id, from, until, PaymentMode::Cash)?;
    Ok(Takings {
        sales,
        stamp,
        customer_payments: repo::customer_payments_by_user(
            conn,
            shop_id,
            user_id,
            from,
            until,
            PaymentMethod::Cash,
        )?,
    })
}
