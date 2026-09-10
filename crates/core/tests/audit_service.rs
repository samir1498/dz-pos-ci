// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The audit log of sensitive actions (features.md §5). Each call site is
//! asserted on the stored entry, and the entries that must not be written are
//! asserted too: a log that records everything hides the one that matters.

use chrono::NaiveDate;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::models::shop::StoreBlock;
use dzpos_core::money::{Bps, Money, Regime};
use dzpos_core::services::{audit, products, settings, shops};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

mod common;

use common::open_temp;

fn block(name: &str) -> StoreBlock {
    StoreBlock {
        name: name.to_string(),
        rc: Some("16/00-1234567 B 21".to_string()),
        nif: None,
        nis: None,
        ai: None,
        address: None,
        phone: None,
    }
}

fn draft(selling: i64, active: bool) -> NewProduct {
    NewProduct {
        name: "Sucre".to_string(),
        barcode: None,
        category_id: None,
        unit: Unit::Piece,
        cost: Money::centimes(500),
        selling: Money::centimes(selling),
        wholesale: None,
        qty_on_hand_milli: 0,
        low_stock_at_milli: 0,
        rate_bps: Some(Bps::new(1900).unwrap()),
        active,
    }
}

#[test]
fn a_new_database_has_an_empty_log() {
    let (_dir, mut conn) = open_temp();
    assert!(audit::list(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn writing_the_store_block_records_what_it_replaced() {
    let (_dir, mut conn) = open_temp();
    shops::update_store(&mut conn, SHOP, OWNER, block("Supérette El Bahdja")).unwrap();
    let log = audit::list(&mut conn, SHOP).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].action, "update");
    assert_eq!(log[0].entity, "shop");
    assert_eq!(log[0].entity_id, Some(SHOP));
    assert_eq!(log[0].user_id, OWNER);
    let before = log[0].before.as_deref().unwrap();
    let after = log[0].after.as_deref().unwrap();
    assert!(before.contains("Mon magasin"), "{before}");
    assert!(after.contains("Supérette El Bahdja"), "{after}");
    assert!(after.contains("16/00-1234567 B 21"), "{after}");
}

#[test]
fn a_refused_store_block_records_nothing() {
    // The entry and the change are one transaction. A log of attempts that
    // never happened is a log nobody trusts.
    let (_dir, mut conn) = open_temp();
    let mut bad = block("Supérette");
    bad.name = "   ".to_string();
    assert!(shops::update_store(&mut conn, SHOP, OWNER, bad).is_err());
    assert!(audit::list(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn a_regime_change_records_the_one_it_left_and_the_day_it_takes() {
    let (_dir, mut conn) = open_temp();
    let day = NaiveDate::from_ymd_opt(2027, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, day).unwrap();
    let log = audit::list(&mut conn, SHOP).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].action, "set_regime");
    assert_eq!(log[0].entity, "regime_fiscal");
    assert!(log[0].before.as_deref().unwrap().contains("reel"));
    let after = log[0].after.as_deref().unwrap();
    assert!(after.contains("ifu"), "{after}");
    assert!(after.contains("2027-01-01"), "{after}");
}

#[test]
fn a_regime_change_the_shop_is_already_under_records_nothing() {
    let (_dir, mut conn) = open_temp();
    let day = NaiveDate::from_ymd_opt(2027, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    assert!(settings::set_regime(&mut conn, SHOP, OWNER, Regime::Reel, day).is_err());
    assert!(audit::list(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn a_price_change_is_recorded_with_both_amounts() {
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft(1_000, true))
        .unwrap()
        .id;
    products::update(&mut conn, SHOP, OWNER, p, draft(1_500, true)).unwrap();
    let log = audit::list(&mut conn, SHOP).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].entity, "product");
    assert_eq!(log[0].entity_id, Some(p));
    assert!(log[0].before.as_deref().unwrap().contains("1000"));
    assert!(log[0].after.as_deref().unwrap().contains("1500"));
}

#[test]
fn taking_a_product_off_sale_is_recorded() {
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft(1_000, true))
        .unwrap()
        .id;
    products::update(&mut conn, SHOP, OWNER, p, draft(1_000, false)).unwrap();
    let log = audit::list(&mut conn, SHOP).unwrap();
    assert_eq!(log.len(), 1);
    let after = log[0].after.as_deref().unwrap();
    assert!(after.contains("\"active\":false"), "{after}");
}

#[test]
fn an_edit_that_touches_neither_a_price_nor_the_active_flag_records_nothing() {
    // features.md §5 names price changes and the deletion-like actions. A
    // renamed product on every keystroke would bury them.
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft(1_000, true))
        .unwrap()
        .id;
    let mut renamed = draft(1_000, true);
    renamed.name = "Sucre cristallisé".to_string();
    products::update(&mut conn, SHOP, OWNER, p, renamed).unwrap();
    assert!(audit::list(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn the_log_is_scoped_to_its_shop() {
    use diesel::prelude::*;
    let (_dir, mut conn) = open_temp();
    shops::update_store(&mut conn, SHOP, OWNER, block("Supérette")).unwrap();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut conn)
        .unwrap();
    assert_eq!(audit::list(&mut conn, SHOP).unwrap().len(), 1);
    assert!(audit::list(&mut conn, 2).unwrap().is_empty());
}
