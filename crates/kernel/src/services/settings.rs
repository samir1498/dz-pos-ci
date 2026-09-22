//! Shop settings that carry a date. A document reads the row that was
//! current when it was issued, so the régime fiscal it printed under stays
//! readable after the shop changes régime (features.md, Régime fiscal row).

use chrono::NaiveDateTime;
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::money::{Bps, Regime};
use crate::repos::settings as repo;
use crate::services::audit;

/// The key holding the shop's régime fiscal. `ifu` or `reel`; the migration
/// carries the same CHECK.
pub const REGIME_FISCAL: &str = "regime_fiscal";

/// A régime and the moment it took, or takes, effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatedRegime {
    pub regime: Regime,
    pub valid_from: NaiveDateTime,
}

/// The régime current at `at` and since when. The settings screen reads
/// this; a document reads `regime_as_of`.
pub fn regime_current(
    conn: &mut SqliteConnection,
    shop_id: i32,
    at: NaiveDateTime,
) -> Result<DatedRegime, CoreError> {
    let (value, valid_from) =
        repo::current_as_of(conn, shop_id, REGIME_FISCAL, at)?.ok_or(CoreError::NotFound {
            entity: "regime_fiscal",
            id: shop_id,
        })?;
    Ok(DatedRegime {
        regime: parse(&value)?,
        valid_from,
    })
}

/// A régime change dated after `at` that has not taken effect yet, so the
/// screen can show "IFU from 2027-01-01" instead of hiding what the owner
/// just entered.
pub fn regime_planned(
    conn: &mut SqliteConnection,
    shop_id: i32,
    at: NaiveDateTime,
) -> Result<Option<DatedRegime>, CoreError> {
    repo::next_after(conn, shop_id, REGIME_FISCAL, at)?
        .map(|(value, valid_from)| {
            Ok(DatedRegime {
                regime: parse(&value)?,
                valid_from,
            })
        })
        .transpose()
}

/// The régime the shop was under at `at`. A document computes its totals
/// with this, never with whatever the shop is on today.
pub fn regime_as_of(
    conn: &mut SqliteConnection,
    shop_id: i32,
    at: NaiveDateTime,
) -> Result<Regime, CoreError> {
    let value =
        repo::value_as_of(conn, shop_id, REGIME_FISCAL, at)?.ok_or(CoreError::NotFound {
            entity: "regime_fiscal",
            id: shop_id,
        })?;
    parse(&value)
}

fn parse(value: &str) -> Result<Regime, CoreError> {
    match value {
        "reel" => Ok(Regime::Reel),
        "ifu" => Ok(Regime::Ifu),
        // The migration's CHECK keeps this out, so a row here means a file
        // written by something else. It is an error, never a guess.
        other => Err(CoreError::validation(
            REGIME_FISCAL,
            &format!("{other} is not a régime fiscal"),
        )),
    }
}

/// Records that the shop is under `regime` from `valid_from`. Earlier rows
/// stay: they are what earlier documents printed under.
pub fn set_regime(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    regime: Regime,
    valid_from: NaiveDateTime,
) -> Result<(), CoreError> {
    conn.transaction(|conn| {
        let before = repo::value_as_of(conn, shop_id, REGIME_FISCAL, valid_from)?;
        // The shop is already under that régime on that day: nothing to
        // record. Accepting it would move the "since" date a comptable reads
        // to the day of the click. Before the first row there is no régime to
        // compare with, so a first entry is always accepted.
        if before.as_deref() == Some(stored(regime)) {
            return Err(CoreError::validation(
                REGIME_FISCAL,
                "the shop is already under that régime on that day",
            ));
        }
        repo::append(conn, shop_id, REGIME_FISCAL, stored(regime), valid_from)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_SET_REGIME,
                entity: REGIME_FISCAL,
                entity_id: Some(shop_id),
                before: before.map(|value| serde_json::json!({ "regime": value }).to_string()),
                after: Some(
                    serde_json::json!({
                        "regime": stored(regime),
                        "valid_from": valid_from.to_string(),
                    })
                    .to_string(),
                ),
            },
        )
    })
}

const fn stored(regime: Regime) -> &'static str {
    match regime {
        Regime::Reel => "reel",
        Regime::Ifu => "ifu",
    }
}

