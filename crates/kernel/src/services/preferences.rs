//! What a shop has set that is written over in place rather than dated: the
//! theme it opens on, the facture layout it prints, the language every
//! fiscal paper prints in, and the idle time a session survives.
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
use crate::lang::Lang;
use crate::print::{FactureLayout, ThermalMode};
use crate::repos::preferences as repo;
use crate::services::audit;

/// The key the theme is stored under.
pub const THEME: &str = "theme";

/// The key the chosen facture layout is stored under.
pub const FACTURE_LAYOUT: &str = "facture_layout";

/// The key the shop's chosen print language is stored under.
pub const PRINT_LANG: &str = "print_lang";

/// The key the shop's chosen thermal path is stored under.
pub const THERMAL_MODE: &str = "thermal_mode";

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

/// The layout the shop's factures print in.
///
/// Not an `Option` the way the theme is. A shop that has never chosen still
/// prints factures, and the layout it prints in is `Standard`, so the answer
/// to "which layout" is always a layout. A stored name this build cannot read
/// falls back the same way: printing on the default beats refusing to print.
pub fn facture_layout(
    conn: &mut SqliteConnection,
    shop_id: i32,
) -> Result<FactureLayout, CoreError> {
    Ok(repo::value(conn, shop_id, FACTURE_LAYOUT)?
        .as_deref()
        .and_then(FactureLayout::parse)
        .unwrap_or_default())
}

/// Records the layout the shop prints factures in.
///
/// No audit row, for the reason the theme has none: `services::audit` records
/// what somebody would have to answer for, and which of the shop's own
/// layouts a facture is drawn in is not that. What the facture says is
/// audited; the sheet it is drawn on is a preference.
pub fn set_facture_layout(
    conn: &mut SqliteConnection,
    shop_id: i32,
    layout: FactureLayout,
    at: NaiveDateTime,
) -> Result<(), CoreError> {
    repo::put(conn, shop_id, FACTURE_LAYOUT, layout.as_str(), at)
}

/// The language every fiscal paper prints in, or `None` when the shop has
/// never chosen one. `None` does not mean French: it means what it always
/// has, that the till prints in whatever language it is being used in at the
/// moment somebody presses print (`docs/features.md` §4, the ruling in
/// `context/plans/20260920-a-print-language-the-shop-keeps.md`).
///
/// A stored value that is not `fr`, `en` or `ar` reads as no preference
/// rather than as an error, the way an unknown theme does: a row written by
/// a future version, or by hand, degrades to today's behaviour instead of
/// refusing to print.
pub fn print_lang(conn: &mut SqliteConnection, shop_id: i32) -> Result<Option<Lang>, CoreError> {
    Ok(repo::value(conn, shop_id, PRINT_LANG)?
        .as_deref()
        .and_then(Lang::parse))
}

/// Records the shop's print language, or forgets it when `lang` is `None`,
/// which puts every fiscal paper back on the till's own language. The `None`
/// arm exists for the same reason the theme's does: a shop that chose
/// Arabic needs a way back to "follow the till" without a second field.
///
/// No audit row, the same call `set_facture_layout` above makes and for a
/// reason that reaches a little further: the three languages are three
/// spellings of the same fixed strings in `print::strings`, and the number,
/// the date and every total on the paper are identical in all three, which
/// the per-language goldens are what prove. What the facture says does not
/// change, so there is nothing here somebody would have to answer for.
///
/// The argument on the other side has not been put to Samir and is written
/// down rather than decided: a shop that switches part way through a year
/// leaves an archive in two languages, and an inspector asking why would
/// find nothing in the log saying when or who. If that is worth a row, it is
/// worth one on the layout beside it too, and both change together.
pub fn set_print_lang(
    conn: &mut SqliteConnection,
    shop_id: i32,
    lang: Option<Lang>,
    at: NaiveDateTime,
) -> Result<(), CoreError> {
    match lang {
        Some(lang) => repo::put(conn, shop_id, PRINT_LANG, lang.tag(), at),
        None => repo::clear(conn, shop_id, PRINT_LANG),
    }
}

