//! The stock ledger rows. `qty_milli` is signed: a sale takes stock out, a
//! purchase puts it back. Amounts are `Money`; the `*_centimes` column is the
//! `i64` behind it, converted here so no other layer sees a bare integer.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::money::Money;
use crate::schema::stock_movements;

pub use super::sql_types::MovementKind;

/// A movement as a caller asks for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movement {
    pub product_id: i32,
    pub kind: MovementKind,
    /// Signed thousandths of the unit: negative takes stock out.
    pub qty_milli: i64,
    pub unit_cost: Money,
    pub document_id: Option<i32>,
    pub user_id: i32,
}

/// A movement as it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockMovement {
    pub id: i32,
    pub shop_id: i32,
    pub product_id: i32,
    pub kind: MovementKind,
    pub qty_milli: i64,
    pub unit_cost: Money,
    pub document_id: Option<i32>,
    pub user_id: i32,
    pub created_at: NaiveDateTime,
}

/// A product whose cached quantity and ledger sum disagree. features.md §1:
/// the nightly job re-derives and reports; it never silently repairs, since
/// the difference is the thing a shop owner has to look at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drift {
    pub product_id: i32,
    pub cached_milli: i64,
    pub ledger_milli: i64,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = stock_movements)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct StockMovementRow {
    pub id: i32,
    pub shop_id: i32,
    pub product_id: i32,
    pub kind: MovementKind,
    pub qty_milli: i64,
    pub unit_cost_centimes: i64,
    pub document_id: Option<i32>,
    pub user_id: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = stock_movements)]
pub(crate) struct StockMovementRowWrite {
    pub shop_id: i32,
    pub product_id: i32,
    pub kind: MovementKind,
    pub qty_milli: i64,
    pub unit_cost_centimes: i64,
    pub document_id: Option<i32>,
    pub user_id: i32,
}

impl From<StockMovementRow> for StockMovement {
    fn from(r: StockMovementRow) -> Self {
        StockMovement {
            id: r.id,
            shop_id: r.shop_id,
            product_id: r.product_id,
            kind: r.kind,
            qty_milli: r.qty_milli,
            unit_cost: Money::centimes(r.unit_cost_centimes),
            document_id: r.document_id,
            user_id: r.user_id,
            created_at: r.created_at,
        }
    }
}
