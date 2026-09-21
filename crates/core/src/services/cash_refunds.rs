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
use crate::models::document::Document;
use crate::money::{Money, PaymentMode};
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

    /// How a reader of the audit log is told the money went back. Written
    /// into the `after` block of both reversal actions, because a credit note
    /// that opened the drawer and one that credited an account are otherwise
    /// the same row, and the drawer is the one somebody answers for.
    ///
    /// `ledger` and not `none`: on the path that writes it, the account is
    /// where the money went. A cancelled cash sale settled this way handed
    /// nothing back at all, and the `remaining_debt_centimes` beside it in
    /// the same block is what says so.
    pub const fn as_str(self) -> &'static str {
        match self {
            Refund::None => "ledger",
            Refund::Cash => "cash",
        }
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

/// The most this paper may still hand back over the counter.
///
/// Cash handed back may not exceed what was actually paid in against the
/// paper, less whatever has already gone back on it or on an avoir written
/// against it. A cash or card document was settled when it was issued, so the
/// whole `net_to_pay` came in; a credit document put the money on an account,
/// so what came in is the payments allocated to it and nothing else.
///
/// Card and not only cash on the paid-in side, because a shop settling a
/// returned card sale out of the drawer is ordinary: the card takings keep
/// the sale and the cash outgoing says where the notes went.
///
/// What it deliberately does not read is `remaining_debt`. A downward
/// adjustment zeroes that column with nothing coming in (features.md §3), so
/// a written-off facture reads as settled, and a guard standing on it would
/// hand notes over for goods nobody ever paid for.
pub fn still_to_hand_back(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document: &Document,
) -> Result<Money, CoreError> {
    let paid_on_the_spot = match document.payment_mode {
        PaymentMode::Credit => None,
        PaymentMode::Cash | PaymentMode::Card => Some(document.totals.net_to_pay),
    };
    repo::still_to_hand_back(conn, shop_id, document.id, paid_on_the_spot)
}
