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
use dzpos_core::models::document::{Document, DocumentKind, DocumentLine, DocumentStatus};
use dzpos_core::models::product::{NewProduct, Product, Unit};
use dzpos_core::models::shop::{Shop, StoreBlock};
use dzpos_core::models::stock::Drift;
use dzpos_core::money::{Bps, Money, PaymentMode, Regime, TvaLine};
use dzpos_core::services::avoir::AvoirLine;
use dzpos_core::services::backup::Backup;
use dzpos_core::services::cash::{CashPosition, Outgoings, Takings};
use dzpos_core::services::clock::Month;
use dzpos_core::services::customers::{CustomerWithBalance, NewCustomer, PartyKind};
use dzpos_core::services::dashboard::{
    Dashboard, Figures, LowStock, Owed, Series, SeriesPoint, TopProduct,
};
use dzpos_core::services::debt::{DebtAllocation, DebtKind, LedgerLine, Payment, PaymentMethod};
use dzpos_core::services::documents::CancelEffect;
use dzpos_core::services::expenses::{Expense, ExpenseCategory, NewExpense};
use dzpos_core::services::import::{Applied, DryRun, Outcome, RowReport};
use dzpos_core::services::permissions::{Permission, Role};
use dzpos_core::services::preferences::Theme;
use dzpos_core::services::purchases::{
    NewLine, NewPurchase, Paid, Purchase, PurchaseLine, PurchaseStatus, PurchaseView, ReceiveLine,
};
use dzpos_core::services::sales::{NewSale, NewSaleLine, Sale, SaleKind, Warning};
use dzpos_core::services::settings::DatedRegime;
use dzpos_core::services::stock::{LastRecount, Report};
use dzpos_core::services::supplier_debt::{SupplierAllocation, SupplierDebtKind};
use dzpos_core::services::suppliers::{NewSupplier, SupplierWithBalance};
use dzpos_core::services::users::User;
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
    /// True while nobody in the shop has a PIN or a password. The desktop
    /// shows the first-setup screen then, not a sign-in for a user who does not
    /// exist yet (features.md §5).
    pub needs_first_setup: bool,
}

/// The day the shop is on, `YYYY-MM-DD`. A screen that needs "today" asks
/// for it rather than reading the machine's calendar: the core dates every
/// document on Algeria's, UTC+1 with no daylight saving, and a browser in
/// another zone would date a statement a day either side of what the ledger
/// holds (features.md §2, "One clock").
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "ClockDto.ts")]
pub struct ClockDto {
    pub today: String,
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
    /// Only on `credit_limit`: what the customer would owe once this sale
    /// landed, and the limit that refused it. Absent from every other error,
    /// so the till reads them as optional and never as a zero somebody meant
    /// (crates/api/src/error.rs writes them).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub balance_after_centimes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub credit_limit_centimes: Option<i64>,
    /// The field of the request a refusal is about, when it is about one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub field: Option<String>,
    /// Only on a payment refused for being more than the debt: what the
    /// customer actually owes. "Too much" is useless without the amount that
    /// would not have been.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub outstanding_centimes: Option<i64>,
    /// Only on `party_ids`: which half of the facture is short (`seller` or
    /// `buyer`) and which identifiers it is short of (`rc`, `nis`, `name`,
    /// `address`). The till sends the cashier to the settings or to the
    /// fiche on the side, and names the fields from the list; neither is
    /// re-derived from the code (architecture.md rule 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub party_side: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub missing_ids: Option<Vec<String>>,
    /// Only on `locked_out`: how long the user has to wait before the till
    /// will look at their PIN again. The sign-in screen counts it down.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub retry_after_seconds: Option<i64>,
    /// Only on `forbidden`: the permission the route wanted, spelled the way
    /// `PermissionDto` spells it. The screen says which thing this role may
    /// not do without working it out from the route (M4 T2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub permission: Option<PermissionDto>,
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

/// The four themes the design package emits a `[data-theme]` block for.
/// Serialised as the same string the CSS attribute carries, so the value in
/// the shop file, the value on the wire and the value on `<html>` are one
/// spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ThemeDto.ts")]
#[serde(rename_all = "kebab-case")]
pub enum ThemeDto {
    Comptoir,
    Registre,
    Observe,
    ObserveDark,
}

impl From<Theme> for ThemeDto {
    fn from(t: Theme) -> Self {
        match t {
            Theme::Comptoir => ThemeDto::Comptoir,
            Theme::Registre => ThemeDto::Registre,
            Theme::Observe => ThemeDto::Observe,
            Theme::ObserveDark => ThemeDto::ObserveDark,
        }
    }
}

impl From<ThemeDto> for Theme {
    fn from(t: ThemeDto) -> Self {
        match t {
            ThemeDto::Comptoir => Theme::Comptoir,
            ThemeDto::Registre => Theme::Registre,
            ThemeDto::Observe => Theme::Observe,
            ThemeDto::ObserveDark => Theme::ObserveDark,
        }
    }
}

/// The theme the shop chose. `null` is not a missing answer: it is the shop
/// asking to forget its choice, which puts the app back on Comptoir, the
/// default (the machine's own light or dark preference is not consulted).
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export_to = "ThemeChoiceDto.ts")]
#[serde(deny_unknown_fields)]
pub struct ThemeChoiceDto {
    pub theme: Option<ThemeDto>,
}

/// What the settings screen reads: the store block, the régime in force
/// and, when the owner has dated a change ahead, the one coming.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "SettingsDto.ts")]
pub struct SettingsDto {
    pub store: StoreDto,
    pub regime: DatedRegimeDto,
    pub regime_planned: Option<DatedRegimeDto>,
    /// `null` when the shop has never chosen one.
    pub theme: Option<ThemeDto>,
    /// How much a cashier may take off a basket before the sale needs
    /// someone holding `discount_above_threshold`, in basis points of the
    /// basket before any discount (250 is 2,5 %). Zero on a shop that has
    /// never set one, which refuses a cashier every discount: the screen
    /// should say so rather than leave an owner wondering why the till
    /// refuses a round number off.
    pub discount_threshold_bps: u32,
}

/// The version, the git short hash and the build date the running binary
/// was built with (M5 T1, docs/architecture.md § Release), plus whether it
/// is a debug build. The About screen's only source for the three: there is
/// no second, hand-typed copy in `apps/desktop`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "BuildInfoDto.ts")]
pub struct BuildInfoDto {
    pub version: String,
    pub git_hash: String,
    pub build_date: String,
    pub debug: bool,
}

impl From<dzpos_core::build_info::BuildInfo> for BuildInfoDto {
    fn from(info: dzpos_core::build_info::BuildInfo) -> Self {
        BuildInfoDto {
            version: info.version.to_string(),
            git_hash: info.git_hash.to_string(),
            build_date: info.build_date.to_string(),
            debug: info.is_debug,
        }
    }
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

/// A change to the discount a cashier may give without asking anyone: the
/// threshold in basis points and the day it applies from. Dated and appended
/// like the régime, never written over, so a sale refused last month can
/// still be read against the threshold that refused it.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "DiscountThresholdChangeDto.ts")]
#[serde(deny_unknown_fields)]
pub struct DiscountThresholdChangeDto {
    pub threshold_bps: u32,
    pub valid_from: String,
}

/// One copy of the shop file in the backup folder. `name` is both what the
/// screen shows and the id the restore route takes back, so a caller never
/// builds a path: the server owns the folder and only the name crosses.
/// `bytes` is a file size, which is why it is not money and not centimes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "BackupDto.ts")]
pub struct BackupDto {
    pub name: String,
    /// `YYYY-MM-DDTHH:MM:SS` on the shop's own calendar, the time the copy
    /// was taken, read from the name rather than from the file's mtime.
    pub taken_at: String,
    pub bytes: i64,
}

impl From<Backup> for BackupDto {
    fn from(b: Backup) -> Self {
        BackupDto {
            name: b.name,
            taken_at: b.taken_at.format(STAMP_FORMAT).to_string(),
            // A backup past 9.2 exabytes would round in JavaScript. The
            // clamp is what keeps the wire honest rather than a silent
            // rounding.
            bytes: i64::try_from(b.bytes).unwrap_or(MAX_SAFE_INTEGER),
        }
    }
}

/// What the settings screen reads: the daily copies, the copies taken on the
/// way into a restore, and the copies taken on the way into an upgrade. The
/// three are separate lists because they are kept under different rules: the
/// daily ones are pruned to thirty, the other two are never touched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "BackupsDto.ts")]
pub struct BackupsDto {
    pub backups: Vec<BackupDto>,
    pub safety_copies: Vec<BackupDto>,
    pub upgrade_copies: Vec<BackupDto>,
}

/// What the shop file holds after a restore: the copy it came from and the
/// counts read out of it, so the screen can say what landed instead of
/// "done".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "RestoreDto.ts")]
pub struct RestoreDto {
    pub restored_from: String,
    /// The copy of the shop file as it was a moment before, taken on the way
    /// in and kept beside the shop file. Named on the wire because nothing
    /// deletes it and the owner is the only one who can decide to.
    pub safety_copy: String,
    pub products: i64,
    /// Null only for a copy taken before the documents table existed
    /// (migration 2, the sale); every copy since carries the count.
    pub documents: Option<i64>,
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

/// The document kinds of features.md §3. The till issues `ticket` and
/// `facture`; the union is whole so a later milestone adds a screen, not a
/// type.
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
    Quittance,
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
            DocumentKind::Quittance => DocumentKindDto::Quittance,
        }
    }
}

/// The paper the till rings a basket up on (features.md §3). Three values
/// and not `DocumentKindDto`: an avoir and a bon de livraison are their own
/// writes with their own rules, and a till that could name one on `POST
/// /sales` would be issuing a document nobody asked for.
///
/// A proforma is here because the till is where the basket is. It is the one
/// value that ends in no sale: the core hands it to `services::proforma`,
/// which writes the quotation and moves neither stock nor debt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export_to = "SaleKindDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum SaleKindDto {
    #[default]
    Ticket,
    Facture,
    Proforma,
}

