//! Cash handed back over the counter (features.md §1, the cash position).
//!
//! One row per reversal settled in notes instead of on an account: the paper
//! it was handed back against, how much left the drawer, who handed it over
//! and when. The anonymous walk-in is why the table exists at all — a sale
//! with no customer has no ledger to credit, so before this row a shop that
//! gave 3 000 DA back had no record of it anywhere.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::money::Money;
use crate::schema::cash_refunds;

/// A refund as a caller asks for it to be written. The person handing the
/// cash over is not here; it comes from the caller's identity, the way it
/// does on every other write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewCashRefund {
    /// The avoir on the avoir path, the cancelled document on the other.
    pub document_id: i32,
    /// Above zero. The services refuse anything else before the file has to.
    pub amount: Money,
    /// The shop's clock: the moment the notes changed hands, which on a
    /// ticket sold Monday and cancelled Wednesday is Wednesday.
    pub refunded_at: NaiveDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CashRefund {
    pub id: i32,
    pub shop_id: i32,
    pub document_id: i32,
    /// Whoever handed the notes over, which is not always whoever rang the
    /// sale. A till shift subtracts by this column and never by the
    /// document's own `user_id`: cashier B refunding cashier A's ticket is
    /// B's drawer that is short.
    pub user_id: i32,
    pub amount: Money,
    pub refunded_at: NaiveDateTime,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = cash_refunds)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct CashRefundRow {
    pub id: i32,
    pub shop_id: i32,
    pub document_id: i32,
    pub user_id: i32,
    pub amount_centimes: i64,
    pub refunded_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = cash_refunds)]
pub(crate) struct CashRefundRowWrite {
    pub shop_id: i32,
    pub document_id: i32,
    pub user_id: i32,
    pub amount_centimes: i64,
    pub refunded_at: NaiveDateTime,
}

impl From<CashRefundRow> for CashRefund {
    fn from(row: CashRefundRow) -> Self {
        CashRefund {
            id: row.id,
            shop_id: row.shop_id,
            document_id: row.document_id,
            user_id: row.user_id,
            amount: Money::centimes(row.amount_centimes),
            refunded_at: row.refunded_at,
        }
    }
}
