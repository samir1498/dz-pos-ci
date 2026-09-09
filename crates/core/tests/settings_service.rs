// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The dated settings series. A document reads the row that was current when
//! it was issued, so the régime fiscal it printed under stays readable after
//! the shop changes régime (features.md, Régime fiscal row).

use chrono::NaiveDate;
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::money::Regime;
use dzpos_core::services::settings;

const SHOP: i32 = 1;
/// The owner the first migration seeds. TODO(M4): the real user.
const OWNER: i32 = 1;

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
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, at(2026, 6, 1)).unwrap();

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
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, moment).unwrap();
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Reel, moment).unwrap();

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

#[test]
fn the_current_regime_carries_the_date_it_took_effect() {
    let (_dir, mut conn) = open_temp();
    let current = settings::regime_current(&mut conn, SHOP, at(2026, 3, 15)).unwrap();
    assert_eq!(current.regime, Regime::Reel);
    assert_eq!(current.valid_from, at(2026, 1, 1), "the seeded row's date");

    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, at(2026, 6, 1)).unwrap();
    let later = settings::regime_current(&mut conn, SHOP, at(2026, 9, 1)).unwrap();
    assert_eq!(
        (later.regime, later.valid_from),
        (Regime::Ifu, at(2026, 6, 1))
    );
}

#[test]
fn a_change_dated_in_the_future_is_planned_not_current() {
    let (_dir, mut conn) = open_temp();
    let today = at(2026, 9, 9);
    assert_eq!(
        settings::regime_planned(&mut conn, SHOP, today).unwrap(),
        None
    );

    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, at(2027, 1, 1)).unwrap();
    let current = settings::regime_current(&mut conn, SHOP, today).unwrap();
    assert_eq!(
        current.regime,
        Regime::Reel,
        "the future row does not apply yet"
    );
    let planned = settings::regime_planned(&mut conn, SHOP, today)
        .unwrap()
        .unwrap();
    assert_eq!(
        (planned.regime, planned.valid_from),
        (Regime::Ifu, at(2027, 1, 1))
    );
    // A document issued today still computes under réel (regime_as_of is
    // what documents read).
    assert_eq!(
        settings::regime_as_of(&mut conn, SHOP, today).unwrap(),
        Regime::Reel
    );
    // Once the date arrives the planned row is the current one and nothing
    // is planned any more.
    assert_eq!(
        settings::regime_current(&mut conn, SHOP, at(2027, 1, 1))
            .unwrap()
            .regime,
        Regime::Ifu
    );
    assert_eq!(
        settings::regime_planned(&mut conn, SHOP, at(2027, 1, 1)).unwrap(),
        None
    );
}

#[test]
fn two_planned_rows_report_the_nearest_date_and_its_latest_decision() {
    let (_dir, mut conn) = open_temp();
    let today = at(2026, 9, 9);
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, at(2027, 6, 1)).unwrap();
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, at(2027, 1, 1)).unwrap();
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Reel, at(2027, 1, 1)).unwrap();
    let planned = settings::regime_planned(&mut conn, SHOP, today)
        .unwrap()
        .unwrap();
    assert_eq!(
        (planned.regime, planned.valid_from),
        (Regime::Reel, at(2027, 1, 1)),
        "the nearest date, and on that date the later write"
    );
}

#[test]
fn the_dated_reads_are_scoped_by_shop() {
    let (_dir, mut conn) = open_temp();
    assert!(settings::regime_current(&mut conn, 2, at(2026, 3, 15)).is_err());
    assert_eq!(
        settings::regime_planned(&mut conn, 2, at(2026, 3, 15)).unwrap(),
        None
    );
}

#[test]
fn a_change_to_the_regime_already_in_force_on_that_day_is_refused() {
    // Seeded: reel from 2026-01-01. "Réel from today" would only move the
    // since date; the row is refused and the series is untouched.
    let (_dir, mut conn) = open_temp();
    let err =
        settings::set_regime(&mut conn, SHOP, OWNER, Regime::Reel, at(2026, 9, 9)).unwrap_err();
    assert!(matches!(&err, CoreError::Validation { field, .. } if field == "regime_fiscal"));
    let current = settings::regime_current(&mut conn, SHOP, at(2026, 9, 9)).unwrap();
    assert_eq!(
        current.valid_from,
        at(2026, 1, 1),
        "the since date did not move"
    );

    // Cancelling a planned change is a change: ifu planned for 2027, then
    // reel from the same day is what is in force there, so it is refused
    // only once the planned row is the one in force. Reel dated 2027-01-01
    // is compared with the planned ifu row, and lands.
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, at(2027, 1, 1)).unwrap();
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Reel, at(2027, 1, 1)).unwrap();
    assert_eq!(
        settings::regime_current(&mut conn, SHOP, at(2027, 1, 1))
            .unwrap()
            .regime,
        Regime::Reel
    );
    // Before the first row there is nothing to compare with.
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Reel, at(2025, 6, 1)).unwrap();
    assert_eq!(
        settings::regime_current(&mut conn, SHOP, at(2025, 7, 1))
            .unwrap()
            .valid_from,
        at(2025, 6, 1)
    );
}

#[test]
fn a_regime_change_leaves_an_audit_entry_naming_the_day_it_starts() {
    // The régime decides whether a document prints TVA at all, so a change
    // is a sensitive one (features.md §5) and the entry has to carry the
    // date it takes effect, not only the day of the click.
    use dzpos_core::services::audit;
    let (_dir, mut conn) = open_temp();
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, at(2026, 3, 1)).unwrap();

    let entries = audit::list(&mut conn, SHOP).unwrap();
    assert_eq!(entries.len(), 1, "the régime change was not logged");
    let entry = &entries[0];
    assert_eq!(entry.action, "set_regime");
    assert_eq!(entry.entity, "regime_fiscal");
    assert_eq!(entry.entity_id, Some(SHOP));
    assert_eq!(entry.user_id, OWNER);
    let before = entry.before.clone().expect("no before");
    let after = entry.after.clone().expect("no after");
    assert!(
        before.contains("reel"),
        "the old régime is missing: {before}"
    );
    assert!(after.contains("ifu"), "the new régime is missing: {after}");
    assert!(
        after.contains("2026-03-01"),
        "the day the change starts is missing: {after}"
    );
}
