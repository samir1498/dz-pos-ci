// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The dated settings series. A document reads the row that was current when
//! it was issued, so the régime fiscal it printed under stays readable after
//! the shop changes régime (features.md, Régime fiscal row).

use chrono::NaiveDate;
use diesel::sqlite::SqliteConnection;
use dzpos_core::money::Regime;
use dzpos_core::services::settings;

const SHOP: i32 = 1;

fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let conn = dzpos_core::db::open(&path).unwrap();
    (dir, conn)
}

fn at(y: i32, m: u32, d: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(y, m, d)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

#[test]
fn the_seeded_shop_reads_as_reel() {
    let (_dir, mut conn) = open_temp();
    assert_eq!(
        settings::regime_as_of(&mut conn, SHOP, at(2026, 3, 15)).unwrap(),
        Regime::Reel
    );
}

#[test]
fn a_date_between_two_rows_reads_the_earlier_one() {
    // Seeded: reel from 2026-01-01. Then ifu from 2026-06-01.
    let (_dir, mut conn) = open_temp();
    settings::set_regime(&mut conn, SHOP, Regime::Ifu, at(2026, 6, 1)).unwrap();

    assert_eq!(
        settings::regime_as_of(&mut conn, SHOP, at(2026, 3, 15)).unwrap(),
        Regime::Reel,
        "a March document must still print under the régime of March"
    );
    assert_eq!(
        settings::regime_as_of(&mut conn, SHOP, at(2026, 9, 1)).unwrap(),
        Regime::Ifu
    );
    assert_eq!(
        settings::regime_as_of(&mut conn, SHOP, at(2026, 6, 1)).unwrap(),
        Regime::Ifu,
        "a row is current from its own valid_from"
    );
}

#[test]
fn a_date_before_every_row_has_no_regime_to_read() {
    let (_dir, mut conn) = open_temp();
    assert!(
        settings::regime_as_of(&mut conn, SHOP, at(2025, 12, 31)).is_err(),
        "a date before the first row cannot invent a régime"
    );
}

#[test]
fn two_changes_in_the_same_second_both_land_and_the_later_one_wins() {
    // valid_from is whole seconds. Keyed on (shop_id, key, valid_from) the
    // second write collided with the first and was lost.
    let (_dir, mut conn) = open_temp();
    let moment = at(2026, 6, 1);
    settings::set_regime(&mut conn, SHOP, Regime::Ifu, moment).unwrap();
    settings::set_regime(&mut conn, SHOP, Regime::Reel, moment).unwrap();

    assert_eq!(
        settings::regime_as_of(&mut conn, SHOP, at(2026, 7, 1)).unwrap(),
        Regime::Reel,
        "the second write in the same second must win"
    );
}

#[test]
fn the_series_is_scoped_by_shop() {
    // Rule 3: shop 2 never reads shop 1's régime.
    let (_dir, mut conn) = open_temp();
    assert!(settings::regime_as_of(&mut conn, 2, at(2026, 3, 15)).is_err());
}
