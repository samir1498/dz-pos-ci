// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The till's write, against a real temp SQLite file. Every rule of
//! features.md §1 (Sale) and §3 that the sale service owns is asserted on the
//! stored document and on the ledger, never on a return value alone.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::models::shop::StoreBlock;
use dzpos_core::models::stock::MovementKind;
use dzpos_core::money::{Bps, Money, PaymentMode, Regime};
use dzpos_core::services::documents::DocumentKind;
use dzpos_core::services::sales::{self, NewSale, NewSaleLine};
use dzpos_core::services::{documents, products, settings, shops, stock};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let conn = dzpos_core::db::open(&path).unwrap();
    (dir, conn)
}

fn at(day: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, day)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

/// A product priced in centimes with the rate the caller names.
fn product(
    conn: &mut SqliteConnection,
    name: &str,
    selling: i64,
    rate_bps: u32,
    unit: Unit,
) -> i32 {
    products::create(
        conn,
        SHOP,
        OWNER,
        NewProduct {
            name: name.to_string(),
            barcode: None,
            category_id: None,
            unit,
            cost: Money::centimes(selling / 2),
            selling: Money::centimes(selling),
            wholesale: None,
            qty_on_hand_milli: 10_000,
            low_stock_at_milli: 0,
            rate_bps: Some(Bps::new(rate_bps).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id
}

fn line(product_id: i32, qty_milli: i64) -> NewSaleLine {
    NewSaleLine {
        product_id,
        qty_milli,
        unit_price: None,
        line_discount: Money::ZERO,
    }
}

fn cash(lines: Vec<NewSaleLine>, tendered: i64) -> NewSale {
    NewSale {
        lines,
        global_discount: Money::ZERO,
        payment_mode: PaymentMode::Cash,
        tendered: Some(Money::centimes(tendered)),
        issued_at: Some(at(9)),
    }
}

#[test]
fn a_cash_sale_issues_a_numbered_ticket_with_its_totals_and_its_change() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 11_000, 1900, Unit::Piece);
    let doc = sales::issue(&mut conn, SHOP, OWNER, cash(vec![line(p, 2_000)], 30_000)).unwrap();

    assert_eq!(doc.kind, DocumentKind::Ticket);
    assert_eq!(doc.number, 1);
    assert_eq!(doc.series, "doc_ticket");
    assert_eq!(doc.user_id, OWNER);
    assert_eq!(doc.issued_at, at(9));
    // 110,00 DA twice is 220,00 HT; 19 % of it is 41,80; the TTC is 261,80,
    // under the 300,00 stamp floor, so nothing is due.
    assert_eq!(doc.totals.total_ht, Money::centimes(22_000));
    assert_eq!(doc.totals.tva, Money::centimes(4_180));
    assert_eq!(doc.totals.total_ttc, Money::centimes(26_180));
    assert_eq!(doc.totals.stamp, Money::ZERO);
    assert_eq!(doc.totals.net_to_pay, Money::centimes(26_180));
    assert_eq!(doc.tendered, Some(Money::centimes(30_000)));
    assert_eq!(doc.change, Some(Money::centimes(3_820)));
    assert_eq!(doc.totals.tva_by_rate.len(), 1);

    // Read back, not trusted from the return value.
    let stored = documents::get(&mut conn, SHOP, doc.id).unwrap();
    assert_eq!(stored, doc);
}

#[test]
fn a_weighed_line_is_rounded_once_and_the_document_holds_that_total() {
    // line_total_fractional_qty: 1,5 kg at 3,33 DA is 4,995 DA, and the line
    // is worth 5,00 rounded away from zero, not 4,99.
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Farine", 333, 900, Unit::Kg);
    let doc = sales::issue(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_500)], 1_000)).unwrap();
    assert_eq!(doc.lines[0].qty_milli, 1_500);
    assert_eq!(doc.lines[0].line_total, Money::centimes(500));
    assert_eq!(doc.totals.total_ht, Money::centimes(500));
}

