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

/// A product whose cached quantity and ledger sum disagreed at a recount.
/// features.md §1: the quantity on hand is derived from the ledger and
/// cached on the product, so the ledger is the truth and the cache is what
/// the recount writes back; this row is the record of what it moved and by
/// how much.
///
/// The name travels beside the id because the drift list is read by a
/// person, and an id is a number they would have to look up in a table the
/// row does not carry. It is also what the audit entry stores, so reading a
/// past run back needs no join against a product that may since have been
/// renamed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drift {
    pub product_id: i32,
    pub name: String,
    pub cached_milli: i64,
    pub ledger_milli: i64,
}

impl Drift {
    /// What the correction moved the cache by: positive when the ledger
    /// holds more than the cache said.
    ///
    /// Saturating rather than plain: both figures are thousandths of a unit
    /// and no movement this app writes could put them a whole `i64` apart,
    /// but the cached one can be anything a repaired file left behind, and a
    /// number that is merely bounded is a better answer than a till that
    /// stops.
    pub fn difference_milli(&self) -> i64 {
        self.ledger_milli.saturating_sub(self.cached_milli)
    }
}

/// One product as the recount compares it: what the column says it has and
/// what its movements add up to. Crate-internal, because it is the shape the
/// repo hands back and `Drift` is the shape a caller reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Counted {
    pub product_id: i32,
    pub name: String,
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
