// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The stock ledger against a real temp SQLite file. features.md §1: the
//! ledger is append-only and the quantity on the product is a cache of it.

use diesel::prelude::*;
use dzpos_core::error::CoreError;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::models::stock::{Movement, MovementKind};
use dzpos_core::money::Money;
use dzpos_core::services::{audit, clock, products, stock};

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
    assert!(stock::recount(&mut conn, SHOP, OWNER)
        .unwrap()
        .drifts
        .is_empty());
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
    assert!(stock::recount(&mut conn, SHOP, OWNER)
        .unwrap()
        .drifts
        .is_empty());
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

/// Today on the shop's calendar, read the way the service reads it.
fn today() -> String {
    clock::now().date().format("%Y-%m-%d").to_string()
}

/// Writes a cached quantity no movement explains, which is what a repaired
/// file or an older build looks like. Nothing in the app can produce one, so
/// the raw statement is the only honest way to set the test up.
fn forge_cache(conn: &mut SqliteConnection, product_id: i32, qty_milli: i64) {
    diesel::sql_query(format!(
        "UPDATE products SET qty_on_hand_milli = {qty_milli} WHERE id = {product_id}"
    ))
    .execute(conn)
    .unwrap();
}

fn drift_rows(conn: &mut SqliteConnection) -> Vec<dzpos_core::models::audit::AuditEntry> {
    audit::list(conn, SHOP)
        .unwrap()
        .into_iter()
        .filter(|e| e.action == audit::ACTION_STOCK_DRIFT)
        .collect()
}

#[test]
fn a_forged_cache_is_reported_corrected_and_written_into_the_log() {
    // features.md §1: the ledger is the truth and the quantity on the
    // product is a cache of it, so the recount writes the ledger back over
    // the cache and the audit row is what says it happened.
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft("Sucre", 24_000)).unwrap();
    products::create(&mut conn, SHOP, OWNER, draft("Farine", 3_000)).unwrap();
    forge_cache(&mut conn, p.id, 99_000);

    let report = stock::recount(&mut conn, SHOP, OWNER).unwrap();
    assert_eq!(report.checked, 2, "both products were compared");
    assert_eq!(report.day, today());
    assert_eq!(report.drifts.len(), 1);
    assert_eq!(report.drifts[0].product_id, p.id);
    assert_eq!(report.drifts[0].name, "Sucre");
    assert_eq!(report.drifts[0].cached_milli, 99_000);
    assert_eq!(report.drifts[0].ledger_milli, 24_000);
    assert_eq!(report.drifts[0].difference_milli(), -75_000);

    assert_eq!(
        products::get(&mut conn, SHOP, p.id)
            .unwrap()
            .qty_on_hand_milli,
        24_000,
        "the cache was reported and left wrong"
    );

    let rows = drift_rows(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].entity, "product");
    assert_eq!(rows[0].entity_id, Some(p.id));
    assert_eq!(rows[0].user_id, OWNER);
    let before: serde_json::Value =
        serde_json::from_str(rows[0].before.as_deref().unwrap()).unwrap();
    let after: serde_json::Value = serde_json::from_str(rows[0].after.as_deref().unwrap()).unwrap();
    assert_eq!(before["qty_on_hand_milli"], 99_000);
    assert_eq!(after["qty_on_hand_milli"], 24_000);
    assert_eq!(after["difference_milli"], -75_000);
    assert_eq!(after["name"], "Sucre");
    assert_eq!(after["day"], today());
}

#[test]
fn a_shop_with_nothing_wrong_marks_the_run_and_writes_no_row() {
    // A quiet night is the usual night. The marker still moves, or the loop
    // would recount the same shop every hour it is switched on.
    let (_dir, mut conn) = open_temp();
    products::create(&mut conn, SHOP, OWNER, draft("Sucre", 24_000)).unwrap();

    let report = stock::recount(&mut conn, SHOP, OWNER).unwrap();
    assert!(report.drifts.is_empty());
    assert_eq!(report.checked, 1);
    assert!(drift_rows(&mut conn).is_empty(), "a clean shop wrote a row");
    assert_eq!(
        stock::last_recount(&mut conn, SHOP).unwrap().last_run_day,
        Some(today())
    );
}

#[test]
fn the_recount_is_due_once_on_a_day_and_not_again() {
    // The decision on its own, driven by the clock rather than by a loop: a
    // restart at 23:59 must not run it twice, and a marker in the future (a
    // file carried back from a machine whose clock was ahead) must not make
    // it run every hour either.
    assert!(stock::is_due(None, "2026-09-10"), "never run is due");
    assert!(stock::is_due(Some("2026-09-09"), "2026-09-10"));
    assert!(!stock::is_due(Some("2026-09-10"), "2026-09-10"));
    assert!(!stock::is_due(Some("2026-09-11"), "2026-09-10"));
}

#[test]
fn the_due_recount_runs_once_and_the_next_call_the_same_day_does_nothing() {
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft("Sucre", 24_000)).unwrap();
    forge_cache(&mut conn, p.id, 99_000);

    let first = stock::recount_if_due(&mut conn, SHOP, OWNER).unwrap();
    assert_eq!(first.map(|r| r.drifts.len()), Some(1));
    assert!(
        stock::recount_if_due(&mut conn, SHOP, OWNER)
            .unwrap()
            .is_none(),
        "the same day ran twice"
    );
    assert_eq!(drift_rows(&mut conn).len(), 1);
}

#[test]
fn a_recount_looks_only_at_its_own_shop() {
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

    let mine = stock::recount(&mut conn, SHOP, OWNER).unwrap();
    assert!(mine.drifts.is_empty());
    assert_eq!(mine.checked, 1, "another shop's product was compared");

    let other = stock::recount(&mut conn, 2, OWNER).unwrap();
    assert_eq!(other.drifts.len(), 1);
    assert_eq!(other.drifts[0].name, "Ailleurs");
    // The marker and the log belong to the shop that drifted, not to mine.
    assert!(drift_rows(&mut conn).is_empty());
    assert!(stock::last_recount(&mut conn, SHOP)
        .unwrap()
        .drifts
        .is_empty());
    assert_eq!(stock::last_recount(&mut conn, 2).unwrap().drifts.len(), 1);
}

#[test]
fn the_last_run_reads_its_drifts_back_out_of_the_log() {
    // The audit rows are the record of a recount (no second table), so the
    // panel asks the log what the last run corrected.
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft("Sucre", 24_000)).unwrap();
    forge_cache(&mut conn, p.id, 99_000);
    stock::recount(&mut conn, SHOP, OWNER).unwrap();

    let last = stock::last_recount(&mut conn, SHOP).unwrap();
    assert_eq!(last.last_run_day, Some(today()));
    assert_eq!(last.drifts.len(), 1);
    assert_eq!(last.drifts[0].product_id, p.id);
    assert_eq!(last.drifts[0].name, "Sucre");
    assert_eq!(last.drifts[0].cached_milli, 99_000);
    assert_eq!(last.drifts[0].ledger_milli, 24_000);
}

#[test]
fn a_shop_that_has_never_recounted_has_no_day_and_no_drift() {
    let (_dir, mut conn) = open_temp();
    let last = stock::last_recount(&mut conn, SHOP).unwrap();
    assert_eq!(last.last_run_day, None);
    assert!(last.drifts.is_empty());
}