#[test]
fn the_stock_leaves_line_by_line_and_names_the_document() {
    let (_dir, mut conn) = open_temp();
    let a = product(&mut conn, "Sucre", 11_000, 1900, Unit::Piece);
    let b = product(&mut conn, "Farine", 7_000, 900, Unit::Kg);
    let doc = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        cash(vec![line(a, 2_000), line(b, 1_500)], 100_000),
    )
    .unwrap();

    let moves = stock::list_for_product(&mut conn, SHOP, a).unwrap();
    assert_eq!(moves.len(), 2, "the opening movement and the sale");
    let sale = moves.last().unwrap();
    assert_eq!(sale.kind, MovementKind::Sale);
    assert_eq!(sale.qty_milli, -2_000);
    assert_eq!(sale.document_id, Some(doc.id));
    assert_eq!(sale.unit_cost, Money::centimes(5_500));
    assert_eq!(sale.user_id, OWNER);

    assert_eq!(
        products::get(&mut conn, SHOP, a).unwrap().qty_on_hand_milli,
        8_000
    );
    assert_eq!(
        products::get(&mut conn, SHOP, b).unwrap().qty_on_hand_milli,
        8_500
    );
    assert!(stock::rederive(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn a_sale_beyond_the_count_goes_through_and_leaves_the_stock_negative() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 1_000, 1900, Unit::Piece);
    sales::issue(&mut conn, SHOP, OWNER, cash(vec![line(p, 12_000)], 20_000)).unwrap();
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        -2_000
    );
}

#[test]
fn an_unknown_product_is_not_found_and_leaves_nothing_behind() {
    let (_dir, mut conn) = open_temp();
    let err =
        sales::issue(&mut conn, SHOP, OWNER, cash(vec![line(404, 1_000)], 1_000)).unwrap_err();
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
    assert!(documents::list(&mut conn, SHOP, None).unwrap().is_empty());
}

#[test]
fn a_product_of_another_shop_is_not_found() {
    // Rule 3: the sale reads the product through the scoped service, so
    // another shop's id is a 404 and never a line on this shop's ticket.
    let (_dir, mut conn) = open_temp();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO products (shop_id, name, unit, cost_centimes, selling_centimes, \
         qty_on_hand_milli, low_stock_at_milli, rate_bps) \
         VALUES (2, 'Ailleurs', 'piece', 0, 1000, 5000, 0, 1900)",
    )
    .execute(&mut conn)
    .unwrap();
    let err = sales::issue(&mut conn, SHOP, OWNER, cash(vec![line(1, 1_000)], 10_000)).unwrap_err();
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
}

#[test]
fn an_inactive_product_cannot_be_sold() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 1_000, 1900, Unit::Piece);
    let mut off = NewProduct {
        name: "Sucre".to_string(),
        barcode: None,
        category_id: None,
        unit: Unit::Piece,
        cost: Money::centimes(500),
        selling: Money::centimes(1_000),
        wholesale: None,
        qty_on_hand_milli: 0,
        low_stock_at_milli: 0,
        rate_bps: Some(Bps::new(1900).unwrap()),
        active: true,
    };
    off.active = false;
    products::update(&mut conn, SHOP, OWNER, p, off).unwrap();
    let err = sales::issue(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 10_000)).unwrap_err();
    assert_eq!(err.code(), "validation", "{err:?}");
}

#[test]
fn a_quantity_at_or_below_zero_is_refused() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 1_000, 1900, Unit::Piece);
    for qty in [0, -1_000] {
        let err =
            sales::issue(&mut conn, SHOP, OWNER, cash(vec![line(p, qty)], 10_000)).unwrap_err();
        assert_eq!(err.code(), "validation", "{qty} was accepted");
    }
}

#[test]
fn an_empty_basket_is_refused() {
    let (_dir, mut conn) = open_temp();
    let err = sales::issue(&mut conn, SHOP, OWNER, cash(vec![], 0)).unwrap_err();
    assert_eq!(err.code(), "validation", "{err:?}");
}

#[test]
fn a_credit_sale_is_refused_until_customers_exist() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 1_000, 1900, Unit::Piece);
    let err = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(p, 1_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Credit,
            tendered: None,
            issued_at: Some(at(9)),
        },
    )
    .unwrap_err();
    assert_eq!(err.code(), "validation", "{err:?}");
    assert!(documents::list(&mut conn, SHOP, None).unwrap().is_empty());
}

#[test]
fn cash_short_of_the_amount_to_pay_is_refused_and_nothing_is_written() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 11_000, 1900, Unit::Piece);
    let err = sales::issue(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 100)).unwrap_err();
    assert_eq!(err.code(), "validation", "{err:?}");
    assert!(documents::list(&mut conn, SHOP, None).unwrap().is_empty());
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        10_000,
        "a refused sale moved stock"
    );
}

