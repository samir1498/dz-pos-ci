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
