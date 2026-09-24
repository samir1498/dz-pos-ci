// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! A purchase's own number (plan shop-manual-test-findings T45) and the four
//! figures its page shows (T46): features.md §1, Purchase.
//!
//! Its own file because `purchases_service.rs` is at the length
//! `scripts/file-sizes.json` pins it to and may not grow.
//!
//! Every expected figure below is worked out by hand in the comment above it,
//! from the rule in features.md, and never read back from the code under
//! test.

use diesel::sqlite::SqliteConnection;
use diesel::RunQueryDsl;
use dzpos_kernel::services::clock;
use dzpos_retail::models::product::{NewProduct, Unit};
use dzpos_retail::money::{Bps, Money};
use dzpos_retail::services::products;
use dzpos_retail::services::purchases::{self, NewLine, NewPurchase, ReceiveLine};
use dzpos_retail::services::supplier_debt::{self, PaymentMethod};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

mod common;

use common::{a_supplier, open_temp};

fn a_product(conn: &mut SqliteConnection, name: &str) -> i32 {
    products::create(
        conn,
        SHOP,
        OWNER,
        NewProduct {
            name: name.to_string(),
            barcode: None,
            category_id: None,
            unit: Unit::Piece,
            cost: Money::ZERO,
            selling: Money::centimes(40_000),
            wholesale: None,
            qty_on_hand_milli: 0,
            low_stock_at_milli: 0,
            rate_bps: Some(Bps::new(1900).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id
}

/// One line of one unit at 100,00, dated `day`, with the supplier's own paper
/// number `theirs`.
fn one_line(supplier_id: i32, product_id: i32, day: &str, theirs: Option<&str>) -> NewPurchase {
    NewPurchase {
        supplier_id,
        supplier_document_number: theirs.map(str::to_string),
        purchase_date: day.to_string(),
        due_date: None,
        transport: Money::ZERO,
        extra_costs: Money::ZERO,
        note: None,
        lines: vec![NewLine {
            product_id,
            qty_ordered_milli: 1_000,
            unit_cost: Money::centimes(10_000),
        }],
        paid_now: None,
        receive_now: false,
    }
}

#[test]
fn two_orders_take_the_number_after_the_last() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg");

    let first = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        one_line(supplier, farine, "2026-09-24", None),
    )
    .unwrap();
    let second = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        one_line(supplier, farine, "2026-09-24", None),
    )
    .unwrap();

    assert_eq!(first.purchase.printed_number(), "BA-2026-000001");
    assert_eq!(second.purchase.printed_number(), "BA-2026-000002");
}

#[test]
fn a_refused_purchase_burns_no_number() {
    // Two refusals, one on each side of the number being taken. The first is
    // refused before the transaction opens (an order with no line); the
    // second is refused by the file itself at the insert, after `take_next`
    // has advanced the counter inside the transaction: the supplier's own
    // paper number `BL-7` is already on an order of this supplier. The next
    // order that is written must still be number 2, which only holds if the
    // failed write rolled the counter back with it.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg");

    let first = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        one_line(supplier, farine, "2026-09-24", Some("BL-7")),
    )
    .unwrap();
    assert_eq!(first.purchase.printed_number(), "BA-2026-000001");

    let mut empty = one_line(supplier, farine, "2026-09-24", None);
    empty.lines.clear();
    assert!(purchases::save(&mut conn, SHOP, OWNER, empty).is_err());

    let twice = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        one_line(supplier, farine, "2026-09-24", Some("BL-7")),
    );
    assert!(twice.is_err(), "the same supplier paper was taken twice");

    let next = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        one_line(supplier, farine, "2026-09-24", None),
    )
    .unwrap();
    assert_eq!(next.purchase.printed_number(), "BA-2026-000002");
}

#[test]
fn the_purchase_series_restarts_at_one_in_the_new_year_and_the_old_one_keeps_counting() {
    // The year is the one the order is dated in. An order dated 31 December
    // written after the first one of January still counts in the old year.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg");

    let december = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        one_line(supplier, farine, "2026-12-31", None),
    )
    .unwrap();
    let january = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        one_line(supplier, farine, "2027-01-02", None),
    )
    .unwrap();
    let late = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        one_line(supplier, farine, "2026-12-30", None),
    )
    .unwrap();

    assert_eq!(december.purchase.printed_number(), "BA-2026-000001");
    assert_eq!(january.purchase.printed_number(), "BA-2027-000001");
    assert_eq!(late.purchase.printed_number(), "BA-2026-000002");
}

#[test]
fn a_delivery_is_named_by_its_own_printed_number() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg");
    let mut order = one_line(supplier, farine, "2026-09-24", None);
    order.receive_now = true;

    let saved = purchases::save(&mut conn, SHOP, OWNER, order).unwrap();

    // The delivery's year is the shop clock's at the moment it was written,
    // which is the year its series key carries.
    let year = saved.receipts[0].receipt.series.replace("reception:", "");
    assert_eq!(
        saved.receipts[0].receipt.printed_number(),
        format!("BR-{year}-000001")
    );
}

