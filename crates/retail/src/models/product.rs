//! The product a service returns, and the diesel rows it maps to. Prices
//! are `Money`; the `*_centimes` columns are the `i64` behind them, and the
//! conversion happens here so no other layer sees a bare integer price.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::error::CoreError;
use crate::money::{Bps, Money};
use crate::schema::products;

pub use super::contenance::{display_name, Contenance, ContenanceUnit};
pub use super::sql_types::Unit;

/// A product as the rest of the app sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Product {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub barcode: Option<String>,
    pub category_id: Option<i32>,
    pub unit: Unit,
    pub cost: Money,
    pub selling: Money,
    pub wholesale: Option<Money>,
    pub qty_on_hand_milli: i64,
    pub low_stock_at_milli: i64,
    pub rate_bps: Bps,
    pub active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    /// The pack size, when the shop gave one (T13).
    pub contenance: Option<Contenance>,
}

/// A product as a caller asks for it. `barcode` blank means "number it for
/// me" (features.md §1); `rate_bps` unset means "take the category's".
#[derive(Debug, Clone)]
pub struct NewProduct {
    pub name: String,
    pub barcode: Option<String>,
    pub category_id: Option<i32>,
    pub unit: Unit,
    pub cost: Money,
    pub selling: Money,
    pub wholesale: Option<Money>,
    pub qty_on_hand_milli: i64,
    pub low_stock_at_milli: i64,
    pub rate_bps: Option<Bps>,
    pub active: bool,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = products)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct ProductRow {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub barcode: Option<String>,
    pub category_id: Option<i32>,
    pub unit: Unit,
    pub cost_centimes: i64,
    pub selling_centimes: i64,
    pub wholesale_centimes: Option<i64>,
    pub qty_on_hand_milli: i64,
    pub low_stock_at_milli: i64,
    pub rate_bps: i32,
    pub active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub contenance_milli: Option<i64>,
    pub contenance_unit: Option<String>,
}

// `treat_none_as_null`: diesel's changeset skips a `None` field by default,
// so an update that removed the category or the wholesale price silently
// kept the old value. Every field here is the whole new row; a `None` is a
// NULL. The service fills `barcode` before the write, so it is never None
// on an update.
#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = products, treat_none_as_null = true)]
pub(crate) struct ProductRowWrite {
    pub shop_id: i32,
    pub name: String,
    pub barcode: Option<String>,
    pub category_id: Option<i32>,
    pub unit: Unit,
    pub cost_centimes: i64,
    pub selling_centimes: i64,
    pub wholesale_centimes: Option<i64>,
    pub qty_on_hand_milli: i64,
    pub low_stock_at_milli: i64,
    pub rate_bps: i32,
    pub active: bool,
    pub updated_at: NaiveDateTime,
}

impl Product {
    /// The name with the pack size after it, `Huile 1,5 L`.
    pub fn display_name(&self) -> String {
        display_name(&self.name, self.contenance)
    }
}

impl TryFrom<ProductRow> for Product {
    type Error = CoreError;

    fn try_from(r: ProductRow) -> Result<Self, CoreError> {
        // A rate stored outside 0..=10 000 bps (an old import, a repaired
        // file) becomes an error here, never a panic.
        let rate = u32::try_from(r.rate_bps)
            .ok()
            .map(Bps::new)
            .transpose()?
            .ok_or(crate::money::MoneyError::RateOutOfRange)?;
        Ok(Product {
            id: r.id,
            shop_id: r.shop_id,
            name: r.name,
            barcode: r.barcode,
            category_id: r.category_id,
            unit: r.unit,
            cost: Money::centimes(r.cost_centimes),
            selling: Money::centimes(r.selling_centimes),
            wholesale: r.wholesale_centimes.map(Money::centimes),
            qty_on_hand_milli: r.qty_on_hand_milli,
            low_stock_at_milli: r.low_stock_at_milli,
            rate_bps: rate,
            active: r.active,
            created_at: r.created_at,
            updated_at: r.updated_at,
            contenance: Contenance::from_columns(r.contenance_milli, r.contenance_unit.as_deref())?,
        })
    }
}
