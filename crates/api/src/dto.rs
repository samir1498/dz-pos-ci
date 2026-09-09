//! The wire types. These structs are the source for
//! `packages/shared/src/generated`; nothing hand-writes them twice.
//!
//! Money crosses as an integer number of centimes in a JSON `number`
//! (architecture.md, contract between Rust and TypeScript). ts-rs would
//! call an `i64` a `bigint`, so the exporter in `tests/export_bindings.rs`
//! configures large ints as `number`: centimes are safe below 2^53, which
//! is 90 trillion dinars. The bound is enforced at this edge, not only
//! written down: an amount beyond it would round silently in JavaScript.

use chrono::NaiveDate;
use dzpos_core::error::CoreError;
use dzpos_core::models::category::Category;
use dzpos_core::models::document::{
    Document, DocumentKind, DocumentLine, DocumentStatus, SellerBlock,
};
use dzpos_core::models::product::{NewProduct, Product, Unit};
use dzpos_core::models::shop::{Shop, StoreBlock};
use dzpos_core::money::{Bps, Money, PaymentMode, Regime, TvaLine};
use dzpos_core::services::sales::{NewSale, NewSaleLine};
use dzpos_core::services::settings::DatedRegime;
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
    if !(-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&value) {
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

/// The seller block a ticket prints (features.md §3). Sent whole on every
/// write: an identifier left out or sent null is cleared, a name is
/// required, and a field the type does not know is refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "StoreDto.ts")]
#[serde(deny_unknown_fields)]
pub struct StoreDto {
    pub name: String,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
}

impl From<Shop> for StoreDto {
    fn from(s: Shop) -> Self {
        StoreDto {
            name: s.name,
            rc: s.rc,
            nif: s.nif,
            nis: s.nis,
            ai: s.ai,
            address: s.address,
            phone: s.phone,
        }
    }
}

impl From<StoreDto> for StoreBlock {
    fn from(d: StoreDto) -> Self {
        StoreBlock {
            name: d.name,
            rc: d.rc,
            nif: d.nif,
            nis: d.nis,
            ai: d.ai,
            address: d.address,
            phone: d.phone,
        }
    }
}

/// The two régimes the migration's CHECK allows (features.md, Régime
/// fiscal row).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "RegimeDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum RegimeDto {
    Ifu,
    Reel,
}

impl From<Regime> for RegimeDto {
    fn from(r: Regime) -> Self {
        match r {
            Regime::Ifu => RegimeDto::Ifu,
            Regime::Reel => RegimeDto::Reel,
        }
    }
}

impl From<RegimeDto> for Regime {
    fn from(r: RegimeDto) -> Self {
        match r {
            RegimeDto::Ifu => Regime::Ifu,
            RegimeDto::Reel => Regime::Reel,
        }
    }
}

/// A régime and the day it took, or takes, effect. `valid_from` is a
/// calendar day, `YYYY-MM-DD`: the régime is a yearly election, and a time
/// of day on it would be a lie of precision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "DatedRegimeDto.ts")]
pub struct DatedRegimeDto {
    pub regime: RegimeDto,
    pub valid_from: String,
}

impl From<DatedRegime> for DatedRegimeDto {
    fn from(d: DatedRegime) -> Self {
        DatedRegimeDto {
            regime: d.regime.into(),
            valid_from: d.valid_from.date().format(DATE_FORMAT).to_string(),
        }
    }
}

/// What the settings screen reads: the store block, the régime in force
/// and, when the owner has dated a change ahead, the one coming.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "SettingsDto.ts")]
pub struct SettingsDto {
    pub store: StoreDto,
    pub regime: DatedRegimeDto,
    pub regime_planned: Option<DatedRegimeDto>,
}

/// A régime change: the régime and the day it applies from. Appended to
/// the dated series, never written over the row a past document read.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "RegimeChangeDto.ts")]
#[serde(deny_unknown_fields)]
pub struct RegimeChangeDto {
    pub regime: RegimeDto,
    pub valid_from: String,
}

/// How the till pays (features.md §3). cheque and transfer are parked, so
/// the wire does not offer them even though the column admits them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "PaymentModeDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum PaymentModeDto {
    Cash,
    Card,
    Credit,
}

impl From<PaymentMode> for PaymentModeDto {
    fn from(m: PaymentMode) -> Self {
        match m {
            PaymentMode::Cash => PaymentModeDto::Cash,
            PaymentMode::Card => PaymentModeDto::Card,
            PaymentMode::Credit => PaymentModeDto::Credit,
        }
    }
}

