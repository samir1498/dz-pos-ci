// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::models::supplier::SupplierRowWrite;
use crate::models::supplier_debt::{PaymentMethod, SupplierDebtKind};
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

fn movement(
    supplier_id: i32,
    kind: SupplierDebtKind,
    debit: i64,
    credit: i64,
) -> SupplierDebtRowWrite {
    SupplierDebtRowWrite {
        shop_id: SHOP,
        supplier_id,
        purchase_id: None,
        kind,
        debit_centimes: debit,
        credit_centimes: credit,
        user_id: OWNER,
        note: None,
        payment_mode: None,
        // Stamped from the shop's clock, which is what `append` now asks
        // of every caller.
        created_at: Some(dzpos_kernel::services::clock::now()),
    }
}

#[test]
fn a_movement_with_no_moment_on_it_is_refused_rather_than_dated_by_the_file() {
    // The column's default is SQLite's CURRENT_TIMESTAMP, which is UTC,
    // and every period this app answers for is a stretch of days on the
    // shop's calendar (UTC+1). A payment taken at 00:30 in Algiers would
    // land on the day before in the file and fall out of the day the shop
    // counted, so the caller stamps it from the clock or the row is
    // refused here.
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Fournisseur Sans Heure");
    let mut unstamped = movement(supplier, SupplierDebtKind::Payment, 0, 1_000);
    unstamped.payment_mode = Some(PaymentMethod::Cash);
    unstamped.created_at = None;
    match append(&mut conn, &unstamped) {
        Err(RetailError::Unstamped { entity }) => assert_eq!(entity, "supplier_ledger"),
        other => panic!("expected an unstamped row to be refused, got {other:?}"),
    }
    // And nothing was written: a refusal leaves the ledger as it was.
    assert!(ledger(&mut conn, SHOP, supplier).unwrap().is_empty());
}

#[test]
fn a_supplier_with_no_movement_owes_nothing() {
    // The SUM is coalesced to zero, so an empty ledger answers a balance
    // rather than no row at all.
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    assert_eq!(
        balance(&mut conn, SHOP, supplier).unwrap(),
        crate::money::Money::ZERO
    );
    assert!(ledger(&mut conn, SHOP, supplier).unwrap().is_empty());
}

#[test]
fn the_balance_is_the_sum_of_the_ledger_and_may_go_below_zero() {
    // A shop that has paid in advance is owed goods, and the ledger says
    // so rather than clamping at nothing.
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    append(
        &mut conn,
        &movement(supplier, SupplierDebtKind::Opening, 250_000, 0),
    )
    .unwrap();
    append(
        &mut conn,
        &movement(supplier, SupplierDebtKind::Purchase, 100_000, 0),
    )
    .unwrap();
    let mut paid = movement(supplier, SupplierDebtKind::Payment, 0, 400_000);
    paid.payment_mode = Some(PaymentMethod::Cash);
    append(&mut conn, &paid).unwrap();
    assert_eq!(
        balance(&mut conn, SHOP, supplier).unwrap(),
        crate::money::Money::centimes(-50_000)
    );
}

#[test]
fn every_kind_and_both_modes_go_to_the_file_and_come_back() {
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    for kind in [
        SupplierDebtKind::Opening,
        SupplierDebtKind::Purchase,
        SupplierDebtKind::Return,
        SupplierDebtKind::Adjustment,
    ] {
        let written = append(&mut conn, &movement(supplier, kind, 1000, 0)).unwrap();
        assert_eq!(written.kind, kind);
        assert_eq!(written.payment_mode, None);
    }
    for mode in [PaymentMethod::Cash, PaymentMethod::Card] {
        let mut paid = movement(supplier, SupplierDebtKind::Payment, 0, 1000);
        paid.payment_mode = Some(mode);
        let written = append(&mut conn, &paid).unwrap();
        assert_eq!(written.kind, SupplierDebtKind::Payment);
        assert_eq!(written.payment_mode, Some(mode));
    }
}