impl From<SaleKindDto> for SaleKind {
    fn from(k: SaleKindDto) -> Self {
        match k {
            SaleKindDto::Ticket => SaleKind::Ticket,
            SaleKindDto::Facture => SaleKind::Facture,
            SaleKindDto::Proforma => SaleKind::Proforma,
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
    /// The facture line this one credits, on an avoir line and nowhere else.
    /// The screen showing an avoir beside its facture lines the two up by it.
    pub ref_line_id: Option<i32>,
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
            ref_line_id: l.ref_line_id,
        }
    }
}

/// One row of the TVA recap, stored at issue so a reprint never recomputes
/// it. Empty under the IFU (`an_ifu_facture_names_no_tax_in_any_language`).
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

/// What the customer owed before this document, what it leaves unpaid, and
/// what they owe now (features.md §3, the balance triple). Stored on the
/// document at issue and never recomputed, so a screen and a reprint say the
/// same thing. `remaining_debt_centimes` is the one of the three that moves
/// afterwards: a payment settles part of a document and the column says how
/// much of it is left. Null on a document that names no customer.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SaleBalanceDto.ts")]
pub struct SaleBalanceDto {
    pub old_balance_centimes: i64,
    pub remaining_debt_centimes: i64,
    pub total_debt_centimes: i64,
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
    /// The number as it is printed and as a customer quotes it back,
    /// `FA-2026-000001`. Built by the core beside the templates that print it
    /// (`print::number`), so a screen naming a document and the paper in the
    /// customer's hand cannot spell it two ways.
    pub printed_number: String,
    /// `YYYY-MM-DD HH:MM:SS` on the shop's calendar (core, services::clock).
    pub issued_at: String,
    pub user_id: i32,
    pub regime: RegimeDto,
    pub payment_mode: PaymentModeDto,
    pub seller: StoreDto,
    pub customer_id: Option<i32>,
    /// The facture an avoir is written against, null on every other kind.
    /// The screen showing an avoir follows it to name the paper it credits.
    pub ref_document_id: Option<i32>,
    /// The buyer's name as this document printed it, snapshotted at issue.
    /// Null on a ticket sold to whoever walked in. A list naming the customer
    /// reads it from here and never from the fiche: the fiche is edited in
    /// place, and the paper says who it was made out to on the day.
    pub buyer_name: Option<String>,
    /// Null on a document with no customer, which is every cash ticket.
    pub balance: Option<SaleBalanceDto>,
    pub totals: SaleTotalsDto,
    pub tva: Vec<SaleTvaDto>,
    pub tendered_centimes: Option<i64>,
    pub change_centimes: Option<i64>,
    pub status: DocumentStatusDto,
    /// Filled exactly when `status` is `cancelled`: when it was annulled, by
    /// whom, why, and the avoir that carried the money back when one did.
    pub cancellation: Option<SaleCancellationDto>,
    pub lines: Vec<SaleLineDto>,
    /// What cancelling this document would do, so a screen can say it before
    /// it asks. Null on a list and on the answer to a sale: it is a question
    /// about one stored document and it costs a read of that document's credit
    /// notes, so only a read of one document carries it.
    pub cancel_effect: Option<SaleCancelEffectDto>,
    /// What the till should say while still handing over the ticket, null
    /// when there is nothing to say. A read of a stored document carries
    /// none: a warning is about the moment the sale was rung up, not about
    /// the paper.
    pub warning: Option<SaleWarningDto>,
}

/// What cancelling a document would do. A union rather than a word and a
/// nullable amount, so the amount cannot go missing on the one shape that has
/// one, and so the day a fourth effect exists the screens matching on these
/// three stop compiling.
///
/// The screen must not work this out from the document's own fields. A
/// facture whose goods have all come back on earlier credit notes carries
/// debt, was sold on credit and names a customer, and cancelling it does
/// nothing at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "SaleCancelEffectDto.ts")]
#[serde(tag = "effect", rename_all = "snake_case")]
pub enum SaleCancelEffectDto {
    /// Annulled and nothing moves: every line has already come back.
    NothingToReverse,
    /// The goods go back on the shelf. Nobody was owed anything.
    StockBack,
    /// The goods go back and this much comes off the customer's account. On a
    /// facture that is a numbered avoir; on a ticket it is a ledger row alone,
    /// because an avoir is written against a facture.
    StockBackAndAvoir { amount_centimes: i64 },
}

impl From<CancelEffect> for SaleCancelEffectDto {
    fn from(e: CancelEffect) -> Self {
        match e {
            CancelEffect::NothingToReverse => SaleCancelEffectDto::NothingToReverse,
            CancelEffect::StockBack => SaleCancelEffectDto::StockBack,
            CancelEffect::StockBackAndAvoir { amount } => SaleCancelEffectDto::StockBackAndAvoir {
                amount_centimes: amount.as_centimes(),
            },
        }
    }
}

/// What the till should say about a sale that went through anyway. A union
/// rather than a string, so the day a second warning exists the screens that
/// match on this one stop compiling instead of quietly ignoring it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "SaleWarningDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum SaleWarningDto {
    /// The balance this sale leaves reached the customer's warn threshold.
    NearLimit,
}

impl From<Warning> for SaleWarningDto {
    fn from(w: Warning) -> Self {
        match w {
            Warning::NearLimit => SaleWarningDto::NearLimit,
        }
    }
}

impl From<Document> for SaleDto {
    fn from(d: Document) -> Self {
        SaleDto {
            id: d.id,
            shop_id: d.shop_id,
            printed_number: dzpos_core::print::number(&d),
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
            ref_document_id: d.ref_document_id,
            buyer_name: d.buyer.as_ref().map(|b| b.name.clone()),
            balance: d.balance.map(|b| SaleBalanceDto {
                old_balance_centimes: b.old_balance.as_centimes(),
                remaining_debt_centimes: b.remaining_debt.as_centimes(),
                total_debt_centimes: b.total_debt.as_centimes(),
            }),
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
            cancellation: d.cancellation.map(|c| SaleCancellationDto {
                cancelled_at: c.at.format(DATE_TIME_FORMAT).to_string(),
                cancelled_by: c.by,
                reason: c.reason,
                avoir_document_id: c.avoir_document_id,
            }),
            lines: d.lines.into_iter().map(Into::into).collect(),
            cancel_effect: None,
            warning: None,
        }
    }
}

impl From<Sale> for SaleDto {
    fn from(s: Sale) -> Self {
        SaleDto {
            warning: s.warning.map(Into::into),
            ..SaleDto::from(s.document)
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
    /// Who the sale is made out to. Required on credit; on cash and card it
    /// names the buyer on the document and moves no debt.
    #[serde(default)]
    pub customer_id: Option<i32>,
    /// Sell past the customer's credit limit on purpose. `override` on the
    /// wire because that is what the button says; `override` is a Rust
    /// keyword, so the field is spelled out here and renamed on both sides.
    #[serde(default, rename = "override")]
    #[ts(rename = "override")]
    pub override_credit: bool,
    /// Ticket or facture, decided at the till before the sale is saved
    /// (features.md §3). Left out means a ticket: a sale to a consumer is
    /// the ordinary case and asks nothing of the buyer, so a caller written
    /// before this field existed keeps issuing what it always did.
    #[serde(default)]
    pub kind: SaleKindDto,
    /// Retry key (M7 T4). A caller that got no answer posts the same basket
    /// with the same key and gets the original sale back instead of ringing
    /// twice. Left out means no promise: today's desktop keeps ringing like
    /// it always did.
    #[serde(default)]
    pub idempotency_key: Option<String>,
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
            customer_id: d.customer_id,
            override_credit: d.override_credit,
            kind: d.kind.into(),
            // The server dates the document (core, services::clock).
            issued_at: None,
        })
    }
}

/// Who the buyer is (features.md §2). Asked for on the fiche, never inferred
/// from whether an RC was typed in: loi 04-02 art. 10 decides ticket against
/// facture by the buyer, so an inference would flip the rule the moment
/// somebody cleared a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "PartyKindDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum PartyKindDto {
    Company,
    Consumer,
}

impl From<PartyKind> for PartyKindDto {
    fn from(k: PartyKind) -> Self {
        match k {
            PartyKind::Company => PartyKindDto::Company,
            PartyKind::Consumer => PartyKindDto::Consumer,
        }
    }
}

impl From<PartyKindDto> for PartyKind {
    fn from(k: PartyKindDto) -> Self {
        match k {
            PartyKindDto::Company => PartyKind::Company,
            PartyKindDto::Consumer => PartyKind::Consumer,
        }
    }
}

/// Why the debt moved (features.md §2). The whole union crosses from the
/// first version: the ledger already holds the `sale` and `payment` rows the
/// till writes, and a screen that met an unknown kind could only refuse the
/// whole answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "DebtKindDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum DebtKindDto {
    Opening,
    Sale,
    Payment,
    Avoir,
    Adjustment,
}

impl From<DebtKind> for DebtKindDto {
    fn from(k: DebtKind) -> Self {
        match k {
            DebtKind::Opening => DebtKindDto::Opening,
            DebtKind::Sale => DebtKindDto::Sale,
            DebtKind::Payment => DebtKindDto::Payment,
            DebtKind::Avoir => DebtKindDto::Avoir,
            DebtKind::Adjustment => DebtKindDto::Adjustment,
        }
    }
}

/// What a cancellation left on the document it annulled (features.md §3).
/// Whole or absent: a screen never has to ask whether the date is there
/// while the reason is not.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SaleCancellationDto.ts")]
pub struct SaleCancellationDto {
    /// `YYYY-MM-DD HH:MM:SS` on the shop's calendar.
    pub cancelled_at: String,
    pub cancelled_by: i32,
    pub reason: String,
    /// The avoir the cancellation issued, null when there was nothing to
    /// carry back: a cash ticket owed nobody anything.
    pub avoir_document_id: Option<i32>,
}

/// One line of a facture and how much of it is coming back on an avoir. The
/// line is named by id and never by the product on it: a facture carries one
/// product on two lines as soon as a line discount is involved.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export_to = "AvoirLineDto.ts")]
#[serde(deny_unknown_fields)]
pub struct AvoirLineDto {
    pub document_line_id: i32,
    pub qty_milli: i64,
}