#[test]
fn cash_with_nothing_tendered_is_refused() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 1_000, 1900, Unit::Piece);
    let err = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(p, 1_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Cash,
            tendered: None,
            issued_at: Some(at(9)),
        },
    )
    .unwrap_err();
    assert_eq!(err.code(), "validation", "{err:?}");
}

#[test]
fn a_card_sale_records_no_tendered_and_carries_no_stamp() {
    // The stamp is a cash tax; a card payment is the electronic exemption
    // (stamp_progressive_tranches).
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Télévision", 5_000_000, 1900, Unit::Piece);
    let doc = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(p, 1_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Card,
            tendered: None,
            issued_at: Some(at(9)),
        },
    )
    .unwrap();
    assert_eq!(doc.payment_mode, PaymentMode::Card);
    assert_eq!(doc.totals.stamp, Money::ZERO);
    assert_eq!(doc.tendered, None);
    assert_eq!(doc.change, None);
}

#[test]
fn a_cash_sale_above_the_floor_carries_the_stamp_in_the_net_to_pay() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Huile", 100_000, 0, Unit::Piece);
    // Ten units of 1 000,00 DA, exempt of TVA: 10 000,00 DA is a hundred
    // tranches of 100,00 DA at 1,00 DA each, so the stamp is 100,00 DA.
    let doc = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        cash(vec![line(p, 10_000)], 2_000_000),
    )
    .unwrap();
    assert_eq!(doc.totals.total_ttc, Money::centimes(1_000_000));
    assert_eq!(doc.totals.stamp, Money::centimes(10_000));
    assert_eq!(doc.totals.net_to_pay, Money::centimes(1_010_000));
    assert_eq!(doc.change, Some(Money::centimes(990_000)));
}

#[test]
fn a_card_sale_that_sends_an_amount_tendered_is_refused() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 1_000, 1900, Unit::Piece);
    let err = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(p, 1_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Card,
            tendered: Some(Money::centimes(1_000)),
            issued_at: Some(at(9)),
        },
    )
    .unwrap_err();
    assert_eq!(err.code(), "validation", "{err:?}");
}

#[test]
fn a_discount_the_basket_cannot_carry_is_a_validation_error_not_a_money_fault() {
    // A MoneyError leaves the API as a 500: the stored file would be at
    // fault. These are the caller's numbers, so they are named and refused
    // here, before compute_totals ever sees them.
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 1_000, 1900, Unit::Piece);
    let too_big = NewSale {
        lines: vec![line(p, 1_000)],
        global_discount: Money::centimes(999_999),
        payment_mode: PaymentMode::Cash,
        tendered: Some(Money::centimes(1_000)),
        issued_at: Some(at(9)),
    };
    assert_eq!(
        sales::issue(&mut conn, SHOP, OWNER, too_big)
            .unwrap_err()
            .code(),
        "validation"
    );

    let negative = NewSale {
        lines: vec![line(p, 1_000)],
        global_discount: Money::centimes(-1),
        payment_mode: PaymentMode::Cash,
        tendered: Some(Money::centimes(1_000)),
        issued_at: Some(at(9)),
    };
    assert_eq!(
        sales::issue(&mut conn, SHOP, OWNER, negative)
            .unwrap_err()
            .code(),
        "validation"
    );

    let line_too_big = NewSale {
        lines: vec![NewSaleLine {
            product_id: p,
            qty_milli: 1_000,
            unit_price: None,
            line_discount: Money::centimes(9_999),
        }],
        global_discount: Money::ZERO,
        payment_mode: PaymentMode::Cash,
        tendered: Some(Money::centimes(1_000)),
        issued_at: Some(at(9)),
    };
    assert_eq!(
        sales::issue(&mut conn, SHOP, OWNER, line_too_big)
            .unwrap_err()
            .code(),
        "validation"
    );
}

