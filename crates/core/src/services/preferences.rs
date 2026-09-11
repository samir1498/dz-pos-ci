//! What a shop has set that is written over in place rather than dated: the
//! theme it opens on, and the idle time a session survives.
//!
//! Not in `services::settings`: that module is the dated series a document
//! reads to know the régime it printed under, and nothing here is ever read
//! by a document. A preference is one row per (shop, key), rewritten; a
//! setting is a row per change, with the day it applies from, kept forever.
//! The question that sorts a value into one or the other is whether anybody
//! ever reads its history. Nobody reads back what the idle time was in March,
//! and a dated series would append a row every time an owner nudged the
//! figure; the régime fiscal is the opposite of that on both counts.
//!
//! The theme has no audit row either, because `services::audit` records what
//! somebody would have to answer for and a colour scheme is not that. The
//! idle time is not in that class and `set_session_idle` writes one.

use chrono::{Duration, NaiveDateTime};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::repos::preferences as repo;
use crate::services::audit;

/// The key the theme is stored under.
pub const THEME: &str = "theme";

/// The key the session idle time is stored under, in whole minutes.
pub const SESSION_IDLE_MINUTES: &str = "session_idle_minutes";

/// How long a session survives with nothing happening on it, for a shop that
/// has never set the figure. Fifteen minutes: a till in a shop with the door
/// open is the thing being protected, and a cashier who has served nobody for
/// a quarter of an hour typing four digits again is the whole cost of it.
/// The alternative a shop will ask for is longer, not shorter, which is why
/// the default is at the short end and the setting exists.
pub const DEFAULT_SESSION_IDLE_MINUTES: i64 = 15;
/// The shortest a shop may set it to. Under a minute the till would lock
/// between a customer's two items.
pub const MIN_SESSION_IDLE_MINUTES: i64 = 1;
/// The longest. Twelve hours is a whole opening day, and past it the setting
/// stops meaning "idle" and starts meaning "never".
pub const MAX_SESSION_IDLE_MINUTES: i64 = 720;

/// The four themes the design package emits a block for. `Comptoir` is the
/// default, the one a shop that has never chosen opens on; the rest are
/// chosen by hand.
///
/// A fifth theme is a stylesheet and one arm here. The table carries no CHECK
/// on the value on purpose, so adding one is not a migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Comptoir,
    Registre,
    Observe,
    ObserveDark,
}

impl Theme {
    /// The name the CSS `[data-theme]` attribute carries, which is also what
    /// is stored. One spelling, so a value written by an older build reads
    /// back the same.
    pub const fn as_str(self) -> &'static str {
        match self {
            Theme::Comptoir => "comptoir",
            Theme::Registre => "registre",
            Theme::Observe => "observe",
            Theme::ObserveDark => "observe-dark",
        }
    }

    pub fn parse(value: &str) -> Option<Theme> {
        match value {
            "comptoir" => Some(Theme::Comptoir),
            "registre" => Some(Theme::Registre),
            "observe" => Some(Theme::Observe),
            "observe-dark" => Some(Theme::ObserveDark),
            _ => None,
        }
    }
}

/// The shop's chosen theme, or `None` when it has never chosen (the app then
/// opens on Comptoir).
///
/// A stored value the code does not know reads as `None` rather than as an
/// error. The alternative is a shop that cannot open its own settings screen
/// because a newer build once wrote a theme this one has never heard of, and
/// the cost of being wrong is one screen in the wrong colours.
pub fn theme(conn: &mut SqliteConnection, shop_id: i32) -> Result<Option<Theme>, CoreError> {
    Ok(repo::value(conn, shop_id, THEME)?
        .as_deref()
        .and_then(Theme::parse))
}

/// Records the shop's theme, or forgets it when `theme` is `None`, which puts
/// the app back on the default.
pub fn set_theme(
    conn: &mut SqliteConnection,
    shop_id: i32,
    theme: Option<Theme>,
    at: NaiveDateTime,
) -> Result<(), CoreError> {
    match theme {
        Some(theme) => repo::put(conn, shop_id, THEME, theme.as_str(), at),
        None => repo::clear(conn, shop_id, THEME),
    }
}

/// How long a session survives with nothing happening on it.
///
/// A stored figure this build cannot read, or one outside the bounds, reads
/// as the default rather than as an error, the way an unknown theme reads as
/// no choice: the cost of being wrong is a session that lives fifteen minutes
/// instead of twenty, and the alternative is a shop that cannot sign in
/// because somebody once wrote a bad row.
pub fn session_idle(conn: &mut SqliteConnection, shop_id: i32) -> Result<Duration, CoreError> {
    let minutes = repo::value(conn, shop_id, SESSION_IDLE_MINUTES)?
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|m| (MIN_SESSION_IDLE_MINUTES..=MAX_SESSION_IDLE_MINUTES).contains(m))
        .unwrap_or(DEFAULT_SESSION_IDLE_MINUTES);
    Ok(Duration::minutes(minutes))
}