/// What is coming back on a credit note. `lines` of null is the whole of what
/// is left on the facture, which is what the "avoir the lot" button sends and
/// what a cancellation uses.
///
/// Which is why a field this type does not know is refused rather than
/// dropped: `line` for `lines` would otherwise read as the whole facture
/// coming back, and a shop asking for one unit of three would have credited
/// all three without being told.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewAvoirDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewAvoirDto {
    pub lines: Option<Vec<AvoirLineDto>>,
    pub reason: Option<String>,
}

impl NewAvoirDto {
    pub fn lines(&self) -> Option<Vec<AvoirLine>> {
        self.lines.as_ref().map(|lines| {
            lines
                .iter()
                .map(|l| AvoirLine {
                    document_line_id: l.document_line_id,
                    qty_milli: l.qty_milli,
                })
                .collect()
        })
    }
}

/// Why a document is being annulled. Required: a document annulled for no
/// stated reason is what features.md §5 keeps a log against.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "CancelDocumentDto.ts")]
#[serde(deny_unknown_fields)]
pub struct CancelDocumentDto {
    pub reason: String,
}

/// The fiche as a screen reads it, with what the customer owes. The balance
/// is the ledger's sum computed in the core, never a stored column, and it
/// travels with the fiche so the list does not make a call per row.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "CustomerDto.ts")]
pub struct CustomerDto {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub party_kind: PartyKindDto,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    /// Null is no limit at all, zero is no credit at all: two different
    /// answers, and the till acts on them differently.
    pub credit_limit_centimes: Option<i64>,
    pub warn_threshold_centimes: Option<i64>,
    pub notes: Option<String>,
    pub active: bool,
    /// Below zero is the shop owing the customer after an overpayment.
    pub balance_centimes: i64,
}

impl From<CustomerWithBalance> for CustomerDto {
    fn from(c: CustomerWithBalance) -> Self {
        let balance = c.balance.as_centimes();
        let c = c.customer;
        CustomerDto {
            id: c.id,
            shop_id: c.shop_id,
            name: c.name,
            party_kind: c.party_kind.into(),
            phone: c.phone,
            address: c.address,
            rc: c.rc,
            nif: c.nif,
            nis: c.nis,
            ai: c.ai,
            credit_limit_centimes: c.credit_limit.map(Money::as_centimes),
            warn_threshold_centimes: c.warn_threshold.map(Money::as_centimes),
            notes: c.notes,
            active: c.active,
            balance_centimes: balance,
        }
    }
}

/// The fields a fiche is written with, on a create and on an update alike.
/// The whole row travels every time, the way the store block does: a field
/// left out is a bug at the edge, and a null clears the column.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "CustomerWriteDto.ts")]
#[serde(deny_unknown_fields)]
pub struct CustomerWriteDto {
    pub name: String,
    pub party_kind: PartyKindDto,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub credit_limit_centimes: Option<i64>,
    pub warn_threshold_centimes: Option<i64>,
    pub notes: Option<String>,
    pub active: bool,
    /// Why a fiche is being closed. Asked for only when the update closes one
    /// that still carries a balance either way or a document still asking to
    /// be paid, and ignored on every other update.
    #[serde(default)]
    pub close_reason: Option<String>,
}

/// A new fiche: the same fields, plus the debt the shop was already carrying
/// for this customer before it had the app. The opening debt is only on the
/// create because it is a ledger movement, not a column, and an update that
/// could set it would be an edit to the ledger nobody could see (features.md
/// §2).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewCustomerDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewCustomerDto {
    pub name: String,
    pub party_kind: PartyKindDto,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub credit_limit_centimes: Option<i64>,
    pub warn_threshold_centimes: Option<i64>,
    pub notes: Option<String>,
    #[serde(default = "yes")]
    pub active: bool,
    #[serde(default)]
    pub opening_debt_centimes: Option<i64>,
}

impl TryFrom<CustomerWriteDto> for NewCustomer {
    type Error = ApiError;

    fn try_from(d: CustomerWriteDto) -> Result<Self, ApiError> {
        Ok(NewCustomer {
            name: d.name,
            party_kind: d.party_kind.into(),
            phone: d.phone,
            address: d.address,
            rc: d.rc,
            nif: d.nif,
            nis: d.nis,
            ai: d.ai,
            credit_limit: money_field("credit_limit_centimes", d.credit_limit_centimes)?,
            warn_threshold: money_field("warn_threshold_centimes", d.warn_threshold_centimes)?,
            notes: d.notes,
            active: d.active,
        })
    }
}

impl TryFrom<NewCustomerDto> for NewCustomer {
    type Error = ApiError;

    fn try_from(d: NewCustomerDto) -> Result<Self, ApiError> {
        NewCustomer::try_from(CustomerWriteDto {
            name: d.name,
            party_kind: d.party_kind,
            phone: d.phone,
            address: d.address,
            rc: d.rc,
            nif: d.nif,
            nis: d.nis,
            ai: d.ai,
            credit_limit_centimes: d.credit_limit_centimes,
            warn_threshold_centimes: d.warn_threshold_centimes,
            notes: d.notes,
            active: d.active,
            // A fiche being created closes nothing.
            close_reason: None,
        })
    }
}

/// An optional amount on the wire, checked against the safe-integer bound
/// like every other one: `None` stays `None`, which is the field left empty.
pub fn money_field(field: &'static str, value: Option<i64>) -> Result<Option<Money>, ApiError> {
    value
        .map(|c| within_js_safe_range(field, c))
        .transpose()
        .map(|c| c.map(Money::centimes))
}

/// One movement of the ledger, with the balance it left behind.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "DebtEntryDto.ts")]
pub struct DebtEntryDto {
    pub id: i32,
    pub customer_id: i32,
    /// The document the movement came from, when it came from one. An
    /// opening balance and an adjustment cite none.
    pub document_id: Option<i32>,
    pub kind: DebtKindDto,
    /// What the movement added to the debt; zero on a payment or an avoir.
    pub debit_centimes: i64,
    /// What it took off; zero on a sale or an opening balance.
    pub credit_centimes: i64,
    /// The balance as of this movement: every older one counted, no newer
    /// one. Computed in the core (services::debt).
    pub balance_after_centimes: i64,
    pub user_id: i32,
    pub note: Option<String>,
    /// `YYYY-MM-DD HH:MM:SS`, the shape every stored timestamp holds.
    pub created_at: String,
}

impl From<LedgerLine> for DebtEntryDto {
    fn from(l: LedgerLine) -> Self {
        DebtEntryDto {
            id: l.entry.id,
            customer_id: l.entry.customer_id,
            document_id: l.entry.document_id,
            kind: l.entry.kind.into(),
            debit_centimes: l.entry.debit.as_centimes(),
            credit_centimes: l.entry.credit.as_centimes(),
            balance_after_centimes: l.balance_after.as_centimes(),
            user_id: l.entry.user_id,
            note: l.entry.note,
            created_at: l.entry.created_at.format(DATE_TIME_FORMAT).to_string(),
        }
    }
}

/// A customer's ledger: the movements newest first and the balance they sum
/// to. The balance is in the envelope so a screen showing it never adds the
/// column up itself.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "CustomerLedgerDto.ts")]
pub struct CustomerLedgerDto {
    pub customer_id: i32,
    pub balance_centimes: i64,
    pub entries: Vec<DebtEntryDto>,
}

/// A correction to what a customer owes: signed centimes and why. Positive
/// raises the debt, negative lowers it, zero is refused. The ledger is
/// append-only, so this writes a movement rather than editing one.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "AdjustmentDto.ts")]
#[serde(deny_unknown_fields)]
pub struct AdjustmentDto {
    pub amount_centimes: i64,
    #[serde(default)]
    pub note: Option<String>,
}

impl AdjustmentDto {
    /// The amount as money the core will take. The safe-integer bound is
    /// checked here, at the edge, like every other amount on the wire.
    pub fn amount(&self) -> Result<Money, ApiError> {
        Ok(Money::centimes(within_js_safe_range(
            "amount_centimes",
            self.amount_centimes,
        )?))
    }
}

pub const DATE_FORMAT: &str = "%Y-%m-%d";
/// A stored timestamp, the shape every TEXT timestamp column holds.
pub const DATE_TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// A time on the wire: a day, `T`, and a clock. Seconds, never fractions;
/// the backup name is only that precise.
pub const STAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

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

/// How a payment against a debt was taken (features.md §2). Two ways and not
/// three: settling a credit with more credit is not a payment, so this is not
/// `PaymentModeDto`, which is what a document was sold under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "PaymentMethodDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethodDto {
    Cash,
    Card,
}

impl From<PaymentMethod> for PaymentMethodDto {
    fn from(m: PaymentMethod) -> Self {
        match m {
            PaymentMethod::Cash => PaymentMethodDto::Cash,
            PaymentMethod::Card => PaymentMethodDto::Card,
        }
    }
}

impl From<PaymentMethodDto> for PaymentMethod {
    fn from(m: PaymentMethodDto) -> Self {
        match m {
            PaymentMethodDto::Cash => PaymentMethod::Cash,
            PaymentMethodDto::Card => PaymentMethod::Card,
        }
    }
}

/// What one payment placed on one document (features.md §2). A payment is one
/// movement and the documents it settled are these, oldest first.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PaymentAllocationDto.ts")]
pub struct PaymentAllocationDto {
    pub document_id: i32,
    pub amount_centimes: i64,
}

impl From<DebtAllocation> for PaymentAllocationDto {
    fn from(a: DebtAllocation) -> Self {
        PaymentAllocationDto {
            document_id: a.document_id,
            amount_centimes: a.amount.as_centimes(),
        }
    }
}

/// One payment, with what it settled and the balance it left behind. The
/// allocations travel with it so a screen showing a payment never asks a
/// second time what the money went to.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PaymentDto.ts")]
pub struct PaymentDto {
    /// The ledger movement's id: a payment is a row of the ledger, and an
    /// allocation names it.
    pub ledger_id: i32,
    pub customer_id: i32,
    pub amount_centimes: i64,
    /// Null on a payment written before the mode was stored; nothing writes
    /// one without it now.
    pub payment_mode: Option<PaymentMethodDto>,
    pub note: Option<String>,
    /// The balance as of this payment: every older movement counted, no newer
    /// one. Computed in the core (services::debt).
    pub balance_after_centimes: i64,
    pub allocations: Vec<PaymentAllocationDto>,
    /// `YYYY-MM-DD HH:MM:SS`, the shape every stored timestamp holds.
    pub created_at: String,
}