#[test]
fn the_ledger_reads_newest_first_and_the_id_breaks_a_tie_inside_one_second() {
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let stamped = dzpos_kernel::services::clock::now();
    let mut ids = Vec::new();
    for _ in 0..3 {
        let mut write = movement(supplier, SupplierDebtKind::Purchase, 1000, 0);
        write.created_at = Some(stamped);
        ids.push(append(&mut conn, &write).unwrap().id);
    }
    ids.reverse();
    let read: Vec<i32> = ledger(&mut conn, SHOP, supplier)
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(read, ids);
}

#[test]
fn each_suppliers_two_sums_come_back_in_one_query_for_the_whole_list() {
    let (_dir, mut conn) = open();
    let one = a_supplier(&mut conn, "Sarl Amrani");
    let two = a_supplier(&mut conn, "Bensalem");
    a_supplier(&mut conn, "Jamais servi");
    append(
        &mut conn,
        &movement(one, SupplierDebtKind::Purchase, 100_000, 0),
    )
    .unwrap();
    append(
        &mut conn,
        &movement(two, SupplierDebtKind::Return, 0, 30_000),
    )
    .unwrap();
    let mut rows = balances(&mut conn, SHOP).unwrap();
    rows.sort_by_key(|(supplier_id, _, _)| *supplier_id);
    // A supplier with no movement is not in the answer: no rows is no
    // debt, and the caller reads a missing one as nothing owed.
    assert_eq!(rows, vec![(one, 100_000, 0), (two, 0, 30_000)]);
}

#[test]
fn another_shops_ledger_is_neither_summed_nor_listed() {
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    append(
        &mut conn,
        &movement(supplier, SupplierDebtKind::Purchase, 100_000, 0),
    )
    .unwrap();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(&mut conn)
        .unwrap();
    assert_eq!(
        balance(&mut conn, 2, supplier).unwrap(),
        crate::money::Money::ZERO
    );
    assert!(ledger(&mut conn, 2, supplier).unwrap().is_empty());
    assert!(balances(&mut conn, 2).unwrap().is_empty());
}

#[test]
fn an_allocation_says_what_a_payment_settled_on_which_purchase() {
    let (_dir, mut conn) = open();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let purchase = crate::repos::purchases::insert(
        &mut conn,
        &crate::models::purchase::PurchaseRowWrite {
            shop_id: SHOP,
            supplier_id: supplier,
            supplier_document_number: None,
            purchase_date: "2026-09-10".to_string(),
            due_date: None,
            transport_centimes: 0,
            extra_costs_centimes: 0,
            status: crate::models::purchase::PurchaseStatus::Ordered,
            user_id: OWNER,
            note: None,
            series_year: 2026,
            number: 1,
        },
    )
    .unwrap()
    .id;
    let mut paid = movement(supplier, SupplierDebtKind::Payment, 0, 60_000);
    paid.payment_mode = Some(PaymentMethod::Cash);
    let payment = append(&mut conn, &paid).unwrap();
    allocate(
        &mut conn,
        &SupplierAllocationRowWrite {
            shop_id: SHOP,
            payment_ledger_id: payment.id,
            purchase_id: purchase,
            amount_centimes: 60_000,
        },
    )
    .unwrap();
    let of_payment = allocations_of_payment(&mut conn, SHOP, payment.id).unwrap();
    assert_eq!(of_payment.len(), 1);
    assert_eq!(of_payment[0].amount, crate::money::Money::centimes(60_000));
    assert_eq!(of_payment[0].purchase_id, purchase);
    assert_eq!(allocations(&mut conn, SHOP, purchase).unwrap().len(), 1);
    assert!(allocations(&mut conn, 2, purchase).unwrap().is_empty());
    // An allocation points at a payment by id and the foreign key alone
    // would take another shop's row.
    assert!(entry_belongs_to_shop(&mut conn, SHOP, payment.id).unwrap());
    assert!(!entry_belongs_to_shop(&mut conn, 2, payment.id).unwrap());
}