/// The key holding the rate above which a discount needs
/// `services::permissions::Permission::DiscountAboveThreshold` (this
/// milestone's brief: "the discount threshold becomes a settings value with
/// a dated history, exactly the way the régime fiscal already does it").
/// Unlike `REGIME_FISCAL`, no migration seeds a first row: T1 writes no
/// migration, so a shop that has never set one reads as `Bps::ZERO`
/// (`discount_threshold_as_of`), the conservative default that needs the
/// permission for any discount at all until an owner raises it.
pub const DISCOUNT_THRESHOLD_BPS: &str = "discount_threshold_bps";

/// A discount threshold and the moment it took, or takes, effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatedThreshold {
    pub threshold: Bps,
    pub valid_from: NaiveDateTime,
}

/// The threshold current at `at`, and since when. `None` when the shop has
/// never set one: there is no seeded row to fall back on the way
/// `regime_current` has, and inventing a "since" date for a default nobody
/// chose would be a second, silent decision.
pub fn discount_threshold_current(
    conn: &mut SqliteConnection,
    shop_id: i32,
    at: NaiveDateTime,
) -> Result<Option<DatedThreshold>, CoreError> {
    repo::current_as_of(conn, shop_id, DISCOUNT_THRESHOLD_BPS, at)?
        .map(|(value, valid_from)| {
            Ok(DatedThreshold {
                threshold: parse_bps(&value)?,
                valid_from,
            })
        })
        .transpose()
}

/// A threshold change dated after `at` that has not taken effect yet, the
/// same shape as `regime_planned`.
pub fn discount_threshold_planned(
    conn: &mut SqliteConnection,
    shop_id: i32,
    at: NaiveDateTime,
) -> Result<Option<DatedThreshold>, CoreError> {
    repo::next_after(conn, shop_id, DISCOUNT_THRESHOLD_BPS, at)?
        .map(|(value, valid_from)| {
            Ok(DatedThreshold {
                threshold: parse_bps(&value)?,
                valid_from,
            })
        })
        .transpose()
}

/// The threshold a discount given at `at` is checked against. What
/// `permissions::discount_needs_permission` is called with. `Bps::ZERO` when
/// the shop has never set one, see `DISCOUNT_THRESHOLD_BPS`.
pub fn discount_threshold_as_of(
    conn: &mut SqliteConnection,
    shop_id: i32,
    at: NaiveDateTime,
) -> Result<Bps, CoreError> {
    match repo::value_as_of(conn, shop_id, DISCOUNT_THRESHOLD_BPS, at)? {
        Some(value) => parse_bps(&value),
        None => Ok(Bps::ZERO),
    }
}

fn parse_bps(value: &str) -> Result<Bps, CoreError> {
    let raw: u32 = value.parse().map_err(|_| {
        CoreError::validation(
            DISCOUNT_THRESHOLD_BPS,
            &format!("{value} is not a discount threshold"),
        )
    })?;
    Ok(Bps::new(raw)?)
}

/// Records that a discount above `threshold` needs the permission from
/// `valid_from`. Earlier rows stay: a sale rung up yesterday is judged
/// against yesterday's threshold, the same reason `set_regime` never updates
/// a row in place.
pub fn set_discount_threshold(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    threshold: Bps,
    valid_from: NaiveDateTime,
) -> Result<(), CoreError> {
    conn.transaction(|conn| {
        let before = repo::value_as_of(conn, shop_id, DISCOUNT_THRESHOLD_BPS, valid_from)?;
        // Same value, same day: nothing to record. See set_regime for why.
        if before.as_deref() == Some(threshold.as_u32().to_string().as_str()) {
            return Err(CoreError::validation(
                DISCOUNT_THRESHOLD_BPS,
                "the shop is already under that discount threshold on that day",
            ));
        }
        repo::append(
            conn,
            shop_id,
            DISCOUNT_THRESHOLD_BPS,
            &threshold.as_u32().to_string(),
            valid_from,
        )?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_SET_DISCOUNT_THRESHOLD,
                entity: DISCOUNT_THRESHOLD_BPS,
                entity_id: Some(shop_id),
                before: before.map(|value| {
                    serde_json::json!({ "discount_threshold_bps": value }).to_string()
                }),
                after: Some(
                    serde_json::json!({
                        "discount_threshold_bps": threshold.as_u32(),
                        "valid_from": valid_from.to_string(),
                    })
                    .to_string(),
                ),
            },
        )
    })
}
