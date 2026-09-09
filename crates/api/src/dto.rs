//! The wire types. These structs are the source for
//! `packages/shared/src/generated`; nothing hand-writes them twice.
//!
//! Money crosses as an integer number of centimes in a JSON `number`
//! (architecture.md, contract between Rust and TypeScript). ts-rs would
//! call an `i64` a `bigint`, so the exporter in `tests/export_bindings.rs`
//! configures large ints as `number`: centimes are safe below 2^53, which
//! is 90 trillion dinars. The bound is enforced at this edge, not only
//! written down: an amount beyond it would round silently in JavaScript.

use dzpos_core::error::CoreError;
use dzpos_core::models::category::Category;
use dzpos_core::models::product::{NewProduct, Product, Unit};
use dzpos_core::money::{Bps, Money};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::ApiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "UnitDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum UnitDto {
    Piece,
    Kg,
    Litre,
    Box,
}

impl From<Unit> for UnitDto {
    fn from(u: Unit) -> Self {
        match u {
            Unit::Piece => UnitDto::Piece,
            Unit::Kg => UnitDto::Kg,
            Unit::Litre => UnitDto::Litre,
            Unit::Box => UnitDto::Box,
        }
    }
}

impl From<UnitDto> for Unit {
    fn from(u: UnitDto) -> Self {
        match u {
            UnitDto::Piece => Unit::Piece,
            UnitDto::Kg => Unit::Kg,
            UnitDto::Litre => Unit::Litre,
            UnitDto::Box => Unit::Box,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "ProductDto.ts")]
pub struct ProductDto {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub barcode: Option<String>,
    pub category_id: Option<i32>,
    pub unit: UnitDto,
    pub cost_centimes: i64,
    pub selling_centimes: i64,
    pub wholesale_centimes: Option<i64>,
    pub qty_on_hand_milli: i64,
    pub low_stock_at_milli: i64,
    pub rate_bps: u32,
    pub active: bool,
}

impl From<Product> for ProductDto {
    fn from(p: Product) -> Self {
        ProductDto {
            id: p.id,
            shop_id: p.shop_id,
            name: p.name,
            barcode: p.barcode,
            category_id: p.category_id,
            unit: p.unit.into(),
            cost_centimes: p.cost.as_centimes(),
            selling_centimes: p.selling.as_centimes(),
            wholesale_centimes: p.wholesale.map(Money::as_centimes),
            qty_on_hand_milli: p.qty_on_hand_milli,
            low_stock_at_milli: p.low_stock_at_milli,
            rate_bps: p.rate_bps.as_u32(),
            active: p.active,
        }
    }
}

/// A blank `barcode` asks the server to number the product; a null
/// `rate_bps` takes the category's rate.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewProductDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewProductDto {
    pub name: String,
    #[serde(default)]
    pub barcode: Option<String>,
    #[serde(default)]
    pub category_id: Option<i32>,
    pub unit: UnitDto,
    pub cost_centimes: i64,
    pub selling_centimes: i64,
    #[serde(default)]
    pub wholesale_centimes: Option<i64>,
    #[serde(default)]
    pub qty_on_hand_milli: i64,
    #[serde(default)]
    pub low_stock_at_milli: i64,
    #[serde(default)]
    pub rate_bps: Option<u32>,
    #[serde(default = "yes")]
    pub active: bool,
}

const fn yes() -> bool {
    true
}

/// The largest integer a JSON `number` carries without loss
/// (`Number.MAX_SAFE_INTEGER`). Anything past it would be rounded by every
/// JavaScript caller, so the API refuses it as a request error.
pub const MAX_SAFE_INTEGER: i64 = (1 << 53) - 1;

fn within_js_safe_range(field: &'static str, value: i64) -> Result<i64, ApiError> {
    if value.abs() > MAX_SAFE_INTEGER {
        return Err(ApiError::Request(CoreError::validation(
            field,
            "beyond what a JSON number carries without loss (2^53 - 1)",
        )));
    }
    Ok(value)
}

impl TryFrom<NewProductDto> for NewProduct {
    type Error = ApiError;

    fn try_from(d: NewProductDto) -> Result<Self, ApiError> {
        // A rate above one whole is caught here, at the edge, so no service
        // ever sees an impossible Bps.
        let rate_bps = d
            .rate_bps
            .map(Bps::new)
            .transpose()
            .map_err(|e| ApiError::Request(e.into()))?;
        Ok(NewProduct {
            name: d.name,
            barcode: d.barcode,
            category_id: d.category_id,
            unit: d.unit.into(),
            cost: Money::centimes(within_js_safe_range("cost_centimes", d.cost_centimes)?),
            selling: Money::centimes(within_js_safe_range(
                "selling_centimes",
                d.selling_centimes,
            )?),
            wholesale: d
                .wholesale_centimes
                .map(|w| within_js_safe_range("wholesale_centimes", w))
                .transpose()?
                .map(Money::centimes),
            qty_on_hand_milli: within_js_safe_range("qty_on_hand_milli", d.qty_on_hand_milli)?,
            low_stock_at_milli: within_js_safe_range("low_stock_at_milli", d.low_stock_at_milli)?,
            rate_bps,
            active: d.active,
        })
    }
}

/// A category and the rate a product inherits from it. The add-product form
/// reads this list so the rate stops being hardcoded at 19 %.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "CategoryDto.ts")]
pub struct CategoryDto {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub default_rate_bps: u32,
}

impl From<Category> for CategoryDto {
    fn from(c: Category) -> Self {
        CategoryDto {
            id: c.id,
            shop_id: c.shop_id,
            name: c.name,
            default_rate_bps: c.default_rate_bps.as_u32(),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "HealthDto.ts")]
pub struct HealthDto {
    pub status: String,
    pub shop_id: i32,
}

/// The shape every failure takes. Generated so the client can narrow on
/// `code` without repeating the string list.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "ApiErrorDto.ts")]
pub struct ApiErrorDto {
    pub error: ApiErrorPayloadDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "ApiErrorPayloadDto.ts")]
pub struct ApiErrorPayloadDto {
    pub code: String,
    pub message: String,
}
