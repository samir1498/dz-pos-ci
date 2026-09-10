//! How the app looks, per shop. The only preference so far is the theme.
//!
//! Not in `services::settings`: that module is the dated series a document
//! reads to know the régime it printed under, and nothing here is ever read
//! by a document. This one has no history and no audit row either, for the
//! same reason: `services::audit` records what somebody would have to answer
//! for, and a colour scheme is not that.

use chrono::NaiveDateTime;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::repos::preferences as repo;

/// The key the theme is stored under.
pub const THEME: &str = "theme";

/// The four themes the design package emits a block for. `Comptoir` is the
/// light one and the default the app falls back to when the machine says
/// nothing; the rest are chosen by hand.
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

/// The shop's chosen theme, or `None` for "follow the machine".
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
/// the app back on the machine's own light or dark preference.
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
    fn a_shop_that_has_never_chosen_follows_the_machine() {
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
    fn choosing_nothing_puts_the_shop_back_on_the_machine() {
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

    /// The stored spelling is the one the CSS attribute carries, so a value in
    /// a shop file and a value in a stylesheet are the same string.
    #[test]
    fn the_stored_name_is_the_data_theme_name() {
        assert_eq!(Theme::ObserveDark.as_str(), "observe-dark");
        assert_eq!(Theme::parse("observe-dark"), Some(Theme::ObserveDark));
        assert_eq!(Theme::parse("Observe-Dark"), None);
    }
}