impl From<PaymentModeDto> for PaymentMode {
    fn from(m: PaymentModeDto) -> Self {
        match m {
            PaymentModeDto::Cash => PaymentMode::Cash,
            PaymentModeDto::Card => PaymentMode::Card,
            PaymentModeDto::Credit => PaymentMode::Credit,
        }
    }
}

/// The document kinds of features.md §3. M1 issues `ticket`; the union is
/// whole so a later milestone adds a screen, not a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "DocumentKindDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum DocumentKindDto {
    Ticket,
    Facture,
    Proforma,
    BonDeLivraison,
    Avoir,
    BonDeReception,
}

impl From<DocumentKind> for DocumentKindDto {
    fn from(k: DocumentKind) -> Self {
        match k {
            DocumentKind::Ticket => DocumentKindDto::Ticket,
            DocumentKind::Facture => DocumentKindDto::Facture,
            DocumentKind::Proforma => DocumentKindDto::Proforma,
            DocumentKind::BonDeLivraison => DocumentKindDto::BonDeLivraison,
            DocumentKind::Avoir => DocumentKindDto::Avoir,
            DocumentKind::BonDeReception => DocumentKindDto::BonDeReception,
        }
    }
}

/// A cancelled document keeps its number and its row (features.md,
/// Numbering row), so the state is on the wire from the first version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "DocumentStatusDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum DocumentStatusDto {
    Issued,
    Cancelled,
}

impl From<DocumentStatus> for DocumentStatusDto {
    fn from(s: DocumentStatus) -> Self {
        match s {
            DocumentStatus::Issued => DocumentStatusDto::Issued,
            DocumentStatus::Cancelled => DocumentStatusDto::Cancelled,
        }
    }
}

/// One sold line, snapshotted at issue: the product may be renamed or
/// deleted and a reprint still shows what the customer was handed.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SaleLineDto.ts")]
pub struct SaleLineDto {
    pub id: i32,
    pub position: i32,
    pub product_id: Option<i32>,
    pub name: String,
    pub barcode: Option<String>,
    pub qty_milli: i64,
    pub unit_price_centimes: i64,
    pub line_discount_centimes: i64,
    pub rate_bps: u32,
    pub line_total_centimes: i64,
}

impl From<DocumentLine> for SaleLineDto {
    fn from(l: DocumentLine) -> Self {
        SaleLineDto {
            id: l.id,
            position: l.position,
            product_id: l.product_id,
            name: l.name,
            barcode: l.barcode,
            qty_milli: l.qty_milli,
            unit_price_centimes: l.unit_price.as_centimes(),
            line_discount_centimes: l.line_discount.as_centimes(),
            rate_bps: l.rate_bps.as_u32(),
            line_total_centimes: l.line_total.as_centimes(),
        }
    }
}

/// One row of the TVA recap, stored at issue so a reprint never recomputes
/// it. Empty under the IFU (`regime_ifu_prints_no_tva`).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SaleTvaDto.ts")]
pub struct SaleTvaDto {
    pub rate_bps: u32,
    pub base_centimes: i64,
    pub amount_centimes: i64,
}

impl From<TvaLine> for SaleTvaDto {
    fn from(t: TvaLine) -> Self {
        SaleTvaDto {
            rate_bps: t.rate.as_u32(),
            base_centimes: t.base.as_centimes(),
            amount_centimes: t.amount.as_centimes(),
        }
    }
}

/// The totals table of features.md §3, column for column. The amount in
/// words is not here: it is rendered at print time in the print language.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SaleTotalsDto.ts")]
pub struct SaleTotalsDto {
    pub total_ht_centimes: i64,
    pub discount_centimes: i64,
    pub subtotal_ht_centimes: i64,
    pub tva_centimes: i64,
    pub total_ttc_centimes: i64,
    pub stamp_centimes: i64,
    pub net_to_pay_centimes: i64,
}

/// A sale as the till reads it back: the document, its lines and its TVA
/// recap in one answer, so the receipt view makes one call.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SaleDto.ts")]
pub struct SaleDto {
    pub id: i32,
    pub shop_id: i32,
    pub kind: DocumentKindDto,
    pub series: String,
    pub number: i64,
    /// `YYYY-MM-DD HH:MM:SS` on the shop's calendar (core, services::clock).
    pub issued_at: String,
    pub user_id: i32,
    pub regime: RegimeDto,
    pub payment_mode: PaymentModeDto,
    pub seller: StoreDto,
    pub customer_id: Option<i32>,
    pub totals: SaleTotalsDto,
    pub tva: Vec<SaleTvaDto>,
    pub tendered_centimes: Option<i64>,
    pub change_centimes: Option<i64>,
    pub status: DocumentStatusDto,
    pub lines: Vec<SaleLineDto>,
}

