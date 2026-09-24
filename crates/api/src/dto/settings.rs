//! The shop block, its fiscal regime, its theme and the rest of the settings.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

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

/// Which of the shop's facture layouts its factures are drawn in.
///
/// Not the paper. The sheet is named on each print, because the till knows
/// which tray the cashier reached for and the shop does not; the layout is
/// chosen once and every facture follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "FactureLayoutDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum FactureLayoutDto {
    Standard,
    Compact,
    HalfSheet,
    /// Named outright for the reason `crates/core`'s own variant is:
    /// `snake_case` reads this as `roll80` and the stored spelling is
    /// `roll_80mm`.
    #[serde(rename = "roll_80mm")]
    #[ts(rename = "roll_80mm")]
    Roll80,
}

impl From<FactureLayout> for FactureLayoutDto {
    fn from(layout: FactureLayout) -> Self {
        match layout {
            FactureLayout::Standard => FactureLayoutDto::Standard,
            FactureLayout::Compact => FactureLayoutDto::Compact,
            FactureLayout::HalfSheet => FactureLayoutDto::HalfSheet,
            FactureLayout::Roll80 => FactureLayoutDto::Roll80,
        }
    }
}

impl From<FactureLayoutDto> for FactureLayout {
    fn from(dto: FactureLayoutDto) -> Self {
        match dto {
            FactureLayoutDto::Standard => FactureLayout::Standard,
            FactureLayoutDto::Compact => FactureLayout::Compact,
            FactureLayoutDto::HalfSheet => FactureLayout::HalfSheet,
            FactureLayoutDto::Roll80 => FactureLayout::Roll80,
        }
    }
}

/// The layout the settings screen is putting the shop on.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export_to = "FactureLayoutChoiceDto.ts")]
#[serde(deny_unknown_fields)]
pub struct FactureLayoutChoiceDto {
    pub facture_layout: FactureLayoutDto,
}

/// The language every fiscal paper prints in: the ticket, the facture, the
/// customer statement and the debt slip alike
/// (`context/plans/20260920-a-print-language-the-shop-keeps.md`). The same
/// three `print::strings` already holds and the screen's own three
/// (`apps/desktop/src/i18n`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "PrintLangDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum PrintLangDto {
    Fr,
    En,
    Ar,
}

impl From<Lang> for PrintLangDto {
    fn from(l: Lang) -> Self {
        match l {
            Lang::Fr => PrintLangDto::Fr,
            Lang::En => PrintLangDto::En,
            Lang::Ar => PrintLangDto::Ar,
        }
    }
}

impl From<PrintLangDto> for Lang {
    fn from(d: PrintLangDto) -> Self {
        match d {
            PrintLangDto::Fr => Lang::Fr,
            PrintLangDto::En => Lang::En,
            PrintLangDto::Ar => Lang::Ar,
        }
    }
}

/// The print language the settings screen is putting the shop on. `null` is
/// not a missing answer: it is the shop asking to forget its choice, which
/// puts every fiscal paper back on the till's own language, the way
/// `ThemeChoiceDto`'s `null` puts the screen back on Comptoir.
///
/// The two are told apart rather than trusted to differ, which is why the
/// field is an option of an option and arrives through `answered` below.
/// Serde fills a plain missing `Option` field with `None`, so a caller that
/// dropped the key would have read as the shop asking to forget, and a
/// screen with a bug in it would have quietly put a shop that prints Arabic
/// back on the till's language with nothing refused and nothing logged. The
/// outer `None` is "you did not answer", and the route turns it into a 422.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export_to = "PrintLangChoiceDto.ts")]
#[serde(deny_unknown_fields)]
pub struct PrintLangChoiceDto {
    #[serde(default, deserialize_with = "answered")]
    #[ts(as = "Option<PrintLangDto>")]
    pub print_lang: Option<Option<PrintLangDto>>,
}

/// `Some(None)` for a field that is there and `null`, `Some(Some(_))` for one
/// carrying a language. A field that is not there never reaches this, and
/// `serde(default)` leaves it `None`.
fn answered<'de, D>(d: D) -> Result<Option<Option<PrintLangDto>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::deserialize(d).map(Some)
}