impl From<Payment> for PaymentDto {
    fn from(p: Payment) -> Self {
        PaymentDto {
            ledger_id: p.entry.id,
            customer_id: p.entry.customer_id,
            amount_centimes: p.entry.credit.as_centimes(),
            payment_mode: p.entry.payment_mode.map(Into::into),
            note: p.entry.note,
            balance_after_centimes: p.balance_after.as_centimes(),
            allocations: p.allocations.into_iter().map(Into::into).collect(),
            created_at: p.entry.created_at.format(DATE_TIME_FORMAT).to_string(),
        }
    }
}

/// A customer's payments, newest first, and the balance the whole ledger sums
/// to. The balance is in the envelope for the reason the ledger's is: a screen
/// showing it never adds a column up itself.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "CustomerPaymentsDto.ts")]
pub struct CustomerPaymentsDto {
    pub customer_id: i32,
    pub balance_centimes: i64,
    pub payments: Vec<PaymentDto>,
}

/// Money against a debt: how much, how it was taken, and why if the shop
/// wants to say. The moment is the server's, like a document's `issued_at`.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewPaymentDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewPaymentDto {
    pub amount_centimes: i64,
    pub payment_mode: PaymentMethodDto,
    #[serde(default)]
    pub note: Option<String>,
}

impl NewPaymentDto {
    /// The amount as money the core will take. The safe-integer bound is
    /// checked here, at the edge, like every other amount on the wire.
    pub fn amount(&self) -> Result<Money, ApiError> {
        Ok(Money::centimes(within_js_safe_range(
            "amount_centimes",
            self.amount_centimes,
        )?))
    }
}

/// Why the supplier debt moved (features.md §1). The whole union crosses from
/// the first version, the way the customer side's does: the receipt path
/// writes the `purchase` and `return` rows, and a screen that met an unknown kind could
/// only refuse the whole answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "SupplierDebtKindDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum SupplierDebtKindDto {
    Opening,
    Purchase,
    Payment,
    Return,
    Adjustment,
}

impl From<SupplierDebtKind> for SupplierDebtKindDto {
    fn from(k: SupplierDebtKind) -> Self {
        match k {
            SupplierDebtKind::Opening => SupplierDebtKindDto::Opening,
            SupplierDebtKind::Purchase => SupplierDebtKindDto::Purchase,
            SupplierDebtKind::Payment => SupplierDebtKindDto::Payment,
            SupplierDebtKind::Return => SupplierDebtKindDto::Return,
            SupplierDebtKind::Adjustment => SupplierDebtKindDto::Adjustment,
        }
    }
}

/// The fiche as a screen reads it, with what the shop owes the supplier. The
/// balance is the ledger's sum computed in the core, never a stored column,
/// and it travels with the fiche so the list does not make a call per row.
///
/// There is no credit limit and no `party_kind`: those are what a shop grants
/// a buyer, and nothing it hands a supplier is a document it issues.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SupplierDto.ts")]
pub struct SupplierDto {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub notes: Option<String>,
    pub active: bool,
    /// Below zero is the supplier owing the shop after an advance or a return
    /// past what was due.
    pub balance_centimes: i64,
}

impl From<SupplierWithBalance> for SupplierDto {
    fn from(s: SupplierWithBalance) -> Self {
        let balance = s.balance.as_centimes();
        let s = s.supplier;
        SupplierDto {
            id: s.id,
            shop_id: s.shop_id,
            name: s.name,
            phone: s.phone,
            address: s.address,
            rc: s.rc,
            nif: s.nif,
            nis: s.nis,
            ai: s.ai,
            notes: s.notes,
            active: s.active,
            balance_centimes: balance,
        }
    }
}

/// The fields a supplier fiche is written with, on a create and on an update
/// alike. The whole row travels every time: a field left out is a bug at the
/// edge, and a null clears the column.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "SupplierWriteDto.ts")]
#[serde(deny_unknown_fields)]
pub struct SupplierWriteDto {
    pub name: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub notes: Option<String>,
    pub active: bool,
    /// Why a fiche is being closed. Asked for only when the update closes one
    /// whose account is still open, and ignored on every other update. The
    /// close route sends the same reason under its own field.
    #[serde(default)]
    pub close_reason: Option<String>,
}

/// A new fiche: the same fields, plus the debt the shop was already carrying
/// to this supplier before it had the app. The opening debt is only on the
/// create because it is a ledger movement, not a column.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewSupplierDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewSupplierDto {
    pub name: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub notes: Option<String>,
    #[serde(default = "yes")]
    pub active: bool,
    #[serde(default)]
    pub opening_debt_centimes: Option<i64>,
}

/// Why the shop has stopped buying from this supplier. Its own body rather
/// than a field of the fiche: closing is one decision and the route says so.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "CloseSupplierDto.ts")]
#[serde(deny_unknown_fields)]
pub struct CloseSupplierDto {
    #[serde(default)]
    pub reason: Option<String>,
}

impl From<SupplierWriteDto> for NewSupplier {
    fn from(d: SupplierWriteDto) -> Self {
        NewSupplier {
            name: d.name,
            phone: d.phone,
            address: d.address,
            rc: d.rc,
            nif: d.nif,
            nis: d.nis,
            ai: d.ai,
            notes: d.notes,
            active: d.active,
        }
    }
}

impl From<NewSupplierDto> for NewSupplier {
    fn from(d: NewSupplierDto) -> Self {
        NewSupplier::from(SupplierWriteDto {
            name: d.name,
            phone: d.phone,
            address: d.address,
            rc: d.rc,
            nif: d.nif,
            nis: d.nis,
            ai: d.ai,
            notes: d.notes,
            active: d.active,
            // A fiche being created closes nothing.
            close_reason: None,
        })
    }
}

/// What one payment placed on one order. A payment is one movement and the
/// orders it settled are these, oldest first.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SupplierAllocationDto.ts")]
pub struct SupplierAllocationDto {
    pub purchase_id: i32,
    pub amount_centimes: i64,
}

impl From<SupplierAllocation> for SupplierAllocationDto {
    fn from(a: SupplierAllocation) -> Self {
        SupplierAllocationDto {
            purchase_id: a.purchase_id,
            amount_centimes: a.amount.as_centimes(),
        }
    }
}

/// One movement of the supplier ledger, with the balance it left behind and,
/// on a payment, the orders it settled. The allocations travel with the row
/// so a fiche showing a payment never asks a second time what the money went
/// to; every other kind carries an empty list.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SupplierEntryDto.ts")]
pub struct SupplierEntryDto {
    pub id: i32,
    pub supplier_id: i32,
    /// The order the movement came from, when it came from one. An opening
    /// balance, a payment and a correction cite none.
    pub purchase_id: Option<i32>,
    pub kind: SupplierDebtKindDto,
    /// What the movement added to what the shop owes; zero on a payment or a
    /// return.
    pub debit_centimes: i64,
    /// What it took off; zero on a purchase or an opening balance.
    pub credit_centimes: i64,
    /// The balance as of this movement: every older one counted, no newer
    /// one. Computed in the core (services::supplier_debt).
    pub balance_after_centimes: i64,
    /// Null on every movement that is not a payment.
    pub payment_mode: Option<PaymentMethodDto>,
    pub user_id: i32,
    pub note: Option<String>,
    pub allocations: Vec<SupplierAllocationDto>,
    /// `YYYY-MM-DD HH:MM:SS`, the shape every stored timestamp holds.
    pub created_at: String,
}

/// A supplier's ledger: the movements newest first and the balance they sum
/// to. The balance is in the envelope so a screen showing it never adds the
/// column up itself.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SupplierLedgerDto.ts")]
pub struct SupplierLedgerDto {
    pub supplier_id: i32,
    pub balance_centimes: i64,
    pub entries: Vec<SupplierEntryDto>,
}

/// A supplier's account over a range of days: what the shop owed on the
/// morning of `from`, every movement between the two days oldest first, and
/// what it owed on the evening of `to`. Both balances are read off the core's
/// running column, so a page printing them adds nothing up.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SupplierStatementDto.ts")]
pub struct SupplierStatementDto {
    pub supplier_id: i32,
    /// `YYYY-MM-DD`, both ends included.
    pub from: String,
    pub to: String,
    pub opening_centimes: i64,
    pub entries: Vec<SupplierEntryDto>,
    pub closing_centimes: i64,
}

/// What an expense is filed under (features.md §1, Expense). The row carries
/// an i18n key and not a label: the desktop reads the three languages from
/// its own files by that key, so a shop switching language does not rewrite
/// its rows. `active` travels because a retired category still names the
/// expenses filed under it while the form refuses new ones.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "ExpenseCategoryDto.ts")]
pub struct ExpenseCategoryDto {
    pub id: i32,
    pub key: String,
    pub sort_order: i32,
    pub active: bool,
}

impl From<ExpenseCategory> for ExpenseCategoryDto {
    fn from(c: ExpenseCategory) -> Self {
        ExpenseCategoryDto {
            id: c.id,
            key: c.key,
            sort_order: c.sort_order,
            active: c.active,
        }
    }
}

/// One expense. The day is `YYYY-MM-DD` on the shop's calendar, which is what
/// the column holds.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "ExpenseDto.ts")]
pub struct ExpenseDto {
    pub id: i32,
    pub category_id: i32,
    pub amount_centimes: i64,
    pub expense_date: String,
    pub note: Option<String>,
}

impl From<Expense> for ExpenseDto {
    fn from(e: Expense) -> Self {
        ExpenseDto {
            id: e.id,
            category_id: e.category_id,
            amount_centimes: e.amount.as_centimes(),
            expense_date: e.expense_date,
            note: e.note,
        }
    }
}

