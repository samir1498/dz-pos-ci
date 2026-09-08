//! Shop settings that carry a date. A document reads the row that was
//! current when it was issued, so the régime fiscal it printed under stays
//! readable after the shop changes régime (features.md, Régime fiscal row).

use chrono::NaiveDateTime;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::money::Regime;
use crate::repos::settings as repo;

/// The key holding the shop's régime fiscal. `ifu` or `reel`; the migration
/// carries the same CHECK.
pub const REGIME_FISCAL: &str = "regime_fiscal";

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
    match value.as_str() {
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
    regime: Regime,
    valid_from: NaiveDateTime,
) -> Result<(), CoreError> {
    repo::append(conn, shop_id, REGIME_FISCAL, stored(regime), valid_from)
}

const fn stored(regime: Regime) -> &'static str {
    match regime {
        Regime::Reel => "reel",
        Regime::Ifu => "ifu",
    }
}