/// Which language a fiscal paper prints in, three steps deep
/// (`context/plans/20260920-a-print-language-the-shop-keeps.md`):
///
/// 1. `named`, a `?print_lang=` on the one call, when the caller sent one.
///    A preview of another language wins over everything else, the same
///    shape `?layout=` already takes over `facture_layout` for one page.
/// 2. The shop's own `print_lang`, when it has ever chosen one.
/// 3. `caller`, the language the till (or the phone, or the server's own
///    caller) is being used in right now, `?lang=` on the same call.
///
/// No step defaults to French. A fresh shop running an Arabic till that
/// never opens the settings panel has to keep printing Arabic paper, or
/// nobody would ever connect the French ticket in their hand to a setting
/// they have not found yet.
///
/// Step 1 carries no permission of its own, which is deliberate and worth
/// saying because the gate row on `PUT /settings/print-lang` says a cashier
/// does not choose the language. That row is about the stored choice, which
/// outlives the call and every paper after it. One call naming its own
/// language decides one sheet, the way `?layout=` decides one facture's
/// layout over the stored one. It also takes reach away rather than adding
/// it: before this resolver every caller named the print language outright
/// on all six routes and no step consulted the shop at all.
pub fn print_lang_for(
    conn: &mut SqliteConnection,
    shop_id: i32,
    named: Option<Lang>,
    caller: Lang,
) -> Result<Lang, CoreError> {
    if let Some(named) = named {
        return Ok(named);
    }
    Ok(print_lang(conn, shop_id)?.unwrap_or(caller))
}

/// Which ESC/POS path the shop's thermal head is sent.
///
/// Not an `Option`, the way the facture layout is not: a shop that has never
/// chosen still prints thermal tickets, and the path it prints them down is
/// `Text`. A stored name this build cannot read falls back the same way,
/// because printing on the default beats refusing to print.
///
/// This is the shop's answer and not the final one. Arabic has no
/// single-byte table on a cheap head, so it is drawn whatever is stored
/// here; `ThermalMode::for_lang` is the one place that decides it and every
/// caller goes through `thermal_mode_for` below.
pub fn thermal_mode(conn: &mut SqliteConnection, shop_id: i32) -> Result<ThermalMode, CoreError> {
    Ok(repo::value(conn, shop_id, THERMAL_MODE)?
        .as_deref()
        .and_then(ThermalMode::parse)
        .unwrap_or_default())
}

/// Records the path the shop's thermal head is sent.
///
/// No audit row, for the reason `set_facture_layout` has none and with the
/// same reach: the two paths are pinned to carry the same strings line for
/// line (`the_raster_draws_the_lines_the_text_path_prints`), so the paper
/// says the same words and the same totals either way. What changes is how
/// the head is addressed, not what a customer is handed.
pub fn set_thermal_mode(
    conn: &mut SqliteConnection,
    shop_id: i32,
    mode: ThermalMode,
    at: NaiveDateTime,
) -> Result<(), CoreError> {
    repo::put(conn, shop_id, THERMAL_MODE, mode.as_str(), at)
}

