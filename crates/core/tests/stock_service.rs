// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The stock ledger against a real temp SQLite file. features.md §1: the
//! ledger is append-only and the quantity on the product is a cache of it.

use diesel::prelude::*;
use dzpos_core::error::CoreError;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::models::stock::{Movement, MovementKind};
use dzpos_core::money::Money;
use dzpos_core::services::{products, stock};

const SHOP: i32 = 1;
const SEEDED_CATEGORY: i32 = 1;
const OWNER: i32 = 1;

mod common;

use common::open_temp;

fn draft(name: &str, qty_milli: i64) -> NewProduct {
    NewProduct {
        name: name.to_string(),
        barcode: None,
        category_id: Some(SEEDED_CATEGORY),
        unit: Unit::Kg,
        cost: Money::centimes(820),
        selling: Money::centimes(920),
        wholesale: None,
        qty_on_hand_milli: qty_milli,
        low_stock_at_milli: 0,
        rate_bps: None,
        active: true,
    }
}

fn movement(product_id: i32, kind: MovementKind, qty_milli: i64) -> Movement {
    Movement {
        product_id,
        kind,
        qty_milli,
        unit_cost: Money::centimes(820),
        document_id: None,
        user_id: OWNER,
    }
}

#[test]
fn a_product_created_with_stock_gets_an_opening_movement_not_a_written_column() {
    // architecture.md, Data: the ledger is the truth. Setting the column
    // directly left a quantity no movement explained, and the nightly
    // re-derivation would have reported it as drift for ever.
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft("Sucre", 24_000)).unwrap();
    assert_eq!(p.qty_on_hand_milli, 24_000);
    let ledger = stock::list_for_product(&mut conn, SHOP, p.id).unwrap();
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].kind, MovementKind::Opening);
    assert_eq!(ledger[0].qty_milli, 24_000);
    assert_eq!(ledger[0].user_id, OWNER);
    assert!(stock::rederive(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn a_product_created_empty_opens_no_movement() {
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft("Farine", 0)).unwrap();
    assert_eq!(p.qty_on_hand_milli, 0);
    assert!(stock::list_for_product(&mut conn, SHOP, p.id)
        .unwrap()
        .is_empty());
}

#[test]
fn a_sale_movement_takes_the_quantity_off_the_cached_count() {
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft("Sucre", 24_000)).unwrap();
    stock::record(&mut conn, SHOP, &movement(p.id, MovementKind::Sale, -1_500)).unwrap();
    let after = products::get(&mut conn, SHOP, p.id).unwrap();
    assert_eq!(after.qty_on_hand_milli, 22_500);
    assert!(stock::rederive(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn stock_is_allowed_to_go_below_zero() {
    // A shop's count is often wrong before the first inventory, and refusing
    // the sale would stop the till over a number nobody typed in.
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft("Sucre", 1_000)).unwrap();
    stock::record(&mut conn, SHOP, &movement(p.id, MovementKind::Sale, -3_000)).unwrap();
    assert_eq!(
        products::get(&mut conn, SHOP, p.id)
            .unwrap()
            .qty_on_hand_milli,
        -2_000
    );
}

#[test]
fn a_movement_for_another_shops_product_is_refused() {
    // Rule 3: every query scoped by shop_id. Without the scope the movement
    // would land and quietly move another shop's stock.
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft("Sucre", 1_000)).unwrap();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut conn)
        .unwrap();
    let err = stock::record(&mut conn, 2, &movement(p.id, MovementKind::Sale, -1_000)).unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::NotFound {
                entity: "product",
                ..
            }
        ),
        "{err:?}"
    );
    assert_eq!(
        products::get(&mut conn, SHOP, p.id)
            .unwrap()
            .qty_on_hand_milli,
        1_000,
        "the movement moved a product it could not see"
    );
}

#[test]
fn rederive_reports_a_cached_count_the_ledger_does_not_explain() {
    // features.md §1: a nightly job re-derives the quantity and reports
    // drift. The cache is written here behind the service's back, which is
    // what a repaired file or an older build would look like.
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft("Sucre", 24_000)).unwrap();
    diesel::sql_query("UPDATE products SET qty_on_hand_milli = 99000 WHERE id = 1")
        .execute(&mut conn)
        .unwrap();
    let drift = stock::rederive(&mut conn, SHOP).unwrap();
    assert_eq!(drift.len(), 1);
    assert_eq!(drift[0].product_id, p.id);
    assert_eq!(drift[0].cached_milli, 99_000);
    assert_eq!(drift[0].ledger_milli, 24_000);
}

#[test]
fn rederive_looks_only_at_its_own_shop() {
    let (_dir, mut conn) = open_temp();
    products::create(&mut conn, SHOP, OWNER, draft("Sucre", 24_000)).unwrap();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO products (shop_id, name, unit, cost_centimes, selling_centimes, \
         qty_on_hand_milli, low_stock_at_milli, rate_bps) \
         VALUES (2, 'Ailleurs', 'kg', 0, 0, 5000, 0, 1900)",
    )
    .execute(&mut conn)
    .unwrap();
    assert!(stock::rederive(&mut conn, SHOP).unwrap().is_empty());
    assert_eq!(stock::rederive(&mut conn, 2).unwrap().len(), 1);
}
