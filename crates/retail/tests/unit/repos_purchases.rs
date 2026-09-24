// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::models::purchase::PurchaseStatus;
use crate::models::supplier::SupplierRowWrite;
use crate::repos::suppliers;
use crate::repos::testdb::{open, OWNER, SHOP};

fn a_supplier(conn: &mut SqliteConnection, name: &str) -> i32 {
    suppliers::insert(
        conn,
        &SupplierRowWrite {
            shop_id: SHOP,
            name: name.to_string(),
            phone: None,
            address: None,
            rc: None,
            nif: None,
            nis: None,
            ai: None,
            notes: None,
            active: true,
            updated_at: dzpos_kernel::services::clock::now(),
        },
    )
    .unwrap()
    .id
}

fn a_product(conn: &mut SqliteConnection) -> i32 {
    diesel::sql_query(
        "INSERT INTO products (shop_id, name, unit, cost_centimes, selling_centimes, \
         qty_on_hand_milli, low_stock_at_milli, rate_bps) \
         VALUES (1, 'Farine 5kg', 'piece', 20000, 26000, 0, 0, 1900)",
    )
    .execute(conn)
    .unwrap();
    1
}

/// Every draft its own number, so two orders written in one test do not meet
/// on the series' unique index; the numbering itself is the service's.
static NEXT_NUMBER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

fn draft(supplier_id: i32, day: &str) -> PurchaseRowWrite {
    PurchaseRowWrite {
        series_year: day[..4].parse().unwrap(),
        number: NEXT_NUMBER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        shop_id: SHOP,
        supplier_id,
        supplier_document_number: None,
        purchase_date: day.to_string(),
        due_date: None,
        transport_centimes: 0,
        extra_costs_centimes: 0,
        status: PurchaseStatus::Ordered,
        user_id: OWNER,
        note: None,
    }
}

#[test]
fn a_purchase_is_read_back_with_the_costs_and_the_state_it_was_written_with() {
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let mut write = draft(supplier, "2026-09-10");
    write.supplier_document_number = Some("BL-77".to_string());
    write.due_date = Some("2026-10-10".to_string());
    write.transport_centimes = 50_000;
    write.extra_costs_centimes = 12_500;
    let made = insert(&mut conn, &write).unwrap();
    let read = get(&mut conn, SHOP, made.id).unwrap();
    assert_eq!(read, made);
    assert_eq!(read.transport, crate::money::Money::centimes(50_000));
    assert_eq!(read.extra_costs, crate::money::Money::centimes(12_500));
    assert_eq!(read.status, PurchaseStatus::Ordered);
    assert_eq!(read.supplier_document_number.as_deref(), Some("BL-77"));
}

#[test]
fn every_state_goes_to_the_file_and_comes_back() {
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let made = insert(&mut conn, &draft(supplier, "2026-09-10")).unwrap();
    for status in [
        PurchaseStatus::PartiallyReceived,
        PurchaseStatus::Received,
        PurchaseStatus::ClosedShort,
        PurchaseStatus::Cancelled,
        PurchaseStatus::Ordered,
    ] {
        let after = set_status(&mut conn, SHOP, made.id, status).unwrap();
        assert_eq!(after.status, status);
        assert_eq!(get(&mut conn, SHOP, made.id).unwrap().status, status);
    }
    assert!(set_status(&mut conn, 2, made.id, PurchaseStatus::Received).is_err());
}

#[test]
fn the_list_is_newest_first_and_another_shops_purchases_are_not_in_it() {
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let older = insert(&mut conn, &draft(supplier, "2026-09-01")).unwrap();
    let newer = insert(&mut conn, &draft(supplier, "2026-09-10")).unwrap();
    let days: Vec<i32> = list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .map(|p| p.id)
        .collect();
    assert_eq!(days, vec![newer.id, older.id]);
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(&mut conn)
        .unwrap();
    assert!(list(&mut conn, 2).unwrap().is_empty());
    assert!(get(&mut conn, 2, newer.id).is_err());
}

#[test]
fn a_line_keeps_the_landed_cost_it_was_saved_with_and_counts_what_has_arrived() {
    // The landed cost is fixed once, so a report reads a margin against
    // the cost the goods actually landed at.
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let product = a_product(&mut conn);
    let purchase = insert(&mut conn, &draft(supplier, "2026-09-10")).unwrap();
    let line = insert_line(
        &mut conn,
        &PurchaseLineRowWrite {
            shop_id: SHOP,
            purchase_id: purchase.id,
            product_id: product,
            qty_ordered_milli: 10_000,
            unit_cost_centimes: 20_000,
            landed_unit_cost_centimes: 25_000,
            qty_received_milli: 0,
            qty_returned_milli: 0,
        },
    )
    .unwrap();
    assert_eq!(line.landed_unit_cost, crate::money::Money::centimes(25_000));
    let after = add_received(&mut conn, SHOP, line.id, 4_000).unwrap();
    assert_eq!(after.qty_received_milli, 4_000);
    let again = add_received(&mut conn, SHOP, line.id, 6_000).unwrap();
    assert_eq!(again.qty_received_milli, 10_000);
    // The file refuses more than was ordered; how much one receipt may
    // add is the service's rule, and this is the floor under it.
    assert!(add_received(&mut conn, SHOP, line.id, 1).is_err());
    assert_eq!(lines(&mut conn, SHOP, purchase.id).unwrap().len(), 1);
    assert!(lines(&mut conn, 2, purchase.id).unwrap().is_empty());
}