/// The path a document in `lang` is actually sent down: the shop's stored
/// answer, put through the rule that Arabic has no text path at all.
///
/// One resolver, the shape `print_lang_for` above has, and for the same
/// reason: the ESC/POS route, the spool writer and the TCP sender all have
/// to agree about one document, and a handler working it out for itself is
/// how a spool file and a wire end up carrying different bytes.
pub fn thermal_mode_for(
    conn: &mut SqliteConnection,
    shop_id: i32,
    lang: Lang,
) -> Result<ThermalMode, CoreError> {
    Ok(thermal_mode(conn, shop_id)?.for_lang(lang))
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
    fn a_shop_that_has_never_chosen_prints_on_the_standard_layout() {
        let (_dir, mut conn) = open();
        assert_eq!(
            facture_layout(&mut conn, SHOP).unwrap(),
            FactureLayout::Standard
        );
    }

    #[test]
    fn every_facture_layout_survives_a_round_trip() {
        let (_dir, mut conn) = open();
        for chosen in FactureLayout::ALL {
            set_facture_layout(&mut conn, SHOP, chosen, at()).unwrap();
            assert_eq!(facture_layout(&mut conn, SHOP).unwrap(), chosen);
        }
    }

    /// A build that has seen a layout a later one wrote still prints. The
    /// row is left alone rather than repaired: the newer build is the one
    /// that knows what the name means, and clearing it here would lose the
    /// shop's choice the moment an older build opened the file once.
    #[test]
    fn a_layout_this_build_never_heard_of_prints_on_the_standard_one() {
        let (_dir, mut conn) = open();
        repo::put(&mut conn, SHOP, FACTURE_LAYOUT, "hologram", at()).unwrap();
        assert_eq!(
            facture_layout(&mut conn, SHOP).unwrap(),
            FactureLayout::Standard
        );
        assert_eq!(
            repo::value(&mut conn, SHOP, FACTURE_LAYOUT)
                .unwrap()
                .as_deref(),
            Some("hologram")
        );
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

    #[test]
    fn a_shop_that_has_never_chosen_has_no_stored_print_lang() {
        let (_dir, mut conn) = open();
        assert_eq!(print_lang(&mut conn, SHOP).unwrap(), None);
    }

    #[test]
    fn every_print_lang_survives_a_round_trip() {
        let (_dir, mut conn) = open();
        for chosen in Lang::ALL {
            set_print_lang(&mut conn, SHOP, Some(chosen), at()).unwrap();
            assert_eq!(print_lang(&mut conn, SHOP).unwrap(), Some(chosen));
        }
    }

    #[test]
    fn choosing_no_print_lang_puts_the_shop_back_on_the_till() {
        let (_dir, mut conn) = open();
        set_print_lang(&mut conn, SHOP, Some(Lang::Ar), at()).unwrap();
        set_print_lang(&mut conn, SHOP, None, at()).unwrap();
        assert_eq!(print_lang(&mut conn, SHOP).unwrap(), None);
    }

    /// A row a newer build, or a hand, wrote: the shop prints in the caller's
    /// language rather than the route refusing, the way an unknown theme
    /// reads as no choice.
    #[test]
    fn a_print_lang_this_build_does_not_know_reads_as_no_preference() {
        let (_dir, mut conn) = open();
        repo::put(&mut conn, SHOP, PRINT_LANG, "de", at()).unwrap();
        assert_eq!(print_lang(&mut conn, SHOP).unwrap(), None);
        // The row is left alone rather than repaired, the same as an unknown
        // facture layout: a newer build wrote it and knows what it means.
        assert_eq!(
            repo::value(&mut conn, SHOP, PRINT_LANG).unwrap().as_deref(),
            Some("de")
        );
    }

    #[test]
    fn a_shop_that_has_never_chosen_prints_thermal_as_text() {
        let (_dir, mut conn) = open();
        assert_eq!(thermal_mode(&mut conn, SHOP).unwrap(), ThermalMode::Text);
    }

    #[test]
    fn every_thermal_mode_survives_a_round_trip() {
        let (_dir, mut conn) = open();
        for chosen in ThermalMode::ALL {
            set_thermal_mode(&mut conn, SHOP, chosen, at()).unwrap();
            assert_eq!(thermal_mode(&mut conn, SHOP).unwrap(), chosen);
        }
    }

    /// A row a newer build, or a hand, wrote: the head is sent the default
    /// path rather than the route refusing, the way an unknown facture
    /// layout prints on the standard one. The row is left alone, because
    /// the build that wrote it knows what it means.
    #[test]
    fn a_thermal_mode_this_build_never_heard_of_prints_as_text() {
        let (_dir, mut conn) = open();
        repo::put(&mut conn, SHOP, THERMAL_MODE, "holograph", at()).unwrap();
        assert_eq!(thermal_mode(&mut conn, SHOP).unwrap(), ThermalMode::Text);
        assert_eq!(
            repo::value(&mut conn, SHOP, THERMAL_MODE)
                .unwrap()
                .as_deref(),
            Some("holograph")
        );
    }

    /// The resolver, against the shop file rather than against the enum:
    /// Arabic is drawn under both stored answers, and the other two follow
    /// what the shop stored.
    #[test]
    fn arabic_resolves_to_a_raster_whatever_the_shop_stored() {
        let (_dir, mut conn) = open();
        for stored in ThermalMode::ALL {
            set_thermal_mode(&mut conn, SHOP, stored, at()).unwrap();
            assert_eq!(
                thermal_mode_for(&mut conn, SHOP, Lang::Ar).unwrap(),
                ThermalMode::Raster,
                "{stored:?}"
            );
            for lang in [Lang::Fr, Lang::En] {
                assert_eq!(
                    thermal_mode_for(&mut conn, SHOP, lang).unwrap(),
                    stored,
                    "{stored:?} {lang:?}"
                );
            }
        }
    }

    /// The setting outlives the connection that wrote it: a fresh handle on
    /// the same file, standing in for the process restarting, still reads it
    /// back.
    #[test]
    fn a_print_lang_survives_reopening_the_shop_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        {
            let mut conn = crate::db::open(&path).unwrap();
            set_print_lang(&mut conn, SHOP, Some(Lang::Ar), at()).unwrap();
        }
        let mut reopened = crate::db::open(&path).unwrap();
        assert_eq!(print_lang(&mut reopened, SHOP).unwrap(), Some(Lang::Ar));
    }
}