/// One month of expenses and what it came to. The total is the core's, summed
/// over the same days the list covers: a screen adding the rows up would be a
/// second answer to the same question.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "ExpensesDto.ts")]
pub struct ExpensesDto {
    /// `YYYY-MM`, as the month was read.
    pub month: String,
    pub total_centimes: i64,
    pub expenses: Vec<ExpenseDto>,
}

/// An expense as the form sends it. The user is not on the wire: it comes
/// from the caller's identity like every other write.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewExpenseDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewExpenseDto {
    pub category_id: i32,
    pub amount_centimes: i64,
    pub expense_date: String,
    #[serde(default)]
    pub note: Option<String>,
}

impl TryFrom<NewExpenseDto> for NewExpense {
    type Error = ApiError;

    fn try_from(d: NewExpenseDto) -> Result<Self, ApiError> {
        Ok(NewExpense {
            category_id: d.category_id,
            amount: Money::centimes(within_js_safe_range("amount_centimes", d.amount_centimes)?),
            expense_date: parse_day("expense_date", &d.expense_date)?,
            note: d.note,
        })
    }
}

/// Money that came in over the period, and what it adds up to. The total
/// travels rather than being added on the screen, for the reason the month's
/// does: one question, one answer.
///
/// `sales_centimes` is what the drawer took, the droit de timbre included.
/// `stamp_centimes` is that tax on its own, a part of the figure above and
/// never a second one to add: a screen showing the shop's own takings
/// subtracts it, and one counting the till does not.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export_to = "TakingsDto.ts")]
pub struct TakingsDto {
    pub sales_centimes: i64,
    pub stamp_centimes: i64,
    pub customer_payments_centimes: i64,
    pub total_centimes: i64,
}

impl TryFrom<Takings> for TakingsDto {
    type Error = ApiError;

    fn try_from(t: Takings) -> Result<Self, ApiError> {
        Ok(TakingsDto {
            sales_centimes: t.sales.as_centimes(),
            stamp_centimes: t.stamp.as_centimes(),
            customer_payments_centimes: t.customer_payments.as_centimes(),
            total_centimes: t.total().map_err(ApiError::from)?.as_centimes(),
        })
    }
}

/// Cash that left over the period. `refunds_centimes` is zero in this
/// version: an avoir credits the customer's ledger and brings the goods back,
/// and nothing says the drawer opened for it.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export_to = "OutgoingsDto.ts")]
pub struct OutgoingsDto {
    pub refunds_centimes: i64,
    pub supplier_payments_centimes: i64,
    pub expenses_centimes: i64,
    pub total_centimes: i64,
}

impl TryFrom<Outgoings> for OutgoingsDto {
    type Error = ApiError;

    fn try_from(o: Outgoings) -> Result<Self, ApiError> {
        Ok(OutgoingsDto {
            refunds_centimes: o.refunds.as_centimes(),
            supplier_payments_centimes: o.supplier_payments.as_centimes(),
            expenses_centimes: o.expenses.as_centimes(),
            total_centimes: o.total().map_err(ApiError::from)?.as_centimes(),
        })
    }
}

/// The cash position over a day or a month (features.md §1, Dashboard). Never
/// a stored figure: the core sums the ledgers on every call, and `from` and
/// `to` say which days it read so a screen shows the range it got rather than
/// the one it asked for.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "CashPositionDto.ts")]
pub struct CashPositionDto {
    pub from: String,
    pub to: String,
    pub cash_in: TakingsDto,
    pub cash_out: OutgoingsDto,
    pub cash_centimes: i64,
    pub card_in: TakingsDto,
}

impl TryFrom<CashPosition> for CashPositionDto {
    type Error = ApiError;

    fn try_from(p: CashPosition) -> Result<Self, ApiError> {
        Ok(CashPositionDto {
            from: p.from.format(DATE_FORMAT).to_string(),
            to: p.to.format(DATE_FORMAT).to_string(),
            cash_in: TakingsDto::try_from(p.cash_in)?,
            cash_out: OutgoingsDto::try_from(p.cash_out)?,
            cash_centimes: p.cash.as_centimes(),
            card_in: TakingsDto::try_from(p.card_in)?,
        })
    }
}

/// One product the recount put right: what the column said it had, what its
/// movements add up to, and the difference between them. The name travels
/// with the id because the panel is read by a person and it is what the log
/// stored, so a past run reads the same after a rename.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "StockDriftDto.ts")]
pub struct StockDriftDto {
    pub product_id: i32,
    pub name: String,
    pub cached_milli: i64,
    pub ledger_milli: i64,
    pub difference_milli: i64,
}

impl From<Drift> for StockDriftDto {
    fn from(d: Drift) -> Self {
        StockDriftDto {
            product_id: d.product_id,
            difference_milli: d.difference_milli(),
            name: d.name,
            cached_milli: d.cached_milli,
            ledger_milli: d.ledger_milli,
        }
    }
}

/// What one run of the recount found and did. `products_checked` is there so
/// an empty drift list reads as "nothing is wrong" rather than as "nothing
/// was looked at".
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "StockRecountDto.ts")]
pub struct StockRecountDto {
    /// The day on the shop's calendar the run was marked under.
    pub day: String,
    pub products_checked: i64,
    pub drifts: Vec<StockDriftDto>,
}

impl From<Report> for StockRecountDto {
    fn from(r: Report) -> Self {
        StockRecountDto {
            day: r.day,
            // A count of this shop's products. The bound is unreachable on
            // any file a shop could have, and a number that is merely
            // bounded beats a refusal on a screen that is only reporting.
            products_checked: i64::try_from(r.checked).unwrap_or(i64::MAX),
            drifts: r.drifts.into_iter().map(StockDriftDto::from).collect(),
        }
    }
}

/// The last run as the file remembers it. `last_run_day` is null when the
/// shop has never recounted, which is what a file opened for the first time
/// says before the daily loop has woken once.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "LastStockRecountDto.ts")]
pub struct LastStockRecountDto {
    pub last_run_day: Option<String>,
    pub drifts: Vec<StockDriftDto>,
}

impl From<LastRecount> for LastStockRecountDto {
    fn from(l: LastRecount) -> Self {
        LastStockRecountDto {
            last_run_day: l.last_run_day,
            drifts: l.drifts.into_iter().map(StockDriftDto::from).collect(),
        }
    }
}

/// `YYYY-MM` and nothing else: a month is the range a figure is asked over,
/// and "2026-9" or a day would be answered for another one.
///
/// The month is written back out and compared with what came in, the way
/// `parse_day` compares its day. Rust's integer parser takes a sign, so
/// "+026-09" is four characters of year that read as 26 and "2026-+9" two of
/// month that read as 9: both pass every check on shape and on length, and
/// only the round trip catches them. A figure answered for the year 26 is one
/// nobody would think to doubt.
pub fn parse_month(field: &'static str, text: &str) -> Result<Month, ApiError> {
    let refuse = || ApiError::Request(CoreError::validation(field, "a month is written YYYY-MM"));
    let (year, month) = text.split_once('-').ok_or_else(refuse)?;
    let year: i32 = year.parse().map_err(|_| refuse())?;
    let month: u32 = month.parse().map_err(|_| refuse())?;
    let parsed = Month::new(year, month).map_err(ApiError::Request)?;
    if parsed.as_text() != text {
        return Err(refuse());
    }
    Ok(parsed)
}

/// Where an order stands (features.md §1, Purchase). The whole union crosses
/// from the first version: a screen that met an unknown state could only
/// refuse the whole answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "PurchaseStatusDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum PurchaseStatusDto {
    Ordered,
    PartiallyReceived,
    Received,
    Cancelled,
    ClosedShort,
}

impl From<PurchaseStatus> for PurchaseStatusDto {
    fn from(s: PurchaseStatus) -> Self {
        match s {
            PurchaseStatus::Ordered => PurchaseStatusDto::Ordered,
            PurchaseStatus::PartiallyReceived => PurchaseStatusDto::PartiallyReceived,
            PurchaseStatus::Received => PurchaseStatusDto::Received,
            PurchaseStatus::Cancelled => PurchaseStatusDto::Cancelled,
            PurchaseStatus::ClosedShort => PurchaseStatusDto::ClosedShort,
        }
    }
}

impl From<PurchaseStatusDto> for PurchaseStatus {
    fn from(s: PurchaseStatusDto) -> Self {
        match s {
            PurchaseStatusDto::Ordered => PurchaseStatus::Ordered,
            PurchaseStatusDto::PartiallyReceived => PurchaseStatus::PartiallyReceived,
            PurchaseStatusDto::Received => PurchaseStatus::Received,
            PurchaseStatusDto::Cancelled => PurchaseStatus::Cancelled,
            PurchaseStatusDto::ClosedShort => PurchaseStatus::ClosedShort,
        }
    }
}

/// An order as the list reads it: the paper and nothing of its lines. The
/// list shows a row per order, and the lines are what `/purchases/{id}`
/// answers.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PurchaseDto.ts")]
pub struct PurchaseDto {
    pub id: i32,
    pub shop_id: i32,
    pub supplier_id: i32,
    /// The number written on the paper the supplier sent, when it carried
    /// one.
    pub supplier_document_number: Option<String>,
    /// `YYYY-MM-DD` on the shop's calendar.
    pub purchase_date: String,
    pub due_date: Option<String>,
    pub transport_centimes: i64,
    pub extra_costs_centimes: i64,
    pub status: PurchaseStatusDto,
    pub user_id: i32,
    pub note: Option<String>,
    /// `YYYY-MM-DD HH:MM:SS`, the moment the row was written.
    pub created_at: String,
}

impl From<Purchase> for PurchaseDto {
    fn from(p: Purchase) -> Self {
        PurchaseDto {
            id: p.id,
            shop_id: p.shop_id,
            supplier_id: p.supplier_id,
            supplier_document_number: p.supplier_document_number,
            purchase_date: p.purchase_date,
            due_date: p.due_date,
            transport_centimes: p.transport.as_centimes(),
            extra_costs_centimes: p.extra_costs.as_centimes(),
            status: p.status.into(),
            user_id: p.user_id,
            note: p.note,
            created_at: p.created_at.format(DATE_TIME_FORMAT).to_string(),
        }
    }
}

