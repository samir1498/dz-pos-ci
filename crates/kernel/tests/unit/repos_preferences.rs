#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::repos::testdb::{open, SHOP};

fn at(day: u32) -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 9, day)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap()
}

#[test]
fn a_key_never_set_reads_as_nothing() {
    let (_dir, mut conn) = open();
    assert_eq!(value(&mut conn, SHOP, "theme").unwrap(), None);
}

#[test]
fn writing_twice_leaves_one_row_holding_the_second() {
    let (_dir, mut conn) = open();
    put(&mut conn, SHOP, "theme", "registre", at(10)).unwrap();
    put(&mut conn, SHOP, "theme", "observe", at(11)).unwrap();
    assert_eq!(
        value(&mut conn, SHOP, "theme").unwrap(),
        Some("observe".to_string())
    );
    let rows: i64 = preferences::table
        .filter(preferences::shop_id.eq(SHOP))
        .count()
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(rows, 1, "the second write appended instead of replacing");
}

#[test]
fn clearing_takes_the_row_out_rather_than_blanking_it() {
    let (_dir, mut conn) = open();
    put(&mut conn, SHOP, "theme", "registre", at(10)).unwrap();
    clear(&mut conn, SHOP, "theme").unwrap();
    assert_eq!(value(&mut conn, SHOP, "theme").unwrap(), None);
    // A stored empty string would read as "set to nothing" and the caller
    // could not tell it from "never chosen".
    let rows: i64 = preferences::table
        .filter(preferences::shop_id.eq(SHOP))
        .count()
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(rows, 0);
}

#[test]
fn clearing_a_key_never_set_is_not_an_error() {
    let (_dir, mut conn) = open();
    clear(&mut conn, SHOP, "theme").unwrap();
}

#[test]
fn two_keys_do_not_write_over_each_other() {
    let (_dir, mut conn) = open();
    put(&mut conn, SHOP, "theme", "registre", at(10)).unwrap();
    put(&mut conn, SHOP, "density", "compact", at(10)).unwrap();
    assert_eq!(
        value(&mut conn, SHOP, "theme").unwrap(),
        Some("registre".to_string())
    );
    assert_eq!(
        value(&mut conn, SHOP, "density").unwrap(),
        Some("compact".to_string())
    );
}