/// Sets the idle time, in whole minutes, and writes the audit row: how long
/// the till stays open unattended is a control somebody answers for, unlike
/// the theme beside it.
pub fn set_session_idle(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    minutes: i64,
    at: NaiveDateTime,
) -> Result<(), CoreError> {
    if !(MIN_SESSION_IDLE_MINUTES..=MAX_SESSION_IDLE_MINUTES).contains(&minutes) {
        return Err(CoreError::validation(
            "session_idle_minutes",
            "a session's idle time is between one minute and twelve hours",
        ));
    }
    let before = session_idle(conn, shop_id)?.num_minutes();
    repo::put(
        conn,
        shop_id,
        SESSION_IDLE_MINUTES,
        &minutes.to_string(),
        at,
    )?;
    audit::record(
        conn,
        shop_id,
        actor_id,
        audit::Change {
            action: audit::ACTION_SET_SESSION_IDLE,
            entity: "preference",
            entity_id: None,
            before: Some(serde_json::json!({ "session_idle_minutes": before }).to_string()),
            after: Some(serde_json::json!({ "session_idle_minutes": minutes }).to_string()),
        },
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::repos::preferences as repo;
    use crate::repos::testdb::{open, SHOP};

    fn at() -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 9, 10)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap()
    }

    #[test]
    fn a_shop_that_has_never_chosen_has_no_stored_theme() {
        let (_dir, mut conn) = open();
        assert_eq!(theme(&mut conn, SHOP).unwrap(), None);
    }

    #[test]
    fn every_theme_survives_a_round_trip() {
        let (_dir, mut conn) = open();
        for chosen in [
            Theme::Comptoir,
            Theme::Registre,
            Theme::Observe,
            Theme::ObserveDark,
        ] {
            set_theme(&mut conn, SHOP, Some(chosen), at()).unwrap();
            assert_eq!(theme(&mut conn, SHOP).unwrap(), Some(chosen));
        }
    }

    #[test]
    fn choosing_nothing_puts_the_shop_back_on_the_default() {
        let (_dir, mut conn) = open();
        set_theme(&mut conn, SHOP, Some(Theme::Registre), at()).unwrap();
        set_theme(&mut conn, SHOP, None, at()).unwrap();
        assert_eq!(theme(&mut conn, SHOP).unwrap(), None);
    }

    /// A file written by a newer build, opened by this one.
    #[test]
    fn a_theme_this_build_does_not_know_reads_as_no_choice() {
        let (_dir, mut conn) = open();
        repo::put(&mut conn, SHOP, THEME, "midnight", at()).unwrap();
        assert_eq!(theme(&mut conn, SHOP).unwrap(), None);
    }

    /// The figure a shop that has never set one runs on, read off the
    /// service rather than restated.
    #[test]
    fn a_shop_that_has_never_set_an_idle_time_gets_the_default() {
        let (_dir, mut conn) = open();
        assert_eq!(
            session_idle(&mut conn, SHOP).unwrap(),
            Duration::minutes(DEFAULT_SESSION_IDLE_MINUTES)
        );
    }

    #[test]
    fn an_idle_time_the_owner_sets_survives_a_round_trip() {
        let (_dir, mut conn) = open();
        set_session_idle(&mut conn, SHOP, 1, 45, at()).unwrap();
        assert_eq!(
            session_idle(&mut conn, SHOP).unwrap(),
            Duration::minutes(45)
        );
    }

    /// Both ends, and the row is not written when the figure is refused.
    #[test]
    fn an_idle_time_outside_the_bounds_is_refused_and_nothing_is_stored() {
        let (_dir, mut conn) = open();
        for bad in [0, -5, MAX_SESSION_IDLE_MINUTES + 1] {
            assert!(
                set_session_idle(&mut conn, SHOP, 1, bad, at()).is_err(),
                "{bad}"
            );
        }
        assert_eq!(
            repo::value(&mut conn, SHOP, SESSION_IDLE_MINUTES).unwrap(),
            None
        );
        for ok in [MIN_SESSION_IDLE_MINUTES, MAX_SESSION_IDLE_MINUTES] {
            set_session_idle(&mut conn, SHOP, 1, ok, at()).unwrap();
        }
    }

    /// A row a newer build, or a hand, wrote: the session lives the default
    /// rather than the sign-in screen 500ing.
    #[test]
    fn a_stored_idle_time_this_build_cannot_read_falls_back_to_the_default() {
        let (_dir, mut conn) = open();
        for bad in ["soon", "", "0", "99999", "-3"] {
            repo::put(&mut conn, SHOP, SESSION_IDLE_MINUTES, bad, at()).unwrap();
            assert_eq!(
                session_idle(&mut conn, SHOP).unwrap(),
                Duration::minutes(DEFAULT_SESSION_IDLE_MINUTES),
                "{bad}"
            );
        }
    }

    /// The stored spelling is the one the CSS attribute carries, so a value in
    /// a shop file and a value in a stylesheet are the same string.
    #[test]
    fn the_stored_name_is_the_data_theme_name() {
        assert_eq!(Theme::ObserveDark.as_str(), "observe-dark");
        assert_eq!(Theme::parse("observe-dark"), Some(Theme::ObserveDark));
        assert_eq!(Theme::parse("Observe-Dark"), None);
    }
}
