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