#[test]
fn a_discount_spreads_over_the_rates_and_the_ticket_keeps_the_recap() {
    let (_dir, mut conn) = open_temp();
    let a = product(&mut conn, "Sucre", 10_000, 1900, Unit::Piece);
    let b = product(&mut conn, "Lait", 10_000, 900, Unit::Piece);
    let doc = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(a, 1_000), line(b, 1_000)],
            global_discount: Money::centimes(2_000),
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(50_000)),
            issued_at: Some(at(9)),
        },
    )
    .unwrap();
    assert_eq!(doc.totals.total_ht, Money::centimes(20_000));
    assert_eq!(doc.totals.discount, Money::centimes(2_000));
    assert_eq!(doc.totals.subtotal_ht, Money::centimes(18_000));
    // Half the discount to each group: 9 000 at 9 % is 810, at 19 % is 1 710.
    let rates: Vec<(u32, i64, i64)> = doc
        .totals
        .tva_by_rate
        .iter()
        .map(|r| {
            (
                r.rate.as_u32(),
                r.base.as_centimes(),
                r.amount.as_centimes(),
            )
        })
        .collect();
    assert_eq!(rates, vec![(900, 9_000, 810), (1900, 9_000, 1_710)]);
    assert_eq!(doc.totals.tva, Money::centimes(2_520));
}

#[test]
fn the_unit_price_can_be_overridden_at_the_till() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 11_000, 1900, Unit::Piece);
    let doc = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![NewSaleLine {
                product_id: p,
                qty_milli: 1_000,
                unit_price: Some(Money::centimes(9_000)),
                line_discount: Money::ZERO,
            }],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(20_000)),
            issued_at: Some(at(9)),
        },
    )
    .unwrap();
    assert_eq!(doc.lines[0].unit_price, Money::centimes(9_000));
    assert_eq!(doc.totals.total_ht, Money::centimes(9_000));
}

#[test]
fn the_document_keeps_the_regime_in_force_on_the_day_it_was_issued() {
    // A régime row can be dated into the past (T2 review). A ticket issued
    // before the change keeps réel even after the shop moves to the IFU.
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 10_000, 1900, Unit::Piece);
    let before = sales::issue(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 20_000)).unwrap();
    assert_eq!(before.regime, Regime::Reel);
    assert!(!before.totals.tva_by_rate.is_empty());

    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, at(10)).unwrap();
    let mut later = cash(vec![line(p, 1_000)], 20_000);
    later.issued_at = Some(at(11));
    let after = sales::issue(&mut conn, SHOP, OWNER, later).unwrap();
    assert_eq!(after.regime, Regime::Ifu);
    assert_eq!(after.totals.tva, Money::ZERO);
    assert!(
        after.totals.tva_by_rate.is_empty(),
        "the IFU prints no TVA line"
    );
    // The earlier ticket did not change with the shop.
    assert_eq!(
        documents::get(&mut conn, SHOP, before.id).unwrap().regime,
        Regime::Reel
    );
}

#[test]
fn the_seller_block_is_a_snapshot_a_later_rename_does_not_reach() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 10_000, 1900, Unit::Piece);
    shops::update_store(
        &mut conn,
        SHOP,
        OWNER,
        StoreBlock {
            name: "Supérette El Bahdja".to_string(),
            rc: Some("16/00-1234567 B 21".to_string()),
            nif: Some("000216001234567".to_string()),
            nis: Some("098216001234567".to_string()),
            ai: Some("16123456789".to_string()),
            address: Some("Rue Didouche Mourad, Alger".to_string()),
            phone: Some("021 00 00 00".to_string()),
        },
    )
    .unwrap();
    let doc = sales::issue(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 20_000)).unwrap();
    assert_eq!(doc.seller.name, "Supérette El Bahdja");
    assert_eq!(doc.seller.nis.as_deref(), Some("098216001234567"));

    shops::update_store(
        &mut conn,
        SHOP,
        OWNER,
        StoreBlock {
            name: "Autre nom".to_string(),
            rc: None,
            nif: None,
            nis: None,
            ai: None,
            address: None,
            phone: None,
        },
    )
    .unwrap();
    let stored = documents::get(&mut conn, SHOP, doc.id).unwrap();
    assert_eq!(stored.seller.name, "Supérette El Bahdja");
    assert_eq!(stored.seller.nis.as_deref(), Some("098216001234567"));
}

#[test]
fn two_sales_number_one_after_the_other() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 1_000, 1900, Unit::Piece);
    let first = sales::issue(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 10_000)).unwrap();
    let second = sales::issue(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 10_000)).unwrap();
    assert_eq!((first.number, second.number), (1, 2));
}
