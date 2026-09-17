//! Products, their unit, and the label sheet printed from them.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

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

/// `cost_centimes` and `wholesale_centimes` are `Option`, not because either
/// is ever absent in the row, but because `GET /products` and
/// `GET /products/{id}` are open reads a cashier needs for the till (M4 T5
/// review, 2026-09-11: the route cannot be gated the way `GET /purchases`
/// and `GET /dashboard` are, because ringing a sale up means reading this
/// list). `routes/products.rs::redact_cost` is the one place that turns
/// either field back to `None` for a caller who does not hold
/// `Permission::SeeCostAndMargin`; `From<Product>` below always fills both,
/// so a missing value on the wire is a decision the handler took, never a
/// blank the core left.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "ProductDto.ts")]
pub struct ProductDto {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub barcode: Option<String>,
    pub category_id: Option<i32>,
    pub unit: UnitDto,
    pub cost_centimes: Option<i64>,
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
            cost_centimes: Some(p.cost.as_centimes()),
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

/// The most labels one sheet is asked for. Eighteen fit on an A4 page
/// (three across, six down), so two hundred is eleven pages and already
/// more than anybody stands at a printer for. The cap is here so a body
/// naming fifty thousand ids is refused before it becomes fifty thousand
/// queries and a page nothing can render.
pub const LABEL_SHEET_MAX: usize = 200;

/// The products a sheet of labels is asked for. Ids and not a filter: the
/// screen has a selection in front of it and the sheet is that selection, in
/// the order the caller listed it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export_to = "LabelSheetDto.ts")]
#[serde(deny_unknown_fields)]
pub struct LabelSheetDto {
    pub ids: Vec<i32>,
}
