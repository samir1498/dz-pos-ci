// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The four workbooks of features.md §1, read back cell by cell.
//!
//! Nothing here asserts on the bytes. A workbook is a zip of XML and its
//! bytes change when the writer's version does; what a shop opens is the
//! grid, so the grid is what these tests read, with `calamine`, which
//! shares no code with the writer that produced it.
//!
//! Two things are checked on every workbook: the tab carries the shop's
//! language, and a second shop's rows are not in it (rule 3). The amounts
//! are read back as numbers rather than as text, because a column a
//! spreadsheet cannot sum or total is not an export, and the number a cell
//! holds is compared with the centimes the row stores, divided nowhere but
//! in the assertion itself.

use std::io::Cursor;

use calamine::{Data, Reader, Xlsx};
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::lang::Lang;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::money::{Bps, Money, PaymentMode};
use dzpos_core::services::customers::{NewCustomer, PartyKind};
use dzpos_core::services::export::{self, DayRange};
use dzpos_core::services::sales::{self, NewSale, NewSaleLine, SaleKind};
use dzpos_core::services::suppliers::NewSupplier;
use dzpos_core::services::{customers, products, suppliers};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

mod common;

use common::open_temp;

fn at(day: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, day)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

/// The grid of one sheet, or a panic naming the tabs the workbook does hold:
/// a missing tab is the assertion this helper is there to make readable.
fn sheet(bytes: &[u8], name: &str) -> Vec<Vec<Data>> {
    let mut book: Xlsx<_> = calamine::open_workbook_from_rs(Cursor::new(bytes.to_vec())).unwrap();
    let names = book.sheet_names();
    let range = book
        .worksheet_range(name)
        .unwrap_or_else(|e| panic!("no sheet {name} ({e}); the workbook holds {names:?}"));
    range.rows().map(<[Data]>::to_vec).collect()
}

/// The header row as plain strings. The header is the stable key names, not
/// translated words: a products export is what the import reads back, so the
/// two files have to name their columns identically (features.md §1).
fn header(rows: &[Vec<Data>]) -> Vec<String> {
    rows.first()
        .unwrap_or(&Vec::new())
        .iter()
        .map(std::string::ToString::to_string)
        .collect()
}

/// The value under `column` on `row`, or a panic naming the header. A test
/// that read column 7 by its number would keep passing after a column was
/// inserted before it.
fn cell(rows: &[Vec<Data>], row: usize, column: &str) -> Data {
    let at = header(rows)
        .iter()
        .position(|h| h == column)
        .unwrap_or_else(|| panic!("no column {column} in {:?}", header(rows)));
    rows[row][at].clone()
}

/// The number in a cell, refusing text that merely looks like one: an amount
/// written as a string sums to nothing in a spreadsheet, and that is the
/// failure this whole file is about.
fn number(rows: &[Vec<Data>], row: usize, column: &str) -> f64 {
    match cell(rows, row, column) {
        Data::Float(v) => v,
        Data::Int(v) => {
            // A whole number a writer chose to store as an integer is still
            // a number; anything else is not.
            #[allow(clippy::cast_precision_loss)]
            let v = v as f64;
            v
        }
        other => panic!("{column} is {other:?}, not a number"),
    }
}

fn text(rows: &[Vec<Data>], row: usize, column: &str) -> String {
    cell(rows, row, column).to_string()
}

/// The centimes an amount cell stands for, read out of the cell rather than
/// computed from the row: `1234.56` is 123 456 centimes, and the rounding
/// here is the only place this file goes near a float.
fn centimes(rows: &[Vec<Data>], row: usize, column: &str) -> i64 {
    let value = number(rows, row, column);
    #[allow(clippy::cast_possible_truncation)]
    let rounded = (value * 100.0).round() as i64;
    rounded
}