/// One product on an order, with what has arrived and what has gone back.
/// Both totals are the file's running columns, so a screen counting the
/// receipts itself would be a second answer.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PurchaseLineDto.ts")]
pub struct PurchaseLineDto {
    pub id: i32,
    pub product_id: i32,
    pub qty_ordered_milli: i64,
    /// What the supplier charges for the unit.
    pub unit_cost_centimes: i64,
    /// That plus this line's share of the transport and the extra costs,
    /// fixed when the order was saved.
    pub landed_unit_cost_centimes: i64,
    pub qty_received_milli: i64,
    pub qty_returned_milli: i64,
}

impl From<PurchaseLine> for PurchaseLineDto {
    fn from(l: PurchaseLine) -> Self {
        PurchaseLineDto {
            id: l.id,
            product_id: l.product_id,
            qty_ordered_milli: l.qty_ordered_milli,
            unit_cost_centimes: l.unit_cost.as_centimes(),
            landed_unit_cost_centimes: l.landed_unit_cost.as_centimes(),
            qty_received_milli: l.qty_received_milli,
            qty_returned_milli: l.qty_returned_milli,
        }
    }
}

/// What arrived on one delivery, line by line.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PurchaseReceiptLineDto.ts")]
pub struct PurchaseReceiptLineDto {
    pub purchase_line_id: i32,
    pub qty_milli: i64,
}

/// One bon de réception: the delivery, its number and what came on it. It is
/// not a document and takes no document number; the series is
/// `reception:<year>` and resets on 1 January like every other one.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PurchaseReceiptDto.ts")]
pub struct PurchaseReceiptDto {
    pub id: i32,
    pub series: String,
    pub number: i64,
    /// `YYYY-MM-DD HH:MM:SS` on the shop's calendar.
    pub received_at: String,
    pub user_id: i32,
    pub note: Option<String>,
    pub lines: Vec<PurchaseReceiptLineDto>,
}

/// A whole order: the paper, its lines with what has arrived and gone back,
/// and every delivery against it, newest first. Every route that changes an
/// order answers this, so the screen never has a change without the state it
/// left behind.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PurchaseDetailDto.ts")]
pub struct PurchaseDetailDto {
    pub purchase: PurchaseDto,
    pub lines: Vec<PurchaseLineDto>,
    pub receipts: Vec<PurchaseReceiptDto>,
}

