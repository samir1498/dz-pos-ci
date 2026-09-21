//! Cash handed back over the counter (features.md §1; ruling 5 of the
//! 2026-09-20 loop).
//!
//! A reversal may be settled in notes rather than on an account. When it is,
//! that cash is an outgoing on the day it was handed over, by the person who
//! handed it, and the till shift of that person knows about it.
//!
//! Its own service and not a corner of `cash.rs`, which is a read-only module
//! deriving a position from four ledgers: this one writes. Its own service
//! and not a helper inside `avoir.rs`, because two paths reach it — an avoir
//! against a standing facture, and a cancelled cash ticket, which is where an
//! anonymous customer's refund lands because there is no ledger to credit —
//! and a rule kept in one of them is a rule the other skips.
//!
//! What it deliberately does not do: decide whether cash is allowed. Whether
//! a facture that is still owed for may be refunded in notes is a question
//! about that facture, so `services::avoir` asks it; whether a credit sale
//! may be is a question about that sale, so `services::cancellation` asks it.
//! This module writes the row, refuses an amount that is not money, and
//! answers the two sums the cash figures are made of.

use chrono::NaiveDateTime;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::cash_refund::{CashRefund, CashRefundRowWrite};
use crate::money::Money;
use crate::repos::cash_refunds as repo;

/// How the money goes back on a reversal.
///
/// `None` is what every caller wrote before ruling 5 and is what every caller
/// that says nothing still gets: an avoir credits the customer's ledger and a
/// cancelled cash ticket moves goods alone, exactly as they did. `Cash` is
/// the new arm, and it is the caller's statement that notes came out of the
/// drawer — nothing in the file can work that out on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Refund {
    #[default]
    None,
    Cash,
}

impl Refund {
    pub const fn is_cash(self) -> bool {
        matches!(self, Refund::Cash)
    }
}

/// Records that this much cash left the drawer against this document.
///
/// `document_id` is the avoir on the avoir path and the cancelled document on
/// the other: whichever numbered paper a reader is holding when they ask why
/// the drawer is lighter. One per document, held by a unique index, so a
/// retried call is refused by the file rather than paying twice.
///
/// `user_id` is whoever handed the notes over. Not the document's author: a
/// cashier refunding somebody else's ticket is the one whose drawer is short.
///
/// The amount has to be money. A reversal that comes to nothing — a zero
/// line, a facture whose goods all came back on earlier avoirs — hands
/// nothing over, and a row saying zero cash moved would read on the cash
/// position as an event that did not happen.
pub fn hand_over(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    document_id: i32,
    amount: Money,
    at: NaiveDateTime,
) -> Result<CashRefund, CoreError> {
    if amount <= Money::ZERO {
        return Err(CoreError::validation(
            "refund",
            "there is nothing to hand back on this reversal",
        ));
    }
    repo::append(
        conn,
        &CashRefundRowWrite {
            shop_id,
            document_id,
            user_id,
            amount_centimes: amount.as_centimes(),
            refunded_at: at,
        },
    )
}

/// What the shop handed back over a window: the outgoing half of the cash
/// position, read on the day the notes changed hands.
pub fn total_for_shop(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Money, CoreError> {
    repo::total_for_shop(conn, shop_id, from, until)
}

/// What one person handed back over one stretch of the clock: what comes off
/// that drawer's expected figure at the close.
pub fn total_for_user(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Money, CoreError> {
    repo::total_for_user(conn, shop_id, user_id, from, until)
}
