// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The discount-threshold setting: the dated series T1 put beside the
//! régime fiscal's (`crates/core/tests/settings_service.rs`), moved with
//! the setting itself out of `dzpos_kernel::services::settings` into
//! `dzpos_retail::services::discount_threshold` by S4 of
//! `a-kernel-crate-and-retail-as-the-first-module`.

use chrono::NaiveDate;
use dzpos_retail::error::CoreError;
use dzpos_retail::money::Bps;
use dzpos_retail::services::discount_threshold;

const SHOP: i32 = 1;
/// The owner the first migration seeds. A service takes whoever acted as an
/// argument; the API reads that from the session, and a test says it here.
const OWNER: i32 = 1;

mod common;

use common::open_temp;

fn at(y: i32, m: u32, d: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(y, m, d)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

#[test]
fn a_shop_that_never_set_one_reads_zero() {
    // Unlike the régime, no migration seeds a first row.
    let (_dir, mut conn) = open_temp();
    assert_eq!(
        discount_threshold::discount_threshold_as_of(&mut conn, SHOP, at(2026, 3, 15)).unwrap(),
        Bps::ZERO
    );
    assert_eq!(
        discount_threshold::discount_threshold_current(&mut conn, SHOP, at(2026, 3, 15)).unwrap(),
        None
    );
    assert_eq!(
        discount_threshold::discount_threshold_planned(&mut conn, SHOP, at(2026, 3, 15)).unwrap(),
        None
    );
}

#[test]
fn a_date_between_two_threshold_rows_reads_the_earlier_one() {
    let (_dir, mut conn) = open_temp();
    discount_threshold::set_discount_threshold(
        &mut conn,
        SHOP,
        OWNER,
        Bps::new(500).unwrap(),
        at(2026, 1, 1),
    )
    .unwrap();
    discount_threshold::set_discount_threshold(
        &mut conn,
        SHOP,
        OWNER,
        Bps::new(1000).unwrap(),
        at(2026, 6, 1),
    )
    .unwrap();

    assert_eq!(
        discount_threshold::discount_threshold_as_of(&mut conn, SHOP, at(2026, 3, 15)).unwrap(),
        Bps::new(500).unwrap(),
        "a March sale is judged against the March threshold"
    );
    assert_eq!(
        discount_threshold::discount_threshold_as_of(&mut conn, SHOP, at(2026, 9, 1)).unwrap(),
        Bps::new(1000).unwrap()
    );
}

#[test]
fn the_current_threshold_carries_the_date_it_took_effect() {
    let (_dir, mut conn) = open_temp();
    discount_threshold::set_discount_threshold(
        &mut conn,
        SHOP,
        OWNER,
        Bps::new(500).unwrap(),
        at(2026, 1, 1),
    )
    .unwrap();
    let current = discount_threshold::discount_threshold_current(&mut conn, SHOP, at(2026, 3, 15))
        .unwrap()
        .unwrap();
    assert_eq!(current.threshold, Bps::new(500).unwrap());
    assert_eq!(current.valid_from, at(2026, 1, 1));
}

#[test]
fn a_threshold_change_dated_in_the_future_is_planned_not_current() {
    let (_dir, mut conn) = open_temp();
    let today = at(2026, 9, 9);
    discount_threshold::set_discount_threshold(
        &mut conn,
        SHOP,
        OWNER,
        Bps::new(1000).unwrap(),
        at(2027, 1, 1),
    )
    .unwrap();

    assert_eq!(
        discount_threshold::discount_threshold_as_of(&mut conn, SHOP, today).unwrap(),
        Bps::ZERO,
        "the future row does not apply yet, and nothing else was ever set"
    );
    let planned = discount_threshold::discount_threshold_planned(&mut conn, SHOP, today)
        .unwrap()
        .unwrap();
    assert_eq!(
        (planned.threshold, planned.valid_from),
        (Bps::new(1000).unwrap(), at(2027, 1, 1))
    );
}

#[test]
fn the_threshold_series_is_scoped_by_shop() {
    let (_dir, mut conn) = open_temp();
    discount_threshold::set_discount_threshold(
        &mut conn,
        SHOP,
        OWNER,
        Bps::new(500).unwrap(),
        at(2026, 1, 1),
    )
    .unwrap();
    assert_eq!(
        discount_threshold::discount_threshold_as_of(&mut conn, 2, at(2026, 3, 15)).unwrap(),
        Bps::ZERO,
        "shop 2 never reads shop 1's threshold"
    );
}

#[test]
fn a_change_to_the_threshold_already_in_force_on_that_day_is_refused() {
    let (_dir, mut conn) = open_temp();
    discount_threshold::set_discount_threshold(
        &mut conn,
        SHOP,
        OWNER,
        Bps::new(500).unwrap(),
        at(2026, 1, 1),
    )
    .unwrap();
    let err = discount_threshold::set_discount_threshold(
        &mut conn,
        SHOP,
        OWNER,
        Bps::new(500).unwrap(),
        at(2026, 9, 9),
    )
    .unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "discount_threshold_bps")
    );
}

#[test]
fn a_threshold_change_leaves_an_audit_entry_naming_the_day_it_starts() {
    use dzpos_kernel::services::audit;
    let (_dir, mut conn) = open_temp();
    discount_threshold::set_discount_threshold(
        &mut conn,
        SHOP,
        OWNER,
        Bps::new(500).unwrap(),
        at(2026, 3, 1),
    )
    .unwrap();

    let entries = audit::list(&mut conn, SHOP).unwrap();
    assert_eq!(entries.len(), 1, "the threshold change was not logged");
    let entry = &entries[0];
    assert_eq!(entry.action, "set_discount_threshold");
    assert_eq!(entry.entity, "discount_threshold_bps");
    assert_eq!(entry.user_id, OWNER);
    let after = entry.after.clone().expect("no after");
    assert!(
        after.contains("500"),
        "the new threshold is missing: {after}"
    );
    assert!(
        after.contains("2026-03-01"),
        "the day the change starts is missing: {after}"
    );
}