impl From<Document> for SaleDto {
    fn from(d: Document) -> Self {
        SaleDto {
            id: d.id,
            shop_id: d.shop_id,
            kind: d.kind.into(),
            series: d.series,
            number: d.number,
            issued_at: d.issued_at.format(DATE_TIME_FORMAT).to_string(),
            user_id: d.user_id,
            regime: d.regime.into(),
            payment_mode: d.payment_mode.into(),
            seller: StoreDto {
                name: d.seller.name,
                rc: d.seller.rc,
                nif: d.seller.nif,
                nis: d.seller.nis,
                ai: d.seller.ai,
                address: d.seller.address,
                phone: d.seller.phone,
            },
            customer_id: d.customer_id,
            totals: SaleTotalsDto {
                total_ht_centimes: d.totals.total_ht.as_centimes(),
                discount_centimes: d.totals.discount.as_centimes(),
                subtotal_ht_centimes: d.totals.subtotal_ht.as_centimes(),
                tva_centimes: d.totals.tva.as_centimes(),
                total_ttc_centimes: d.totals.total_ttc.as_centimes(),
                stamp_centimes: d.totals.stamp.as_centimes(),
                net_to_pay_centimes: d.totals.net_to_pay.as_centimes(),
            },
            tva: d.totals.tva_by_rate.into_iter().map(Into::into).collect(),
            tendered_centimes: d.tendered.map(Money::as_centimes),
            change_centimes: d.change.map(Money::as_centimes),
            status: d.status.into(),
            lines: d.lines.into_iter().map(Into::into).collect(),
        }
    }
}

/// One basket line. `unit_price_centimes` left out takes the product's
/// selling price, so a till that shows the price and one that overrides it
/// send the same shape.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewSaleLineDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewSaleLineDto {
    pub product_id: i32,
    pub qty_milli: i64,
    #[serde(default)]
    pub unit_price_centimes: Option<i64>,
    #[serde(default)]
    pub line_discount_centimes: i64,
}

/// The basket the till posts. `issued_at` is not on the wire: the moment a
/// sale happened is the server's to say, on the shop's calendar, and a till
/// with a wrong clock would otherwise date a fiscal document.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewSaleDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewSaleDto {
    pub lines: Vec<NewSaleLineDto>,
    #[serde(default)]
    pub global_discount_centimes: i64,
    pub payment_mode: PaymentModeDto,
    #[serde(default)]
    pub tendered_centimes: Option<i64>,
}

impl TryFrom<NewSaleDto> for NewSale {
    type Error = ApiError;

    fn try_from(d: NewSaleDto) -> Result<Self, ApiError> {
        let mut lines = Vec::with_capacity(d.lines.len());
        for line in d.lines {
            lines.push(NewSaleLine {
                product_id: line.product_id,
                qty_milli: within_js_safe_range("qty_milli", line.qty_milli)?,
                unit_price: line
                    .unit_price_centimes
                    .map(|c| within_js_safe_range("unit_price_centimes", c))
                    .transpose()?
                    .map(Money::centimes),
                line_discount: Money::centimes(within_js_safe_range(
                    "line_discount_centimes",
                    line.line_discount_centimes,
                )?),
            });
        }
        Ok(NewSale {
            lines,
            global_discount: Money::centimes(within_js_safe_range(
                "global_discount_centimes",
                d.global_discount_centimes,
            )?),
            payment_mode: d.payment_mode.into(),
            tendered: d
                .tendered_centimes
                .map(|c| within_js_safe_range("tendered_centimes", c))
                .transpose()?
                .map(Money::centimes),
            // The server dates the document (core, services::clock).
            issued_at: None,
        })
    }
}

pub const DATE_FORMAT: &str = "%Y-%m-%d";
/// A stored timestamp, the shape every TEXT timestamp column holds.
pub const DATE_TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// `YYYY-MM-DD` and nothing else: "2026-1-5", a time, or a month 13 are the
/// caller's mistake and answer 422 naming the field.
pub fn parse_day(field: &'static str, text: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(text, DATE_FORMAT)
        .ok()
        .filter(|d| d.format(DATE_FORMAT).to_string() == text)
        .ok_or_else(|| {
            ApiError::Request(CoreError::validation(field, "a day is written YYYY-MM-DD"))
        })
}