impl From<PurchaseView> for PurchaseDetailDto {
    fn from(v: PurchaseView) -> Self {
        PurchaseDetailDto {
            purchase: PurchaseDto::from(v.purchase),
            lines: v.lines.into_iter().map(PurchaseLineDto::from).collect(),
            receipts: v
                .receipts
                .into_iter()
                .map(|r| PurchaseReceiptDto {
                    id: r.receipt.id,
                    series: r.receipt.series,
                    number: r.receipt.number,
                    received_at: r.receipt.received_at.format(DATE_TIME_FORMAT).to_string(),
                    user_id: r.receipt.user_id,
                    note: r.receipt.note,
                    lines: r
                        .lines
                        .into_iter()
                        .map(|l| PurchaseReceiptLineDto {
                            purchase_line_id: l.purchase_line_id,
                            qty_milli: l.qty_milli,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

/// One line of an order as the form sends it.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewPurchaseLineDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewPurchaseLineDto {
    pub product_id: i32,
    pub qty_ordered_milli: i64,
    pub unit_cost_centimes: i64,
}

/// Money handed to the supplier as the order is written. The mode is on it
/// because the ledger's file refuses a payment that does not say how it was
/// taken.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export_to = "PaidNowDto.ts")]
#[serde(deny_unknown_fields)]
pub struct PaidNowDto {
    pub amount_centimes: i64,
    pub payment_mode: PaymentMethodDto,
}

/// An order as the screen sends it. `receive_now` is the common case of
/// features.md §1: the goods came with the paper, so the whole receipt is
/// written in the same transaction.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewPurchaseDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewPurchaseDto {
    pub supplier_id: i32,
    #[serde(default)]
    pub supplier_document_number: Option<String>,
    pub purchase_date: String,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub transport_centimes: i64,
    #[serde(default)]
    pub extra_costs_centimes: i64,
    #[serde(default)]
    pub note: Option<String>,
    pub lines: Vec<NewPurchaseLineDto>,
    #[serde(default)]
    pub paid_now: Option<PaidNowDto>,
    #[serde(default)]
    pub receive_now: bool,
}

impl NewPurchaseDto {
    /// The order as the core takes it. Every amount is checked against the
    /// safe-integer bound here, at the edge, like every other one on the
    /// wire; what the amounts mean is the core's business.
    pub fn into_core(self) -> Result<NewPurchase, ApiError> {
        let mut lines = Vec::with_capacity(self.lines.len());
        for line in self.lines {
            lines.push(NewLine {
                product_id: line.product_id,
                qty_ordered_milli: line.qty_ordered_milli,
                unit_cost: Money::centimes(within_js_safe_range(
                    "unit_cost_centimes",
                    line.unit_cost_centimes,
                )?),
            });
        }
        let paid_now = match self.paid_now {
            None => None,
            Some(paid) => Some(Paid {
                amount: Money::centimes(within_js_safe_range(
                    "paid_now_centimes",
                    paid.amount_centimes,
                )?),
                mode: paid.payment_mode.into(),
            }),
        };
        Ok(NewPurchase {
            supplier_id: self.supplier_id,
            supplier_document_number: self.supplier_document_number,
            purchase_date: self.purchase_date,
            due_date: self.due_date,
            transport: Money::centimes(within_js_safe_range(
                "transport_centimes",
                self.transport_centimes,
            )?),
            extra_costs: Money::centimes(within_js_safe_range(
                "extra_costs_centimes",
                self.extra_costs_centimes,
            )?),
            note: self.note,
            lines,
            paid_now,
            receive_now: self.receive_now,
        })
    }
}

/// How much of one ordered line a delivery took in, or a return sent back.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export_to = "ReceiveLineDto.ts")]
#[serde(deny_unknown_fields)]
pub struct ReceiveLineDto {
    pub purchase_line_id: i32,
    pub qty_milli: i64,
}

impl From<ReceiveLineDto> for ReceiveLine {
    fn from(l: ReceiveLineDto) -> Self {
        ReceiveLine {
            purchase_line_id: l.purchase_line_id,
            qty_milli: l.qty_milli,
        }
    }
}

/// A delivery, or a return: the lines it names and a note if the shop wants
/// to say why. The two carry the same fields because they are the same
/// question asked in two directions, and the route is what says which.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewReceiptDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewReceiptDto {
    pub lines: Vec<ReceiveLineDto>,
    #[serde(default)]
    pub note: Option<String>,
}

/// Why an order was cancelled or closed short. Required, because writing off
/// goods that never came is a decision and the audit log is where it is
/// written down.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "CloseOrderDto.ts")]
#[serde(deny_unknown_fields)]
pub struct CloseOrderDto {
    pub reason: String,
}

/// What one stretch of days came to (features.md §1, Dashboard). Every field
/// is derived from the ledgers when the screen asks; no column stores any of
/// it.
///
/// `sales_ttc_centimes` is what the tickets and factures of the period asked
/// for over the counter, credit notes not taken off it. The margin is the
/// other question: `lines_ht_centimes` less `discounts_centimes` is the
/// revenue, `cost_of_goods_centimes` is what those goods cost at the cost
/// they left on, and `margin_centimes` is the difference. A credit note
/// lowers the revenue and the cost together, so what it leaves is the margin
/// of what the customer kept.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export_to = "DashboardFiguresDto.ts")]
pub struct DashboardFiguresDto {
    pub sales_ttc_centimes: i64,
    pub sales_count: i64,
    pub lines_ht_centimes: i64,
    pub discounts_centimes: i64,
    pub sales_ht_centimes: i64,
    pub cost_of_goods_centimes: i64,
    pub margin_centimes: i64,
    pub expenses_centimes: i64,
}

impl From<Figures> for DashboardFiguresDto {
    fn from(f: Figures) -> Self {
        DashboardFiguresDto {
            sales_ttc_centimes: f.sales_ttc.as_centimes(),
            sales_count: f.sales_count,
            lines_ht_centimes: f.lines_ht.as_centimes(),
            discounts_centimes: f.discounts.as_centimes(),
            sales_ht_centimes: f.sales_ht.as_centimes(),
            cost_of_goods_centimes: f.cost_of_goods.as_centimes(),
            margin_centimes: f.margin.as_centimes(),
            expenses_centimes: f.expenses.as_centimes(),
        }
    }
}

/// A product the shop is short of: what the count says it has, and the
/// threshold somebody set on the fiche. Quantities are thousandths of the
/// unit, the way every quantity on the wire is.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "LowStockDto.ts")]
pub struct LowStockDto {
    pub product_id: i32,
    pub name: String,
    pub qty_on_hand_milli: i64,
    pub low_stock_at_milli: i64,
}

impl From<LowStock> for LowStockDto {
    fn from(l: LowStock) -> Self {
        LowStockDto {
            product_id: l.product_id,
            name: l.name,
            qty_on_hand_milli: l.qty_on_hand_milli,
            low_stock_at_milli: l.low_stock_at_milli,
        }
    }
}

/// One product's month. `qty_milli` is net of what came back, so a product
/// sold and credited in the same month reads as nothing moved.
///
/// `lines_ht_centimes` is this product's lines and not its share of a
/// remise given off a whole document, which belongs to no line. It is
/// therefore not the same figure as `DashboardFiguresDto::sales_ht_centimes`,
/// and the margins of the products on a month that carried a remise do not
/// add up to that month's margin. The ranking is what these are for.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "TopProductDto.ts")]
pub struct TopProductDto {
    pub product_id: i32,
    pub name: String,
    pub qty_milli: i64,
    pub lines_ht_centimes: i64,
    pub cost_of_goods_centimes: i64,
    pub margin_centimes: i64,
}

impl From<TopProduct> for TopProductDto {
    fn from(p: TopProduct) -> Self {
        TopProductDto {
            product_id: p.product_id,
            name: p.name,
            qty_milli: p.qty_milli,
            lines_ht_centimes: p.lines_ht.as_centimes(),
            cost_of_goods_centimes: p.cost_of_goods.as_centimes(),
            margin_centimes: p.margin.as_centimes(),
        }
    }
}

/// One side of the outstanding money. `parties` counts only those in the red:
/// a customer holding credit is left out rather than netted off, because
/// money the shop owes one of them does not reduce what another one owes.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export_to = "OwedDto.ts")]
pub struct OwedDto {
    pub total_centimes: i64,
    pub parties: i64,
}

impl From<Owed> for OwedDto {
    fn from(o: Owed) -> Self {
        OwedDto {
            total_centimes: o.total.as_centimes(),
            parties: o.parties,
        }
    }
}

/// The whole dashboard for one day and the month it falls in on the shop's
/// calendar. The two top lists are the month's, not the day's: a day names
/// too few products for a ranking to say anything.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "DashboardDto.ts")]
pub struct DashboardDto {
    pub day: String,
    pub month: String,
    pub today: DashboardFiguresDto,
    pub this_month: DashboardFiguresDto,
    pub cash_today: CashPositionDto,
    pub cash_this_month: CashPositionDto,
    pub low_stock: Vec<LowStockDto>,
    pub top_by_quantity: Vec<TopProductDto>,
    pub top_by_margin: Vec<TopProductDto>,
    pub customer_debt: OwedDto,
    pub supplier_debt: OwedDto,
    pub open_purchases: i64,
}

impl TryFrom<Dashboard> for DashboardDto {
    type Error = ApiError;

    fn try_from(d: Dashboard) -> Result<Self, ApiError> {
        Ok(DashboardDto {
            day: d.day.format(DATE_FORMAT).to_string(),
            month: d.month.as_text(),
            today: DashboardFiguresDto::from(d.today),
            this_month: DashboardFiguresDto::from(d.this_month),
            cash_today: CashPositionDto::try_from(d.cash_today)?,
            cash_this_month: CashPositionDto::try_from(d.cash_this_month)?,
            low_stock: d.low_stock.into_iter().map(LowStockDto::from).collect(),
            top_by_quantity: d
                .top_by_quantity
                .into_iter()
                .map(TopProductDto::from)
                .collect(),
            top_by_margin: d
                .top_by_margin
                .into_iter()
                .map(TopProductDto::from)
                .collect(),
            customer_debt: OwedDto::from(d.customer_debt),
            supplier_debt: OwedDto::from(d.supplier_debt),
            open_purchases: d.open_purchases,
        })
    }
}

/// One bucket of the dashboard's chart: a day, or the week its days were
/// folded into. `from` and `to` are both included and they are equal on a
/// day, so a tooltip names the range the figure covers rather than the one
/// the screen asked for.
///
/// `cash_in_centimes` is the drawer's side alone, sales paid on the spot plus
/// money handed over against a debt. The card takings are a movement of the
/// bank and not of the till, so they are not in this line; the day's own
/// `/cash` answer is where they are.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "DashboardSeriesPointDto.ts")]
pub struct DashboardSeriesPointDto {
    pub from: String,
    pub to: String,
    pub figures: DashboardFiguresDto,
    pub cash_in_centimes: i64,
}

impl From<SeriesPoint> for DashboardSeriesPointDto {
    fn from(p: SeriesPoint) -> Self {
        DashboardSeriesPointDto {
            from: p.from.format(DATE_FORMAT).to_string(),
            to: p.to.format(DATE_FORMAT).to_string(),
            figures: DashboardFiguresDto::from(p.figures),
            cash_in_centimes: p.cash_in.as_centimes(),
        }
    }
}

/// The dashboard's chart: a stretch of days ending on the day the screen
/// asked about, each on its own and folded into weeks. Both lists run oldest
/// first, and `days` skips nothing: a day the shop sold nothing is a row of
/// zeros, because a gap in a chart reads as a day it was shut.
///
/// The weeks are cut back from `to`, so the last bucket is a whole week of
/// the days the shop is in and the odd ones fall at the far end. Thirty days
/// is four weeks and two days.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "DashboardSeriesDto.ts")]
pub struct DashboardSeriesDto {
    pub from: String,
    pub to: String,
    pub days: Vec<DashboardSeriesPointDto>,
    pub weeks: Vec<DashboardSeriesPointDto>,
}

impl From<Series> for DashboardSeriesDto {
    fn from(s: Series) -> Self {
        DashboardSeriesDto {
            from: s.from.format(DATE_FORMAT).to_string(),
            to: s.to.format(DATE_FORMAT).to_string(),
            days: s
                .days
                .into_iter()
                .map(DashboardSeriesPointDto::from)
                .collect(),
            weeks: s
                .weeks
                .into_iter()
                .map(DashboardSeriesPointDto::from)
                .collect(),
        }
    }
}

/// What the import would do with one row of the file, flattened for the
/// wire: three words rather than a tagged union, with the field and the
/// reason beside them.
///
/// `field` and `reason` are the core's stable keys, not sentences: the
/// screen translates them, the same way it translates an error code
/// (architecture.md, error policy). A row that is created or updated
/// carries neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "ImportOutcomeDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum ImportOutcomeDto {
    Created,
    Updated,
    Refused,
}

/// One line of the dry run, named the way a person reading the spreadsheet
/// beside it would: the row number the spreadsheet shows, header counted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "ImportRowDto.ts")]
pub struct ImportRowDto {
    pub row: u32,
    pub name: String,
    pub outcome: ImportOutcomeDto,
    /// The column the refusal is about, and why. Null on a row that stands.
    pub field: Option<String>,
    pub reason: Option<String>,
}

impl From<&RowReport> for ImportRowDto {
    fn from(r: &RowReport) -> Self {
        let (outcome, field, reason) = match r.outcome {
            Outcome::Created => (ImportOutcomeDto::Created, None, None),
            Outcome::Updated => (ImportOutcomeDto::Updated, None, None),
            Outcome::Refused { field, reason } => (
                ImportOutcomeDto::Refused,
                Some(field.to_owned()),
                Some(reason.to_owned()),
            ),
        };
        ImportRowDto {
            row: r.row,
            name: r.name.clone(),
            outcome,
            field,
            reason,
        }
    }
}

/// The whole dry run: every row with its verdict, and the two counts the
/// screen puts above the table. Nothing was written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "ImportDryRunDto.ts")]
pub struct ImportDryRunDto {
    pub rows: Vec<ImportRowDto>,
    pub accepted: i64,
    pub refused: i64,
}

impl From<DryRun> for ImportDryRunDto {
    fn from(d: DryRun) -> Self {
        ImportDryRunDto {
            rows: d.rows.iter().map(ImportRowDto::from).collect(),
            accepted: i64::try_from(d.accepted).unwrap_or(i64::MAX),
            refused: i64::try_from(d.refused).unwrap_or(i64::MAX),
        }
    }
}

/// What an apply wrote: the counts the audit row carries, so the screen and
/// the log say the same thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "ImportAppliedDto.ts")]
pub struct ImportAppliedDto {
    pub created: i64,
    pub updated: i64,
    pub categories_created: i64,
}

impl From<Applied> for ImportAppliedDto {
    fn from(a: Applied) -> Self {
        ImportAppliedDto {
            created: i64::try_from(a.created).unwrap_or(i64::MAX),
            updated: i64::try_from(a.updated).unwrap_or(i64::MAX),
            categories_created: i64::try_from(a.categories_created).unwrap_or(i64::MAX),
        }
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

/// What a sign-in sends. Two shapes and not one struct with four optional
/// fields: the till's PIN pad picks a row off the list and sends an id, the
/// office screen asks for a name and a password, and a body carrying a name
/// beside a PIN is a caller that has not decided which it is doing. Untagged,
/// so the wire stays the two plain objects a screen would send anyway.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "LoginDto.ts")]
#[serde(untagged)]
pub enum LoginDto {
    /// The till. `user_id` and never a name: a PIN pad has the list in front
    /// of it, and a name typed at a keypad would be a way to ask the shop
    /// whether somebody works there.
    Pin { user_id: i32, pin: String },
    /// Everywhere else.
    Password { name: String, password: String },
}

/// Who is signed in, as every screen reads it: the person, their role, and
/// what that role may do.
///
/// The permission list travels because the permission table lives in the core
/// (`services::permissions::can`) and a screen that decided for itself which
/// buttons a manager gets would be a second statement of it
/// (architecture.md rule 2). T5's `Can` component reads this array and
/// nothing else; no screen compares a role string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "MeDto.ts")]
pub struct MeDto {
    pub user_id: i32,
    pub name: String,
    pub role: RoleDto,
    pub permissions: Vec<PermissionDto>,
}

/// A sign-in's answer: who is now acting, and the session token.
///
/// The token is in the body because the desktop webview cannot read the
/// httpOnly cookie the same response sets, and a Tauri window has no other
/// way to learn it. A browser client ignores this field and lets the cookie
/// travel; the cookie is what protects a session already open, and an
/// attacker who could read this body would have had to send the PIN to get
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "SessionDto.ts")]
pub struct SessionDto {
    pub me: MeDto,
    pub token: String,
    /// How long the session survives with nothing happening on it, in whole
    /// minutes. The screen counts the lock screen down against this rather
    /// than holding a figure of its own (T4).
    pub idle_minutes: i64,
}

/// The three roles. One value the screens read, never a comparison they make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "RoleDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum RoleDto {
    Owner,
    Manager,
    Cashier,
}

impl From<Role> for RoleDto {
    fn from(r: Role) -> Self {
        match r {
            Role::Owner => RoleDto::Owner,
            Role::Manager => RoleDto::Manager,
            Role::Cashier => RoleDto::Cashier,
        }
    }
}

impl From<RoleDto> for Role {
    fn from(r: RoleDto) -> Self {
        match r {
            RoleDto::Owner => Role::Owner,
            RoleDto::Manager => Role::Manager,
            RoleDto::Cashier => Role::Cashier,
        }
    }
}

/// The permission list, one variant per `services::permissions::Permission`.
/// Serialised as the same string `Permission::as_str` writes, which is also
/// the name a 403 carries, so a screen matches one spelling everywhere.
///
/// The `From` below matches on the core enum, so a fourteenth permission
/// fails to compile here until it is named on the wire too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "PermissionDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum PermissionDto {
    Sell,
    DiscountAboveThreshold,
    OverrideCreditBlock,
    SeeCostAndMargin,
    EditFiches,
    EditSettings,
    SeeReports,
    ManageUsers,
    CommitMoney,
    CorrectLedger,
    ExportAndImport,
    ChangePriceAtTheTill,
    SeeAuditLog,
}

impl From<Permission> for PermissionDto {
    fn from(p: Permission) -> Self {
        match p {
            Permission::Sell => PermissionDto::Sell,
            Permission::DiscountAboveThreshold => PermissionDto::DiscountAboveThreshold,
            Permission::OverrideCreditBlock => PermissionDto::OverrideCreditBlock,
            Permission::SeeCostAndMargin => PermissionDto::SeeCostAndMargin,
            Permission::EditFiches => PermissionDto::EditFiches,
            Permission::EditSettings => PermissionDto::EditSettings,
            Permission::SeeReports => PermissionDto::SeeReports,
            Permission::ManageUsers => PermissionDto::ManageUsers,
            Permission::CommitMoney => PermissionDto::CommitMoney,
            Permission::CorrectLedger => PermissionDto::CorrectLedger,
            Permission::ExportAndImport => PermissionDto::ExportAndImport,
            Permission::ChangePriceAtTheTill => PermissionDto::ChangePriceAtTheTill,
            Permission::SeeAuditLog => PermissionDto::SeeAuditLog,
        }
    }
}

/// The idle time a shop has set, in whole minutes. Its own body rather than a
/// field of the settings block, because T2 ships the mechanism and the
/// settings screen that edits it is T8's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "SessionIdleDto.ts")]
#[serde(deny_unknown_fields)]
pub struct SessionIdleDto {
    pub idle_minutes: i64,
}

/// One row of the audit log, for the owner's screen (M4 T7, features.md
/// §5). `before` and `after` are the JSON documents the writing service
/// gave `services::audit::record`, sent through unparsed: the screen reads
/// them as the shape the service that wrote the row chose, and this layer
/// never guesses one.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "AuditEntryDto.ts")]
pub struct AuditEntryDto {
    pub id: i32,
    pub user_id: i32,
    /// Carried on the row because there is no `/users` list route yet for
    /// the screen to join it itself (`services::audit::EntryWithUser`).
    pub user_name: String,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<i32>,
    pub before: Option<String>,
    pub after: Option<String>,
    /// `YYYY-MM-DD HH:MM:SS` on the shop's calendar, the same clock every
    /// other date this app shows is read on.
    pub created_at: String,
}

impl From<dzpos_core::services::audit::EntryWithUser> for AuditEntryDto {
    fn from(e: dzpos_core::services::audit::EntryWithUser) -> Self {
        AuditEntryDto {
            id: e.entry.id,
            user_id: e.entry.user_id,
            user_name: e.user_name,
            action: e.entry.action,
            entity: e.entry.entity,
            entity_id: e.entry.entity_id,
            before: e.entry.before,
            after: e.entry.after,
            created_at: e.entry.created_at.format(DATE_TIME_FORMAT).to_string(),
        }
    }
}

/// A user the audit log's `user_id` filter offers, whether or not they have
/// written a row.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "AuditUserDto.ts")]
pub struct AuditUserDto {
    pub id: i32,
    pub name: String,
}

/// One page of the log, filtered, and what the screen's two dropdowns may
/// narrow it by. The dropdowns' own options travel with every page rather
/// than being their own route, because they are cheap beside the rows
/// (`services::audit::read` reads the shop's users and its distinct actions
/// once, not once per row) and a screen that filtered down to nothing would
/// otherwise have no way to offer the other choices back.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "AuditLogDto.ts")]
pub struct AuditLogDto {
    pub rows: Vec<AuditEntryDto>,
    pub page: i64,
    /// Whether `page + 1` would answer more rows under the same filter.
    pub has_more: bool,
    pub users: Vec<AuditUserDto>,
    pub actions: Vec<String>,
}

impl AuditLogDto {
    pub fn from_core(
        page: dzpos_core::services::audit::Page,
        facets: dzpos_core::services::audit::Facets,
    ) -> Self {
        AuditLogDto {
            rows: page.rows.into_iter().map(AuditEntryDto::from).collect(),
            page: page.page,
            has_more: page.has_more,
            users: facets
                .users
                .into_iter()
                .map(|(id, name)| AuditUserDto { id, name })
                .collect(),
            actions: facets.actions,
        }
    }
}

/// A fiche on the users screen (M4 T8). No hash and no failure counter ever
/// travel: `has_pin` and `has_password` are the honest answer to "can this
/// person sign in", and a reset never has an old PIN to show because there
/// is not one to show (`services::users`' own doc).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "UserDto.ts")]
pub struct UserDto {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub role: RoleDto,
    pub has_pin: bool,
    pub has_password: bool,
    pub active: bool,
}

impl From<User> for UserDto {
    fn from(u: User) -> Self {
        UserDto {
            id: u.id,
            shop_id: u.shop_id,
            name: u.name,
            role: RoleDto::from(u.role),
            has_pin: u.has_pin,
            has_password: u.has_password,
            active: u.active,
        }
    }
}

/// One name on the sign-in picker. `GET /auth/staff` answers a list of these
/// to a caller that is inside the device gate and has no session yet, so a
/// cashier taps their name and types only their PIN: the user id is a row
/// number nobody standing at a counter knows, and the screens that asked
/// for it were asking for the database's key.
///
/// The fields are chosen for what leaves the shop if a paired phone is
/// stolen: a name, a role, and which door opens for that name. No id of the
/// shop, no `active` (the list holds only active fiches), and no hash, the
/// same rule `UserDto` keeps. `has_pin` decides which box the picker shows
/// after the tap; `has_password` is what the owner's door needs to know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "StaffDto.ts")]
pub struct StaffDto {
    pub id: i32,
    pub name: String,
    pub role: RoleDto,
    pub has_pin: bool,
    pub has_password: bool,
}

impl From<User> for StaffDto {
    fn from(u: User) -> Self {
        StaffDto {
            id: u.id,
            name: u.name,
            role: RoleDto::from(u.role),
            has_pin: u.has_pin,
            has_password: u.has_password,
        }
    }
}

/// A fiche's name and role, the two fields the screen's "add a user" dialog
/// sends. No credential: a PIN is its own call
/// (`services::users::create`'s own doc), so `POST /users/{id}/pin` is what
/// gives a fresh row its first one.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewUserDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewUserDto {
    pub name: String,
    pub role: RoleDto,
}