fn seed_second_shop(conn: &mut SqliteConnection) {
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO products (shop_id, name, unit, cost_centimes, selling_centimes, \
         qty_on_hand_milli, low_stock_at_milli, rate_bps, active) \
         VALUES (2, 'Produit du voisin', 'piece', 100, 200, 0, 0, 1900, 1)",
    )
    .execute(conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO customers (shop_id, name, party_kind) VALUES (2, 'Voisin', 'company')",
    )
    .execute(conn)
    .unwrap();
    diesel::sql_query("INSERT INTO suppliers (shop_id, name) VALUES (2, 'Fournisseur du voisin')")
        .execute(conn)
        .unwrap();
}

fn a_product(conn: &mut SqliteConnection, name: &str, selling: i64) -> i32 {
    products::create(
        conn,
        SHOP,
        OWNER,
        NewProduct {
            name: name.to_string(),
            barcode: Some("6130001234567".to_string()),
            category_id: None,
            unit: Unit::Kg,
            cost: Money::centimes(8_050),
            selling: Money::centimes(selling),
            wholesale: Some(Money::centimes(9_000)),
            qty_on_hand_milli: 2_500,
            low_stock_at_milli: 1_000,
            rate_bps: Some(Bps::new(1900).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id
}

#[test]
fn the_products_workbook_names_its_tab_in_the_shops_language_and_holds_one_row_per_product() {
    let (_dir, mut conn) = open_temp();
    seed_second_shop(&mut conn);
    a_product(&mut conn, "Café moulu 250 g", 32_050);

    let bytes = export::products(&mut conn, SHOP, Lang::Fr).unwrap();
    let rows = sheet(&bytes, "Produits");

    assert_eq!(header(&rows)[0], "name");
    // The neighbour's product is in the file and not in the workbook.
    assert_eq!(rows.len(), 2, "one header row and one product");
    assert_eq!(text(&rows, 1, "name"), "Café moulu 250 g");
    assert_eq!(text(&rows, 1, "barcode"), "6130001234567");
    assert_eq!(text(&rows, 1, "unit"), "kg");
    // 320,50 DA as a number a spreadsheet can sum, not as "320,50".
    assert_eq!(centimes(&rows, 1, "selling_da"), 32_050);
    assert_eq!(centimes(&rows, 1, "cost_da"), 8_050);
    assert_eq!(centimes(&rows, 1, "wholesale_da"), 9_000);
    assert!((number(&rows, 1, "stock") - 2.5).abs() < 1e-9);
    assert!((number(&rows, 1, "rate_percent") - 19.0).abs() < 1e-9);
    assert_eq!(cell(&rows, 1, "active"), Data::Bool(true));
}

#[test]
fn the_products_tab_is_named_in_english_and_in_arabic_too() {
    let (_dir, mut conn) = open_temp();
    a_product(&mut conn, "Sucre", 11_000);

    let en = export::products(&mut conn, SHOP, Lang::En).unwrap();
    assert_eq!(header(&sheet(&en, "Products"))[0], "name");
    let ar = export::products(&mut conn, SHOP, Lang::Ar).unwrap();
    assert_eq!(header(&sheet(&ar, "المنتجات"))[0], "name");
}

#[test]
fn the_sales_workbook_writes_one_row_per_line_with_the_number_the_paper_prints() {
    let (_dir, mut conn) = open_temp();
    seed_second_shop(&mut conn);
    let first = a_product(&mut conn, "Café moulu 250 g", 32_050);
    let second = products::create(
        &mut conn,
        SHOP,
        OWNER,
        NewProduct {
            name: "Pain".to_string(),
            barcode: None,
            category_id: None,
            unit: Unit::Piece,
            cost: Money::centimes(4_000),
            selling: Money::centimes(8_000),
            wholesale: None,
            qty_on_hand_milli: 10_000,
            low_stock_at_milli: 0,
            rate_bps: Some(Bps::new(0).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id;

    sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![
                NewSaleLine {
                    product_id: first,
                    qty_milli: 1_000,
                    unit_price: None,
                    line_discount: Money::ZERO,
                },
                NewSaleLine {
                    product_id: second,
                    qty_milli: 2_000,
                    unit_price: None,
                    line_discount: Money::ZERO,
                },
            ],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(100_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(at(9)),
        },
    )
    .unwrap();

    let bytes = export::sales(&mut conn, SHOP, Lang::Fr, DayRange::default()).unwrap();
    let rows = sheet(&bytes, "Ventes");

    assert_eq!(rows.len(), 3, "one header row and one row per line");
    assert_eq!(text(&rows, 1, "number"), "TK-2026-000001");
    assert_eq!(text(&rows, 1, "kind"), "ticket");
    assert_eq!(text(&rows, 1, "status"), "issued");
    assert_eq!(text(&rows, 1, "customer"), "");
    assert_eq!(text(&rows, 1, "line_name"), "Café moulu 250 g");
    assert_eq!(centimes(&rows, 1, "line_total_da"), 32_050);
    assert_eq!(text(&rows, 2, "line_name"), "Pain");
    assert_eq!(centimes(&rows, 2, "line_total_da"), 16_000);
    // The document totals ride on every one of its lines, so a reader can
    // pivot on the number without a second sheet.
    assert_eq!(
        centimes(&rows, 1, "document_net_to_pay_da"),
        centimes(&rows, 2, "document_net_to_pay_da")
    );
    // A date, not a string: a spreadsheet sorts and filters this column.
    match cell(&rows, 1, "date") {
        Data::DateTime(_) => (),
        other => panic!("the date column holds {other:?}"),
    }
}

#[test]
fn a_range_leaves_out_the_documents_outside_it() {
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre", 11_000);
    for day in [8_u32, 12] {
        sales::issue(
            &mut conn,
            SHOP,
            OWNER,
            NewSale {
                lines: vec![NewSaleLine {
                    product_id: p,
                    qty_milli: 1_000,
                    unit_price: None,
                    line_discount: Money::ZERO,
                }],
                global_discount: Money::ZERO,
                payment_mode: PaymentMode::Cash,
                tendered: Some(Money::centimes(100_000)),
                customer_id: None,
                override_credit: false,
                kind: SaleKind::Ticket,
                issued_at: Some(at(day)),
            },
        )
        .unwrap();
    }

    let whole = export::sales(&mut conn, SHOP, Lang::Fr, DayRange::default()).unwrap();
    assert_eq!(sheet(&whole, "Ventes").len(), 3);

    let narrowed = export::sales(
        &mut conn,
        SHOP,
        Lang::Fr,
        DayRange {
            from: NaiveDate::from_ymd_opt(2026, 9, 9),
            to: NaiveDate::from_ymd_opt(2026, 9, 30),
        },
    )
    .unwrap();
    let rows = sheet(&narrowed, "Ventes");
    assert_eq!(rows.len(), 2, "only the document issued on the 12th");
    assert_eq!(text(&rows, 1, "number"), "TK-2026-000002");
}

#[test]
fn a_document_issued_in_the_last_second_of_the_closing_day_is_inside_the_range() {
    // The range used to close at 23:59:59 inclusive, which is a second and
    // not the end of a day: a timestamp carrying a fraction of that second
    // sorted above it and the document fell out of a range that names the
    // day it was issued on. A comptable reading a month would have been
    // handed a file missing the last sale of the month, with nothing on the
    // page to say so.
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre", 11_000);
    let last_second = NaiveDate::from_ymd_opt(2026, 9, 12)
        .unwrap()
        .and_hms_milli_opt(23, 59, 59, 400)
        .unwrap();
    sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![NewSaleLine {
                product_id: p,
                qty_milli: 1_000,
                unit_price: None,
                line_discount: Money::ZERO,
            }],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(100_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(last_second),
        },
    )
    .unwrap();

    let closing = export::sales(
        &mut conn,
        SHOP,
        Lang::Fr,
        DayRange {
            from: NaiveDate::from_ymd_opt(2026, 9, 1),
            to: NaiveDate::from_ymd_opt(2026, 9, 12),
        },
    )
    .unwrap();
    assert_eq!(
        sheet(&closing, "Ventes").len(),
        2,
        "the last sale of the closing day is not in the range that names it"
    );

    // And the day after it is still outside: half-open at the top, not one
    // day wider.
    let day_before = export::sales(
        &mut conn,
        SHOP,
        Lang::Fr,
        DayRange {
            from: NaiveDate::from_ymd_opt(2026, 9, 1),
            to: NaiveDate::from_ymd_opt(2026, 9, 11),
        },
    )
    .unwrap();
    assert_eq!(sheet(&day_before, "Ventes").len(), 1, "header only");
}

#[test]
fn the_customers_workbook_carries_the_fiche_and_the_balance_the_ledger_sums_to() {
    let (_dir, mut conn) = open_temp();
    seed_second_shop(&mut conn);
    customers::create(
        &mut conn,
        SHOP,
        OWNER,
        NewCustomer {
            name: "Entreprise Amrani".to_string(),
            party_kind: PartyKind::Company,
            phone: Some("0555 12 34 56".to_string()),
            address: Some("7 rue Larbi Ben M'hidi".to_string()),
            rc: Some("16/00-7654321 B 25".to_string()),
            nif: None,
            nis: Some("000216007654321 00".to_string()),
            ai: None,
            credit_limit: Some(Money::centimes(500_000)),
            warn_threshold: None,
            notes: None,
            active: true,
        },
        Some(Money::centimes(250_000)),
    )
    .unwrap();

    let bytes = export::customers(&mut conn, SHOP, Lang::Fr).unwrap();
    let rows = sheet(&bytes, "Clients");

    assert_eq!(rows.len(), 2, "the neighbour's customer is not in it");
    assert_eq!(text(&rows, 1, "name"), "Entreprise Amrani");
    assert_eq!(text(&rows, 1, "party_kind"), "company");
    assert_eq!(text(&rows, 1, "nis"), "000216007654321 00");
    assert_eq!(text(&rows, 1, "nif"), "");
    assert_eq!(centimes(&rows, 1, "credit_limit_da"), 500_000);
    // The opening debt is a ledger row, so this is the ledger read back.
    assert_eq!(centimes(&rows, 1, "balance_da"), 250_000);
}

#[test]
fn the_suppliers_workbook_carries_the_fiche_and_the_balance_the_ledger_sums_to() {
    let (_dir, mut conn) = open_temp();
    seed_second_shop(&mut conn);
    suppliers::create(
        &mut conn,
        SHOP,
        OWNER,
        NewSupplier {
            name: "Grossiste Boudjemaa".to_string(),
            phone: Some("0770 00 11 22".to_string()),
            address: None,
            rc: None,
            nif: None,
            nis: None,
            ai: None,
            notes: Some("Livraison le mardi".to_string()),
            active: true,
        },
        Some(Money::centimes(180_000)),
    )
    .unwrap();

    let bytes = export::suppliers(&mut conn, SHOP, Lang::Fr).unwrap();
    let rows = sheet(&bytes, "Fournisseurs");

    assert_eq!(rows.len(), 2, "the neighbour's supplier is not in it");
    assert_eq!(text(&rows, 1, "name"), "Grossiste Boudjemaa");
    assert_eq!(text(&rows, 1, "phone"), "0770 00 11 22");
    assert_eq!(text(&rows, 1, "notes"), "Livraison le mardi");
    assert_eq!(centimes(&rows, 1, "balance_da"), 180_000);
    assert_eq!(cell(&rows, 1, "active"), Data::Bool(true));
}

#[test]
fn an_amount_below_zero_keeps_its_sign_and_its_centimes() {
    // A supplier the shop has paid ahead holds a balance the wrong way
    // round, and a workbook that dropped the minus would report money owed.
    // An opening balance cannot be negative (the service refuses it), so the
    // advance is a payment row, which is how one really happens.
    let (_dir, mut conn) = open_temp();
    let id = common::a_supplier(&mut conn, "Avance");
    common::a_supplier_payment_row(&mut conn, id, 5);

    let bytes = export::suppliers(&mut conn, SHOP, Lang::Fr).unwrap();
    let rows = sheet(&bytes, "Fournisseurs");
    assert_eq!(centimes(&rows, 1, "balance_da"), -5);
}