#[test]
fn an_order_page_reads_its_total_what_arrived_what_was_paid_and_what_is_owed() {
    // Three lines, one of them worth nothing, and extras that do not divide
    // evenly (features.md §1, the landed cost):
    //
    //   A: 3 units at 100,00 = 30 000   C: 1 unit at 0 = 0   B: 2 at 50,00 = 10 000
    //   order value 40 000; transport 1 000 + extra 1 = 1 001 to spread.
    //   A's share: floor(1 001 × 30 000 / 40 000) = floor(750,75) = 750
    //   C's share: floor(1 001 × 0 / 40 000) = 0
    //   B (last) takes the rest: 1 001 − 750 − 0 = 251
    //   per unit: A floor(750 / 3) = 250 → 10 250; C 0; B floor(251 / 2) = 125 → 5 125
    //   total: 3 × 10 250 + 0 + 2 × 5 125 = 30 750 + 10 250 = 41 000
    //
    // One centime of the 1 001 is lost to B's per-unit floor, which is the
    // one loss the rule allows, so the total is 41 000 and not 41 001.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let a = a_product(&mut conn, "Huile");
    let c = a_product(&mut conn, "Echantillon");
    let b = a_product(&mut conn, "Sucre");
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            supplier_id: supplier,
            supplier_document_number: None,
            purchase_date: "2026-09-24".to_string(),
            due_date: None,
            transport: Money::centimes(1_000),
            extra_costs: Money::centimes(1),
            note: None,
            lines: vec![
                NewLine {
                    product_id: a,
                    qty_ordered_milli: 3_000,
                    unit_cost: Money::centimes(10_000),
                },
                NewLine {
                    product_id: c,
                    qty_ordered_milli: 1_000,
                    unit_cost: Money::ZERO,
                },
                NewLine {
                    product_id: b,
                    qty_ordered_milli: 2_000,
                    unit_cost: Money::centimes(5_000),
                },
            ],
            paid_now: None,
            receive_now: false,
        },
    )
    .unwrap();
    let id = saved.purchase.id;

    // Nothing has arrived: debt rises on receipt, so nothing is owed yet.
    assert_eq!(saved.account.total, Money::centimes(41_000));
    assert_eq!(saved.account.received, Money::ZERO);
    assert_eq!(saved.account.paid, Money::ZERO);
    assert_eq!(saved.account.owed, Money::ZERO);

    // One unit of A arrives: floor(10 250 × 1 000 / 1 000) = 10 250.
    let line_a = saved.lines[0].id;
    purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        id,
        vec![ReceiveLine {
            purchase_line_id: line_a,
            qty_milli: 1_000,
        }],
        None,
    )
    .unwrap();

    // 40,00 paid; the only open order is this one, so it takes all of it.
    supplier_debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(4_000),
        PaymentMethod::Cash,
        None,
        clock::now(),
    )
    .unwrap();
    let paid = purchases::get(&mut conn, SHOP, id).unwrap().account;
    assert_eq!(paid.total, Money::centimes(41_000));
    assert_eq!(paid.received, Money::centimes(10_250));
    assert_eq!(paid.paid, Money::centimes(4_000));
    // 10 250 − 4 000
    assert_eq!(paid.owed, Money::centimes(6_250));

    // The unit goes back: its 10 250 comes off what arrived, and the 4 000
    // already paid is now credit the supplier holds, which reads below zero.
    purchases::return_to_supplier(
        &mut conn,
        SHOP,
        OWNER,
        id,
        vec![ReceiveLine {
            purchase_line_id: line_a,
            qty_milli: 1_000,
        }],
        None,
    )
    .unwrap();
    let returned = purchases::get(&mut conn, SHOP, id).unwrap().account;
    assert_eq!(returned.received, Money::ZERO);
    assert_eq!(returned.paid, Money::centimes(4_000));
    assert_eq!(returned.owed, Money::centimes(-4_000));
}

#[test]
fn a_total_past_the_top_of_the_range_is_an_error_and_not_a_panic() {
    // Two lines each landed at 5 000 000 000 000 000 000 centimes for one
    // unit: each is inside an i64, their sum (10^19) is past i64::MAX
    // (about 9,22 × 10^18). No save can write these (the spread would refuse
    // first), so the rows are written straight into the file, the way a
    // repaired or restored file could hold them.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let a = a_product(&mut conn, "Huile");
    let b = a_product(&mut conn, "Sucre");
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        one_line(supplier, a, "2026-09-24", None),
    )
    .unwrap();
    let id = saved.purchase.id;
    diesel::sql_query(format!(
        "UPDATE purchase_lines SET unit_cost_centimes = 5000000000000000000, \
         landed_unit_cost_centimes = 5000000000000000000 WHERE purchase_id = {id}"
    ))
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(format!(
        "INSERT INTO purchase_lines (shop_id, purchase_id, product_id, qty_ordered_milli, \
         unit_cost_centimes, landed_unit_cost_centimes) \
         VALUES ({SHOP}, {id}, {b}, 1000, 5000000000000000000, 5000000000000000000)"
    ))
    .execute(&mut conn)
    .unwrap();

    assert!(purchases::get(&mut conn, SHOP, id).is_err());
}