/// Which ESC/POS path the shop's thermal head is sent: one byte per column
/// down a single-byte table, or the same lines drawn into dots and sent as
/// `GS v 0` bands
/// (`context/plans/20260921-arabic-on-a-cheap-thermal-head.md`).
///
/// It is not the whole answer for a given paper. Arabic has no single-byte
/// table on a cheap head, so it is drawn whatever a shop stores here; the
/// core's `ThermalMode::for_lang` is the one place that decides it, and no
/// handler repeats the rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "ThermalModeDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum ThermalModeDto {
    Text,
    Raster,
}

impl From<ThermalMode> for ThermalModeDto {
    fn from(m: ThermalMode) -> Self {
        match m {
            ThermalMode::Text => ThermalModeDto::Text,
            ThermalMode::Raster => ThermalModeDto::Raster,
        }
    }
}

impl From<ThermalModeDto> for ThermalMode {
    fn from(d: ThermalModeDto) -> Self {
        match d {
            ThermalModeDto::Text => ThermalMode::Text,
            ThermalModeDto::Raster => ThermalMode::Raster,
        }
    }
}

/// The path the settings screen is putting the shop's head on.
///
/// No `null` arm, unlike the print language beside it, and for the reason
/// `FactureLayoutChoiceDto` has none: a head is always sent down one of the
/// two paths, and "none" would be `text` under another name.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export_to = "ThermalModeChoiceDto.ts")]
#[serde(deny_unknown_fields)]
pub struct ThermalModeChoiceDto {
    pub thermal_mode: ThermalModeDto,
}

/// Whether the shop is putting its NIF, RC, NIS and AI on the ticket too
/// (they print on the facture whatever this says). No `null` arm, the same
/// reason the thermal mode has none: the ticket is always one of the two
/// shapes.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export_to = "TicketFiscalIdsChoiceDto.ts")]
#[serde(deny_unknown_fields)]
pub struct TicketFiscalIdsChoiceDto {
    pub ticket_fiscal_ids: bool,
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
    /// Which layout its factures print in. Never null: a shop that has never
    /// chosen prints on `standard`, so the screen has a value to show and the
    /// printer has a page to draw.
    pub facture_layout: FactureLayoutDto,
    /// Every layout the shop may pick, so the screen does not carry its own
    /// copy of the list and go stale when one is added.
    pub facture_layouts: Vec<FactureLayoutDto>,
    /// The language every fiscal paper prints in. `null` when the shop has
    /// never chosen one, which is not French by default: the till prints in
    /// whatever language it is being used in.
    pub print_lang: Option<PrintLangDto>,
    /// Which ESC/POS path its thermal head is sent. Never null: a shop that
    /// has never chosen is on `text`, so the screen has a value to show and
    /// the head has a wire to eat. An Arabic paper is drawn whatever this
    /// says, which the panel's own hint tells the owner.
    pub thermal_mode: ThermalModeDto,
    /// How much a cashier may take off a basket before the sale needs
    /// someone holding `discount_above_threshold`, in basis points of the
    /// basket before any discount (250 is 2,5 %). Zero on a shop that has
    /// never set one, which refuses a cashier every discount: the screen
    /// should say so rather than leave an owner wondering why the till
    /// refuses a round number off. Retail-only (S5): a discount is given on
    /// a sale, which does not exist without the feature.
    #[cfg(feature = "retail")]
    pub discount_threshold_bps: u32,
    /// Whether the ticket also carries the seller's NIF, RC, NIS and AI.
    /// Off on a shop that has never chosen: those four print on the
    /// facture regardless, and a till receipt is not the paper an
    /// Algerian text asks anything of (T55).
    pub ticket_fiscal_ids: bool,
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
/// still be read against the threshold that refused it. Retail-only (S5):
/// no sale exists to refuse without the feature.
#[cfg(feature = "retail")]
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "DiscountThresholdChangeDto.ts")]
#[serde(deny_unknown_fields)]
pub struct DiscountThresholdChangeDto {
    pub threshold_bps: u32,
    pub valid_from: String,
}
