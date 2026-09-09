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
    regime: Regime,
    valid_from: NaiveDateTime,
) -> Result<(), CoreError> {
    // The shop is already under that régime on that day: nothing to
    // record. Accepting it would move the "since" date a comptable reads
    // to the day of the click. Before the first row there is no régime to
    // compare with, so a first entry is always accepted.
    if repo::value_as_of(conn, shop_id, REGIME_FISCAL, valid_from)?.as_deref()
        == Some(stored(regime))
    {
        return Err(CoreError::validation(
            REGIME_FISCAL,
            "the shop is already under that régime on that day",
        ));
    }
    repo::append(conn, shop_id, REGIME_FISCAL, stored(regime), valid_from)
}

const fn stored(regime: Regime) -> &'static str {
    match regime {
        Regime::Reel => "reel",
        Regime::Ifu => "ifu",
    }
}