/// The body `POST /users/{id}/pin` takes: a PIN alone, on the fiche the path
/// names. The same call gives a fresh row its first PIN and resets one that
/// is forgotten; `services::users::set_pin` does not tell the two apart and
/// neither does this.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "SetPinDto.ts")]
#[serde(deny_unknown_fields)]
pub struct SetPinDto {
    pub pin: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "SetPasswordDto.ts")]
#[serde(deny_unknown_fields)]
pub struct SetPasswordDto {
    pub password: String,
}

/// The body `POST /auth/first-setup` takes: a name and a password, no
/// `user_id`. Nobody signed in yet is not in a position to name a row; the
/// shop's own owner is who `services::users::claim_first_owner` finds and
/// acts on. The PIN for the till is set later from the users screen.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "ClaimFirstOwnerDto.ts")]
#[serde(deny_unknown_fields)]
pub struct ClaimFirstOwnerDto {
    pub name: String,
    pub password: String,
}

/// The QR the desktop shows for the phone to scan (M6 T2). The token is
/// 64 hex, 60s single-use; the phone trades it for a device token.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PairingQrDto.ts")]
pub struct PairingQrDto {
    pub pairing_token: String,
    pub expires_in_seconds: i64,
}

/// The long-lived device token the phone keeps after pairing (M6 T2). Like
/// `SessionDto.token`, 64 hex, stored as hash, shown once.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "DeviceTokenDto.ts")]
pub struct DeviceTokenDto {
    pub device_token: String,
}

/// A paired phone as the settings screen lists it (M6 T4). The token hash
/// never leaves the server; the row is what the screen revokes.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PairedDeviceDto.ts")]
pub struct PairedDeviceDto {
    pub id: i32,
    pub name: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
    pub created_by: i32,
}

impl From<dzpos_core::models::pairing::PairedDeviceRow> for PairedDeviceDto {
    fn from(row: dzpos_core::models::pairing::PairedDeviceRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            created_at: row.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            revoked_at: row
                .revoked_at
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            created_by: row.created_by,
        }
    }
}

#[cfg(test)]
mod audit_dto_tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use dzpos_core::models::audit::AuditEntry;
    use dzpos_core::services::audit::EntryWithUser;

    /// The moment travels through this conversion untouched. It used to be
    /// shifted here, because the column held UTC and the screen wanted the
    /// shop's calendar; the row is stamped from the shop clock now
    /// (`2026-09-12-000013_audit_log_shop_clock`) and a shift put back by
    /// hand would print every row an hour late. The seconds are also the
    /// last of it: `clock::now()` carries nanoseconds and the column keeps
    /// them, so the format string is what stands between the owner's screen
    /// and a date ending in nine digits.
    #[test]
    fn the_moment_is_printed_as_it_stands_to_the_second() {
        let entry = AuditEntry {
            id: 1,
            shop_id: 1,
            user_id: 1,
            action: "product.update".to_string(),
            entity: "product".to_string(),
            entity_id: Some(7),
            before: None,
            after: None,
            created_at: chrono::NaiveDate::from_ymd_opt(2026, 9, 12)
                .unwrap()
                .and_hms_nano_opt(0, 30, 0, 123_456_789)
                .unwrap(),
        };
        let dto = AuditEntryDto::from(EntryWithUser {
            entry,
            user_name: "Propriétaire".to_string(),
        });
        assert_eq!(dto.created_at, "2026-09-12 00:30:00");
    }
}