#[test]
fn what_goes_back_to_the_supplier_is_counted_and_never_more_than_arrived() {
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let product = a_product(&mut conn);
    let purchase = insert(&mut conn, &draft(supplier, "2026-09-10")).unwrap();
    let line = insert_line(
        &mut conn,
        &PurchaseLineRowWrite {
            shop_id: SHOP,
            purchase_id: purchase.id,
            product_id: product,
            qty_ordered_milli: 10_000,
            unit_cost_centimes: 20_000,
            landed_unit_cost_centimes: 25_000,
            qty_received_milli: 0,
            qty_returned_milli: 0,
        },
    )
    .unwrap();
    add_received(&mut conn, SHOP, line.id, 6_000).unwrap();
    let after = add_returned(&mut conn, SHOP, line.id, 2_000).unwrap();
    assert_eq!(after.qty_returned_milli, 2_000);
    assert_eq!(after.qty_received_milli, 6_000);
    let again = add_returned(&mut conn, SHOP, line.id, 4_000).unwrap();
    assert_eq!(again.qty_returned_milli, 6_000);
    // Goods the shop never took in are goods it cannot send back.
    assert!(add_returned(&mut conn, SHOP, line.id, 1).is_err());
}

#[test]
fn a_receipt_and_a_return_of_nothing_are_refused_before_the_file_sees_them() {
    // A column expression adding zero writes a row and reports success,
    // and one adding a negative walks the total backwards inside a CHECK
    // that only looks at the ceiling. Both are a caller sending the wrong
    // number, so both come back as a validation error.
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let product = a_product(&mut conn);
    let purchase = insert(&mut conn, &draft(supplier, "2026-09-10")).unwrap();
    let line = insert_line(
        &mut conn,
        &PurchaseLineRowWrite {
            shop_id: SHOP,
            purchase_id: purchase.id,
            product_id: product,
            qty_ordered_milli: 10_000,
            unit_cost_centimes: 20_000,
            landed_unit_cost_centimes: 25_000,
            qty_received_milli: 4_000,
            qty_returned_milli: 0,
        },
    )
    .unwrap();
    for nothing in [0, -1_000] {
        match add_received(&mut conn, SHOP, line.id, nothing) {
            Err(CoreError::Validation { ref field, .. }) if field == "qty_milli" => {}
            other => panic!("a receipt of {nothing} answered {other:?}"),
        }
        match add_returned(&mut conn, SHOP, line.id, nothing) {
            Err(CoreError::Validation { ref field, .. }) if field == "qty_milli" => {}
            other => panic!("a return of {nothing} answered {other:?}"),
        }
    }
    // And nothing moved on the way past.
    let read = lines(&mut conn, SHOP, purchase.id).unwrap();
    assert_eq!(read[0].qty_received_milli, 4_000);
    assert_eq!(read[0].qty_returned_milli, 0);
}

#[test]
fn a_receipt_and_its_lines_say_what_arrived_on_one_delivery() {
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let product = a_product(&mut conn);
    let purchase = insert(&mut conn, &draft(supplier, "2026-09-10")).unwrap();
    let line = insert_line(
        &mut conn,
        &PurchaseLineRowWrite {
            shop_id: SHOP,
            purchase_id: purchase.id,
            product_id: product,
            qty_ordered_milli: 10_000,
            unit_cost_centimes: 20_000,
            landed_unit_cost_centimes: 25_000,
            qty_received_milli: 0,
            qty_returned_milli: 0,
        },
    )
    .unwrap();
    let receipt = insert_receipt(
        &mut conn,
        &PurchaseReceiptRowWrite {
            shop_id: SHOP,
            purchase_id: purchase.id,
            series: "reception:2026".to_string(),
            number: 1,
            received_at: dzpos_kernel::services::clock::now(),
            user_id: OWNER,
            note: None,
        },
    )
    .unwrap();
    assert_eq!(receipt.series, "reception:2026");
    assert_eq!(receipt.number, 1);
    insert_receipt_line(
        &mut conn,
        &PurchaseReceiptLineRowWrite {
            shop_id: SHOP,
            purchase_id: purchase.id,
            receipt_id: receipt.id,
            purchase_line_id: line.id,
            qty_milli: 4_000,
        },
    )
    .unwrap();
    let read = receipt_lines(&mut conn, SHOP, receipt.id).unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].qty_milli, 4_000);
    assert_eq!(receipts(&mut conn, SHOP, purchase.id).unwrap().len(), 1);
    assert!(receipts(&mut conn, 2, purchase.id).unwrap().is_empty());
    assert!(receipt_lines(&mut conn, 2, receipt.id).unwrap().is_empty());
    // The number is the shop's own, taken once inside its series.
    assert!(insert_receipt(
        &mut conn,
        &PurchaseReceiptRowWrite {
            shop_id: SHOP,
            purchase_id: purchase.id,
            series: "reception:2026".to_string(),
            number: 1,
            received_at: dzpos_kernel::services::clock::now(),
            user_id: OWNER,
            note: None,
        },
    )
    .is_err());
}
