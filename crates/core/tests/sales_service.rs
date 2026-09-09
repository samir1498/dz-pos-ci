// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The till's write, against a real temp SQLite file. Every rule of
//! features.md §1 (Sale) and §3 that the sale service owns is asserted on the
//! stored document and on the ledger, never on a return value alone.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::{CoreError, PartySide};
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::models::shop::StoreBlock;
use dzpos_core::models::stock::MovementKind;
use dzpos_core::money::{Bps, Money, PaymentMode, Regime};
use dzpos_core::services::customers::{NewCustomer, PartyKind};
use dzpos_core::services::debt::DebtKind;
use dzpos_core::services::documents::{Document, DocumentKind};
use dzpos_core::services::sales::{self, NewSale, NewSaleLine, SaleKind};
use dzpos_core::services::{audit, customers, debt, documents, products, settings, shops, stock};

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

/// The document a sale left behind. Most of this file is about the stored
/// document, so the warning a sale may also answer with is asserted in the
/// credit tests that are about it and dropped here.
fn issue_sale(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    new: NewSale,
) -> Result<Document, CoreError> {
    sales::issue(conn, shop_id, user_id, new).map(|s| s.document)
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
        customer_id: None,
        override_credit: false,
        kind: SaleKind::Ticket,
        issued_at: Some(at(9)),
    }
}

#[test]
fn a_cash_sale_issues_a_numbered_ticket_with_its_totals_and_its_change() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 11_000, 1900, Unit::Piece);
    let doc = issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(p, 2_000)], 30_000)).unwrap();

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
    let doc = issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_500)], 1_000)).unwrap();
    assert_eq!(doc.lines[0].qty_milli, 1_500);
    assert_eq!(doc.lines[0].line_total, Money::centimes(500));
    assert_eq!(doc.totals.total_ht, Money::centimes(500));
}

#[test]
fn the_stock_leaves_line_by_line_and_names_the_document() {
    let (_dir, mut conn) = open_temp();
    let a = product(&mut conn, "Sucre", 11_000, 1900, Unit::Piece);
    let b = product(&mut conn, "Farine", 7_000, 900, Unit::Kg);
    let doc = issue_sale(
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
    issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(p, 12_000)], 20_000)).unwrap();
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        -2_000
    );
}

#[test]
fn an_unknown_product_is_not_found_and_leaves_nothing_behind() {
    let (_dir, mut conn) = open_temp();
    let err = issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(404, 1_000)], 1_000)).unwrap_err();
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
    let err = issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(1, 1_000)], 10_000)).unwrap_err();
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
    let err = issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 10_000)).unwrap_err();
    assert_eq!(err.code(), "validation", "{err:?}");
}

#[test]
fn a_quantity_at_or_below_zero_is_refused() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 1_000, 1900, Unit::Piece);
    for qty in [0, -1_000] {
        let err = issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(p, qty)], 10_000)).unwrap_err();
        assert_eq!(err.code(), "validation", "{qty} was accepted");
    }
}

#[test]
fn an_empty_basket_is_refused() {
    let (_dir, mut conn) = open_temp();
    let err = issue_sale(&mut conn, SHOP, OWNER, cash(vec![], 0)).unwrap_err();
    assert_eq!(err.code(), "validation", "{err:?}");
}

#[test]
fn a_credit_sale_is_refused_until_customers_exist() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 1_000, 1900, Unit::Piece);
    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(p, 1_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Credit,
            tendered: None,
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
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
    let err = issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 100)).unwrap_err();
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
    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(p, 1_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Cash,
            tendered: None,
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
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
    let doc = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(p, 1_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Card,
            tendered: None,
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
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
    let doc = issue_sale(
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
    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(p, 1_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Card,
            tendered: Some(Money::centimes(1_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
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
        customer_id: None,
        override_credit: false,
        kind: SaleKind::Ticket,
        issued_at: Some(at(9)),
    };
    assert_eq!(
        issue_sale(&mut conn, SHOP, OWNER, too_big)
            .unwrap_err()
            .code(),
        "validation"
    );

    let negative = NewSale {
        lines: vec![line(p, 1_000)],
        global_discount: Money::centimes(-1),
        payment_mode: PaymentMode::Cash,
        tendered: Some(Money::centimes(1_000)),
        customer_id: None,
        override_credit: false,
        kind: SaleKind::Ticket,
        issued_at: Some(at(9)),
    };
    assert_eq!(
        issue_sale(&mut conn, SHOP, OWNER, negative)
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
        customer_id: None,
        override_credit: false,
        kind: SaleKind::Ticket,
        issued_at: Some(at(9)),
    };
    assert_eq!(
        issue_sale(&mut conn, SHOP, OWNER, line_too_big)
            .unwrap_err()
            .code(),
        "validation"
    );
}

#[test]
fn a_price_times_a_quantity_that_does_not_fit_names_the_field_it_came_from() {
    // Both numbers are inside what the API's edge admits (2^53 - 1), so
    // nothing before the service refuses them; their product is past i64
    // centimes. That is arithmetic the caller asked for, not a stored-file
    // fault, so the field is named and the code is validation, never money.
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 1_000, 1900, Unit::Kg);
    let huge = NewSale {
        lines: vec![NewSaleLine {
            product_id: p,
            qty_milli: (1 << 53) - 1,
            unit_price: Some(Money::centimes((1 << 53) - 1)),
            line_discount: Money::ZERO,
        }],
        global_discount: Money::ZERO,
        payment_mode: PaymentMode::Cash,
        tendered: Some(Money::centimes((1 << 53) - 1)),
        customer_id: None,
        override_credit: false,
        kind: SaleKind::Ticket,
        issued_at: Some(at(9)),
    };
    match issue_sale(&mut conn, SHOP, OWNER, huge).unwrap_err() {
        CoreError::Validation { field, .. } => assert_eq!(field, "qty_milli"),
        other => panic!("expected a validation error, got {other:?}"),
    }
    assert_eq!(documents::list(&mut conn, SHOP, None).unwrap().len(), 0);
}

#[test]
fn a_discount_spreads_over_the_rates_and_the_ticket_keeps_the_recap() {
    let (_dir, mut conn) = open_temp();
    let a = product(&mut conn, "Sucre", 10_000, 1900, Unit::Piece);
    let b = product(&mut conn, "Lait", 10_000, 900, Unit::Piece);
    let doc = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(a, 1_000), line(b, 1_000)],
            global_discount: Money::centimes(2_000),
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(50_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
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
    let doc = issue_sale(
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
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
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
    let before = issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 20_000)).unwrap();
    assert_eq!(before.regime, Regime::Reel);
    assert!(!before.totals.tva_by_rate.is_empty());

    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, at(10)).unwrap();
    let mut later = cash(vec![line(p, 1_000)], 20_000);
    later.issued_at = Some(at(11));
    let after = issue_sale(&mut conn, SHOP, OWNER, later).unwrap();
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
fn an_ifu_line_stores_no_rate_so_a_reprint_never_needs_the_regime() {
    // regime_ifu_prints_no_tva: under the IFU the price is a single price and
    // the document mentions no TVA at all. If the line kept the product's
    // 19 % the stored document would not say so, and the first renderer that
    // printed a rate per line would put TVA on an IFU ticket.
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 10_000, 1900, Unit::Piece);
    let reel = issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 20_000)).unwrap();
    assert_eq!(reel.lines[0].rate_bps, Bps::new(1900).unwrap());

    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, at(10)).unwrap();
    let mut later = cash(vec![line(p, 1_000)], 20_000);
    later.issued_at = Some(at(11));
    let ifu = issue_sale(&mut conn, SHOP, OWNER, later).unwrap();
    // Read back, not the return value: what is on the paper is what is stored.
    let read = documents::get(&mut conn, SHOP, ifu.id).unwrap();
    assert_eq!(read.regime, Regime::Ifu);
    for stored in &read.lines {
        assert_eq!(
            stored.rate_bps,
            Bps::new(0).unwrap(),
            "an IFU line kept a TVA rate"
        );
    }
    assert!(read.totals.tva_by_rate.is_empty());
    assert_eq!(read.totals.tva, Money::ZERO);
    // The réel ticket issued before the change is untouched.
    assert_eq!(
        documents::get(&mut conn, SHOP, reel.id).unwrap().lines[0].rate_bps,
        Bps::new(1900).unwrap()
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
    let doc = issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 20_000)).unwrap();
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
    let first = issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 10_000)).unwrap();
    let second = issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 10_000)).unwrap();
    assert_eq!((first.number, second.number), (1, 2));
}

// The sale on credit (features.md §1 and §2). Every rule is asserted on
// what the file holds afterwards: the document, its balance triple, the
// customer's ledger and the audit log, never on the return value alone.

/// A fiche with the limit and the threshold the test is about. A `None`
/// limit is no limit at all and a zero one is no credit at all, which is
/// two different answers and why both are passed through as they are.
fn customer(
    conn: &mut SqliteConnection,
    name: &str,
    credit_limit: Option<i64>,
    warn_threshold: Option<i64>,
) -> i32 {
    customers::create(
        conn,
        SHOP,
        OWNER,
        NewCustomer {
            name: name.to_string(),
            party_kind: PartyKind::Company,
            phone: Some("0555 00 11 22".to_string()),
            address: Some("Rue Larbi Ben M'hidi, Alger".to_string()),
            rc: Some("16/00-7654321 B 25".to_string()),
            nif: Some("000216007654321".to_string()),
            nis: None,
            ai: None,
            credit_limit: credit_limit.map(Money::centimes),
            warn_threshold: warn_threshold.map(Money::centimes),
            notes: None,
            active: true,
        },
        None,
    )
    .unwrap()
    .id
}

/// Closes a fiche, the way the customers screen does: the fields as they
/// were, `active` off.
fn close(conn: &mut SqliteConnection, id: i32) {
    let open = customers::get(conn, SHOP, id).unwrap();
    customers::update(
        conn,
        SHOP,
        OWNER,
        id,
        NewCustomer {
            name: open.name,
            party_kind: open.party_kind,
            phone: open.phone,
            address: open.address,
            rc: open.rc,
            nif: open.nif,
            nis: open.nis,
            ai: open.ai,
            credit_limit: open.credit_limit,
            warn_threshold: open.warn_threshold,
            notes: open.notes,
            active: false,
        },
    )
    .unwrap();
}

/// A basket sold on credit to `customer_id`. `override_credit` is the
/// owner's decision to pass the limit.
fn credit(customer_id: i32, lines: Vec<NewSaleLine>, override_credit: bool) -> NewSale {
    NewSale {
        lines,
        global_discount: Money::ZERO,
        payment_mode: PaymentMode::Credit,
        tendered: None,
        customer_id: Some(customer_id),
        override_credit,
        kind: SaleKind::Ticket,
        issued_at: Some(at(9)),
    }
}

#[test]
fn a_credit_sale_with_no_customer_is_refused_and_writes_nothing() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sucre", 1_000, 1900, Unit::Piece);
    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(p, 1_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Credit,
            tendered: None,
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(at(9)),
        },
    )
    .unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "customer_id"),
        "{err:?}"
    );
    assert!(documents::list(&mut conn, SHOP, None).unwrap().is_empty());
}

#[test]
fn a_credit_sale_writes_the_document_its_debt_row_and_the_balance_triple() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 1900, Unit::Piece);
    let c = customer(&mut conn, "Entreprise Amrani", Some(1_000_000), None);
    let sale = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        credit(c, vec![line(p, 2_000)], false),
    )
    .unwrap();
    let doc = sale.document;

    // Sold on credit, so nothing was tendered and no stamp is due: the
    // droit de timbre is a cash tax (stamp_progressive_tranches).
    assert_eq!(doc.kind, DocumentKind::Ticket);
    assert_eq!(doc.payment_mode, PaymentMode::Credit);
    assert_eq!(doc.totals.stamp, Money::ZERO);
    assert_eq!(doc.tendered, None);
    assert_eq!(doc.change, None);
    assert_eq!(sale.warning, None, "no threshold, no warning");

    // The buyer block is a snapshot of the fiche, party kind included, on a
    // ticket as much as on a facture.
    let buyer = doc
        .buyer
        .clone()
        .expect("a named customer has a buyer block");
    assert_eq!(buyer.name, "Entreprise Amrani");
    assert_eq!(buyer.party_kind, PartyKind::Company);
    assert_eq!(buyer.rc.as_deref(), Some("16/00-7654321 B 25"));
    assert_eq!(buyer.nif.as_deref(), Some("000216007654321"));
    assert_eq!(doc.customer_id, Some(c));

    // The triple: nothing owed before, this whole document owed now.
    let net = doc.totals.net_to_pay;
    let balance = doc.balance.expect("a credit sale stores its balance");
    assert_eq!(balance.old_balance, Money::ZERO);
    assert_eq!(balance.remaining_debt, net);
    assert_eq!(balance.total_debt, net);

    // One movement, naming the document it came from.
    let ledger = debt::ledger(&mut conn, SHOP, c).unwrap();
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].kind, DebtKind::Sale);
    assert_eq!(ledger[0].debit, net);
    assert_eq!(ledger[0].credit, Money::ZERO);
    assert_eq!(ledger[0].document_id, Some(doc.id));
    assert_eq!(ledger[0].user_id, OWNER);
    assert_eq!(debt::balance(&mut conn, SHOP, c).unwrap(), net);

    // The stock left with it, in the same transaction.
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        8_000
    );
    assert_eq!(documents::get(&mut conn, SHOP, doc.id).unwrap(), doc);
}

#[test]
fn a_credit_sale_past_the_limit_is_refused_whole_and_burns_no_number() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0, Unit::Piece);
    // 500,00 of limit against a 1 000,00 basket.
    let c = customer(&mut conn, "Entreprise Amrani", Some(50_000), None);
    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        credit(c, vec![line(p, 1_000)], false),
    )
    .unwrap_err();

    assert_eq!(err.code(), "credit_limit", "{err:?}");
    let CoreError::CreditLimit {
        balance_after,
        credit_limit,
    } = err
    else {
        panic!("a credit refusal carries the two amounts");
    };
    assert_eq!(balance_after, Money::centimes(100_000));
    assert_eq!(credit_limit, Money::centimes(50_000));

    // Nothing at all: no document, no ledger movement, no stock.
    assert!(documents::list(&mut conn, SHOP, None).unwrap().is_empty());
    assert!(debt::ledger(&mut conn, SHOP, c).unwrap().is_empty());
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        10_000
    );
    // And the number the refused sale would have taken is still the first
    // one: a refusal costs the series nothing (décret 05-468 art. 10).
    let next = issue_sale(&mut conn, SHOP, OWNER, cash(vec![line(p, 1_000)], 200_000)).unwrap();
    assert_eq!(next.number, 1);
}

#[test]
fn the_limit_is_tested_on_the_balance_the_sale_leaves_behind() {
    // 1 000,00 of limit, 600,00 already owed. A 400,00 basket lands exactly
    // on the limit and goes through; the next centime does not, even though
    // each basket on its own is far under the limit.
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sac", 20_000, 0, Unit::Piece);
    let c = customer(&mut conn, "Entreprise Amrani", Some(100_000), None);
    debt::adjust(&mut conn, SHOP, OWNER, c, Money::centimes(60_000), None).unwrap();

    let doc = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        credit(c, vec![line(p, 2_000)], false),
    )
    .unwrap();
    let balance = doc.balance.expect("a credit sale stores its balance");
    assert_eq!(balance.old_balance, Money::centimes(60_000));
    assert_eq!(balance.remaining_debt, Money::centimes(40_000));
    assert_eq!(balance.total_debt, Money::centimes(100_000), "at the limit");

    let cheap = product(&mut conn, "Clou", 1, 0, Unit::Piece);
    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        credit(c, vec![line(cheap, 1_000)], false),
    )
    .unwrap_err();
    assert_eq!(err.code(), "credit_limit", "one centime past: {err:?}");
}

#[test]
fn a_null_limit_never_refuses_and_a_zero_one_refuses_the_first_centime() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 1_000_000, 0, Unit::Piece);
    let cheap = product(&mut conn, "Clou", 1, 0, Unit::Piece);

    let open = customer(&mut conn, "Sans plafond", None, None);
    let doc = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        credit(open, vec![line(p, 5_000)], false),
    )
    .unwrap();
    assert_eq!(doc.totals.net_to_pay, Money::centimes(5_000_000));

    let none = customer(&mut conn, "Aucun crédit", Some(0), None);
    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        credit(none, vec![line(cheap, 1_000)], false),
    )
    .unwrap_err();
    assert_eq!(err.code(), "credit_limit", "{err:?}");
}

#[test]
fn the_warning_fires_at_the_threshold_and_the_sale_still_goes_through() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sac", 10_000, 0, Unit::Piece);
    // 1 000,00 of limit, warn at 400,00.
    let c = customer(&mut conn, "Entreprise Amrani", Some(100_000), Some(40_000));

    // 300,00: under the threshold, nothing said.
    let quiet = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        credit(c, vec![line(p, 3_000)], false),
    )
    .unwrap();
    assert_eq!(quiet.warning, None);

    // 100,00 more lands exactly on 400,00, and the threshold is reached, not
    // passed: a shop that sets one at 400,00 wants to hear about this sale.
    let warned = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        credit(c, vec![line(p, 1_000)], false),
    )
    .unwrap();
    assert_eq!(warned.warning, Some(sales::Warning::NearLimit));
    assert_eq!(warned.warning.map(sales::Warning::code), Some("near_limit"));
    // The warning is informational: the document exists and the debt moved.
    assert_eq!(
        debt::balance(&mut conn, SHOP, c).unwrap(),
        Money::centimes(40_000)
    );
    assert_eq!(documents::list(&mut conn, SHOP, None).unwrap().len(), 2);

    // No threshold at all is no warning, whatever the balance.
    let silent = customer(&mut conn, "Sans seuil", Some(100_000), None);
    let none = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        credit(silent, vec![line(p, 9_000)], false),
    )
    .unwrap();
    assert_eq!(none.warning, None);
}

#[test]
fn a_closed_fiche_is_named_on_no_document_whatever_the_payment_is() {
    // The rule is about the fiche, not about the credit: a fiche somebody
    // closed is not a party a document is made out to, so cash and card are
    // refused on the same field a credit sale is (features.md §1).
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sac", 10_000, 0, Unit::Piece);
    let c = customer(&mut conn, "Entreprise Amrani", None, None);
    close(&mut conn, c);

    for mode in [PaymentMode::Cash, PaymentMode::Card] {
        let sale = NewSale {
            customer_id: Some(c),
            payment_mode: mode,
            tendered: match mode {
                PaymentMode::Cash => Some(Money::centimes(20_000)),
                _ => None,
            },
            ..credit(c, vec![line(p, 1_000)], false)
        };
        let err = issue_sale(&mut conn, SHOP, OWNER, sale).unwrap_err();
        assert!(
            matches!(&err, CoreError::Validation { field, .. } if field == "customer_id"),
            "{mode:?}: {err:?}"
        );
    }
    assert!(documents::list(&mut conn, SHOP, None).unwrap().is_empty());
}

#[test]
fn a_customer_in_credit_may_buy_on_credit_up_to_their_deposit() {
    // The limit is tested on the balance the sale leaves behind, and a
    // customer who has paid ahead has a negative one. A limit of 0 is no
    // credit, but a deposit of 300,00 is money the shop already holds: a
    // 200,00 basket leaves them 100,00 in credit and is not a debt at all.
    // The 400,00 one leaves 100,00 owed against a limit of nothing.
    let (_dir, mut conn) = open_temp();
    let p2 = product(&mut conn, "Sac 200", 20_000, 0, Unit::Piece);
    let p4 = product(&mut conn, "Sac 400", 40_000, 0, Unit::Piece);
    let c = customer(&mut conn, "Entreprise Amrani", Some(0), None);
    // Written straight onto the ledger: an avoir is T6's, and what this test
    // is about is the sign of the balance, not how it got there.
    diesel::sql_query(
        "INSERT INTO debt_ledger (shop_id, customer_id, kind, debit_centimes, \
         credit_centimes, user_id) VALUES (1, ?, 'avoir', 0, 30000, 1)",
    )
    .bind::<diesel::sql_types::Integer, _>(c)
    .execute(&mut conn)
    .unwrap();
    assert_eq!(
        debt::balance(&mut conn, SHOP, c).unwrap(),
        Money::centimes(-30_000)
    );

    let doc = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        credit(c, vec![line(p2, 1_000)], false),
    )
    .unwrap();
    let balance = doc.balance.expect("a credit sale stores its balance");
    assert_eq!(balance.old_balance, Money::centimes(-30_000));
    assert_eq!(balance.remaining_debt, Money::centimes(20_000));
    assert_eq!(balance.total_debt, Money::centimes(-10_000));

    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        credit(c, vec![line(p4, 1_000)], false),
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::CreditLimit {
                balance_after,
                credit_limit,
            } if balance_after == Money::centimes(30_000)
                && credit_limit == Money::ZERO
        ),
        "{err:?}"
    );
}

#[test]
fn an_override_takes_the_sale_past_the_limit_and_the_log_says_who() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0, Unit::Piece);
    let c = customer(&mut conn, "Entreprise Amrani", Some(50_000), None);

    let doc = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        credit(c, vec![line(p, 1_000)], true),
    )
    .unwrap();
    let balance = doc.balance.expect("a credit sale stores its balance");
    assert_eq!(balance.total_debt, Money::centimes(100_000), "past 500,00");
    assert_eq!(
        debt::balance(&mut conn, SHOP, c).unwrap(),
        Money::centimes(100_000)
    );

    let entries = audit::list(&mut conn, SHOP).unwrap();
    let entry = entries
        .iter()
        .find(|e| e.action == "sale.credit_override")
        .expect("an override past a rule is logged");
    assert_eq!(entry.entity, "sale");
    assert_eq!(entry.entity_id, Some(doc.id));
    assert_eq!(entry.user_id, OWNER);
    // `before` is the state the decision was taken against: what the
    // customer owed and what they were allowed to owe.
    let before = entry.before.clone().unwrap_or_default();
    assert!(before.contains("\"balance_centimes\":0"), "{before}");
    assert!(
        before.contains("\"credit_limit_centimes\":50000"),
        "{before}"
    );
    assert!(!before.contains("balance_after_centimes"), "{before}");

    // `after` is what the decision produced: the document, how much of it
    // the customer now owes, where the balance landed, how it was paid and
    // whether the fiche was warning as well as blocking.
    let after = entry.after.clone().unwrap_or_default();
    assert!(
        after.contains(&format!("\"document_id\":{}", doc.id)),
        "{after}"
    );
    assert!(
        after.contains("\"balance_after_centimes\":100000"),
        "{after}"
    );
    assert!(
        after.contains("\"remaining_debt_centimes\":100000"),
        "{after}"
    );
    assert!(after.contains("\"payment_mode\":\"credit\""), "{after}");
    assert!(after.contains("\"warning\":null"), "{after}");
}

#[test]
fn an_override_on_a_fiche_that_was_also_warning_says_so_in_the_log() {
    // The two rules are separate: this fiche blocks at 500,00 and warns at
    // 200,00, so the sale is both overridden and warned, and a comptable
    // reading the row sees the second without opening the fiche.
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0, Unit::Piece);
    let c = customer(&mut conn, "Entreprise Amrani", Some(50_000), Some(20_000));

    let doc = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        credit(c, vec![line(p, 1_000)], true),
    )
    .unwrap();
    let entries = audit::list(&mut conn, SHOP).unwrap();
    let entry = entries
        .iter()
        .find(|e| e.action == "sale.credit_override")
        .expect("an override past a rule is logged");
    assert_eq!(entry.entity_id, Some(doc.id));
    let after = entry.after.clone().unwrap_or_default();
    assert!(after.contains("\"warning\":\"near_limit\""), "{after}");
}

#[test]
fn an_override_on_a_sale_the_limit_would_have_taken_writes_no_log_row() {
    // The log is for the decision, not for the flag: a cashier who leaves the
    // override on for a sale that was inside the limit did not take one.
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sac", 10_000, 0, Unit::Piece);
    let c = customer(&mut conn, "Entreprise Amrani", Some(100_000), None);
    issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        credit(c, vec![line(p, 1_000)], true),
    )
    .unwrap();
    assert!(
        !audit::list(&mut conn, SHOP)
            .unwrap()
            .iter()
            .any(|e| e.action == "sale.credit_override"),
        "a sale inside the limit logged an override nobody took"
    );
}

#[test]
fn a_cash_sale_with_a_customer_names_the_buyer_and_moves_no_debt() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sac", 10_000, 0, Unit::Piece);
    // Zero credit and a threshold of nothing: neither applies to a sale that
    // is paid for on the spot.
    let c = customer(&mut conn, "Entreprise Amrani", Some(0), Some(0));
    let sale = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            customer_id: Some(c),
            ..cash(vec![line(p, 1_000)], 20_000)
        },
    )
    .unwrap();
    let doc = sale.document;

    assert_eq!(sale.warning, None, "a paid sale is never near a limit");
    assert_eq!(doc.customer_id, Some(c));
    assert_eq!(
        doc.buyer.as_ref().map(|b| b.name.as_str()),
        Some("Entreprise Amrani")
    );
    let balance = doc.balance.expect("a named customer stores the triple");
    assert_eq!(balance.old_balance, Money::ZERO);
    assert_eq!(balance.remaining_debt, Money::ZERO, "nothing left unpaid");
    assert_eq!(balance.total_debt, Money::ZERO);
    assert!(
        debt::ledger(&mut conn, SHOP, c).unwrap().is_empty(),
        "a paid sale wrote a debt row"
    );
}

#[test]
fn a_closed_fiche_and_another_shops_fiche_cannot_be_sold_to() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Sac", 10_000, 0, Unit::Piece);
    let c = customer(&mut conn, "Entreprise Amrani", None, None);
    close(&mut conn, c);

    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        credit(c, vec![line(p, 1_000)], false),
    )
    .unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "customer_id"),
        "{err:?}"
    );

    // Rule 3: a fiche of another shop is not this shop's to sell to. The id
    // exists and the row is open for business, so what answers 404 here is
    // the shop filter and nothing else.
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO customers (id, shop_id, name, party_kind, active) \
         VALUES (404, 2, 'Ailleurs', 'company', 1)",
    )
    .execute(&mut conn)
    .unwrap();
    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        credit(404, vec![line(p, 1_000)], false),
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::NotFound {
                entity: "customer",
                ..
            }
        ),
        "{err:?}"
    );
    assert!(documents::list(&mut conn, SHOP, None).unwrap().is_empty());
}

// ---- the facture at the till (features.md §3, `facture_requires_party_ids`)

/// The shop's own settings as a facture needs them. `nis` is handed in so a
/// test can take one identifier away without rewriting the whole block.
fn store_block(rc: Option<&str>, nis: Option<&str>) -> StoreBlock {
    StoreBlock {
        name: "Supérette El Bahdja".to_string(),
        rc: rc.map(str::to_string),
        nif: Some("000216001234567".to_string()),
        nis: nis.map(str::to_string),
        ai: Some("16123456789".to_string()),
        address: Some("Rue Didouche Mourad, Alger".to_string()),
        phone: Some("021 00 00 00".to_string()),
    }
}

/// A seller block that carries what a facture asks of it.
fn seller_ready(conn: &mut SqliteConnection) {
    shops::update_store(
        conn,
        SHOP,
        OWNER,
        store_block(Some("16/00-1234567 B 21"), Some("098216001234567")),
    )
    .unwrap();
}

/// A fiche of the kind the caller names, with only the identifiers it hands
/// over. Everything else is left off on purpose: what refuses a facture is
/// the field that is missing, and a helper filling them all in would hide it.
fn party(
    conn: &mut SqliteConnection,
    name: &str,
    party_kind: PartyKind,
    rc: Option<&str>,
    nis: Option<&str>,
    address: Option<&str>,
) -> i32 {
    customers::create(
        conn,
        SHOP,
        OWNER,
        NewCustomer {
            name: name.to_string(),
            party_kind,
            phone: None,
            address: address.map(str::to_string),
            rc: rc.map(str::to_string),
            nif: None,
            nis: nis.map(str::to_string),
            ai: None,
            credit_limit: None,
            warn_threshold: None,
            notes: None,
            active: true,
        },
        None,
    )
    .unwrap()
    .id
}

/// A basket rung up as a facture, paid the way the caller names.
fn facture(customer_id: i32, lines: Vec<NewSaleLine>, payment_mode: PaymentMode) -> NewSale {
    NewSale {
        lines,
        global_discount: Money::ZERO,
        payment_mode,
        tendered: match payment_mode {
            PaymentMode::Cash => Some(Money::centimes(1_000_000)),
            _ => None,
        },
        customer_id: Some(customer_id),
        override_credit: false,
        kind: SaleKind::Facture,
        issued_at: Some(at(9)),
    }
}

/// What the counter of a series stands at, read from the table the counter
/// writes rather than from a document: a refusal that rolled back leaves no
/// document to read, and the whole claim is that it left no number either.
fn counter(conn: &mut SqliteConnection, series: &str) -> i64 {
    #[derive(QueryableByName)]
    struct Next {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        next_value: i64,
    }
    let rows: Vec<Next> =
        diesel::sql_query("SELECT next_value FROM counters WHERE shop_id = 1 AND name = ?")
            .bind::<diesel::sql_types::Text, _>(series)
            .load(conn)
            .unwrap();
    rows.first().map_or(1, |r| r.next_value)
}

#[test]
fn a_facture_to_a_company_carrying_its_identifiers_is_issued_in_the_facture_series() {
    let (_dir, mut conn) = open_temp();
    seller_ready(&mut conn);
    let p = product(&mut conn, "Ciment", 100_000, 1900, Unit::Piece);
    let c = party(
        &mut conn,
        "Entreprise Amrani",
        PartyKind::Company,
        Some("16/00-7654321 B 22"),
        Some("098216007654321"),
        Some("Zone industrielle, Rouiba"),
    );

    // Credit, to pin the other half of the rule: loi 04-02 art. 10 decides
    // the paper by who the buyer is and never by how they pay.
    let doc = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        facture(c, vec![line(p, 1_000)], PaymentMode::Credit),
    )
    .unwrap();

    assert_eq!(doc.kind, DocumentKind::Facture);
    assert_eq!(doc.series, "doc_facture");
    assert_eq!(doc.number, 1);
    let buyer = doc.buyer.expect("a facture carries a buyer block");
    assert_eq!(buyer.name, "Entreprise Amrani");
    assert_eq!(buyer.rc.as_deref(), Some("16/00-7654321 B 22"));
    assert_eq!(buyer.nis.as_deref(), Some("098216007654321"));
    // The ledger moved, the way it does on any credit sale.
    assert_eq!(
        debt::balance(&mut conn, SHOP, c).unwrap(),
        doc.totals.net_to_pay
    );
}

#[test]
fn a_company_buyer_without_a_nis_refuses_the_facture_and_burns_no_number() {
    let (_dir, mut conn) = open_temp();
    seller_ready(&mut conn);
    let p = product(&mut conn, "Ciment", 100_000, 1900, Unit::Piece);
    let c = party(
        &mut conn,
        "Entreprise Amrani",
        PartyKind::Company,
        Some("16/00-7654321 B 22"),
        None,
        None,
    );

    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        facture(c, vec![line(p, 1_000)], PaymentMode::Cash),
    )
    .unwrap_err();
    match err {
        CoreError::PartyIds { side, ref missing } => {
            assert_eq!(side, PartySide::Buyer);
            assert_eq!(missing, &["nis"]);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(err.code(), "party_ids");

    // Nothing at all happened: no document, no stock movement, and above
    // all no number taken out of either series (features.md, Numbering).
    assert!(documents::list(&mut conn, SHOP, None).unwrap().is_empty());
    assert_eq!(counter(&mut conn, "doc_facture"), 1);
    assert_eq!(counter(&mut conn, "doc_ticket"), 1);

    // The same basket goes through once the fiche carries the identifier,
    // and it is FA number 1: the refusal cost the series nothing.
    customers::update(
        &mut conn,
        SHOP,
        OWNER,
        c,
        NewCustomer {
            name: "Entreprise Amrani".to_string(),
            party_kind: PartyKind::Company,
            phone: None,
            address: None,
            rc: Some("16/00-7654321 B 22".to_string()),
            nif: None,
            nis: Some("098216007654321".to_string()),
            ai: None,
            credit_limit: None,
            warn_threshold: None,
            notes: None,
            active: true,
        },
    )
    .unwrap();
    let doc = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        facture(c, vec![line(p, 1_000)], PaymentMode::Cash),
    )
    .unwrap();
    assert_eq!(doc.number, 1);
    assert_eq!(doc.series, "doc_facture");
}

#[test]
fn a_facture_to_a_consumer_asks_for_a_name_and_an_address_and_nothing_else() {
    let (_dir, mut conn) = open_temp();
    seller_ready(&mut conn);
    let p = product(&mut conn, "Ciment", 100_000, 1900, Unit::Piece);

    // No RC, no NIS: décret 05-468 art. 3-2, last alinéa asks a consumer for
    // « ses nom, prénom(s) et adresse » and stops there.
    let with_address = party(
        &mut conn,
        "Karim Belkacem",
        PartyKind::Consumer,
        None,
        None,
        Some("12 rue des Frères Bouadou, Bir Mourad Raïs"),
    );
    let doc = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        facture(with_address, vec![line(p, 1_000)], PaymentMode::Cash),
    )
    .unwrap();
    assert_eq!(doc.kind, DocumentKind::Facture);

    let no_address = party(
        &mut conn,
        "Yacine Hamdi",
        PartyKind::Consumer,
        None,
        None,
        None,
    );
    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        facture(no_address, vec![line(p, 1_000)], PaymentMode::Cash),
    )
    .unwrap_err();
    match err {
        CoreError::PartyIds { side, ref missing } => {
            assert_eq!(side, PartySide::Buyer);
            assert_eq!(missing, &["address"]);
        }
        other => panic!("{other:?}"),
    }
    // The one facture that went through is still the only one, and the
    // refusal took no second number.
    assert_eq!(counter(&mut conn, "doc_facture"), 2);
}

#[test]
fn a_shop_whose_settings_carry_no_nis_cannot_issue_a_facture_at_all() {
    let (_dir, mut conn) = open_temp();
    shops::update_store(
        &mut conn,
        SHOP,
        OWNER,
        store_block(Some("16/00-1234567 B 21"), None),
    )
    .unwrap();
    let p = product(&mut conn, "Ciment", 100_000, 1900, Unit::Piece);
    let c = party(
        &mut conn,
        "Entreprise Amrani",
        PartyKind::Company,
        Some("16/00-7654321 B 22"),
        Some("098216007654321"),
        None,
    );

    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        facture(c, vec![line(p, 1_000)], PaymentMode::Cash),
    )
    .unwrap_err();
    match err {
        CoreError::PartyIds { side, ref missing } => {
            assert_eq!(side, PartySide::Seller);
            assert_eq!(missing, &["nis"]);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(counter(&mut conn, "doc_facture"), 1);

    // The same shop still rings up tickets: the identifiers are checked when
    // a facture is issued, not when the settings are saved.
    let doc = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        cash(vec![line(p, 1_000)], 1_000_000),
    )
    .unwrap();
    assert_eq!(doc.kind, DocumentKind::Ticket);
}

#[test]
fn a_facture_with_no_customer_is_refused_on_the_customer_field() {
    let (_dir, mut conn) = open_temp();
    seller_ready(&mut conn);
    let p = product(&mut conn, "Ciment", 100_000, 1900, Unit::Piece);
    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(p, 1_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(1_000_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Facture,
            issued_at: Some(at(9)),
        },
    )
    .unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "customer_id"),
        "{err:?}"
    );
    assert!(documents::list(&mut conn, SHOP, None).unwrap().is_empty());
    assert_eq!(counter(&mut conn, "doc_facture"), 1);
}

#[test]
fn a_ticket_and_a_facture_run_two_series_that_do_not_touch() {
    let (_dir, mut conn) = open_temp();
    seller_ready(&mut conn);
    let p = product(&mut conn, "Ciment", 100_000, 1900, Unit::Piece);
    let c = party(
        &mut conn,
        "Entreprise Amrani",
        PartyKind::Company,
        Some("16/00-7654321 B 22"),
        Some("098216007654321"),
        None,
    );

    let ticket = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        cash(vec![line(p, 1_000)], 1_000_000),
    )
    .unwrap();
    assert_eq!((ticket.series.as_str(), ticket.number), ("doc_ticket", 1));

    let invoice = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        facture(c, vec![line(p, 1_000)], PaymentMode::Cash),
    )
    .unwrap();
    assert_eq!(
        (invoice.series.as_str(), invoice.number),
        ("doc_facture", 1)
    );

    let second_ticket = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        cash(vec![line(p, 1_000)], 1_000_000),
    )
    .unwrap();
    assert_eq!(
        (second_ticket.series.as_str(), second_ticket.number),
        ("doc_ticket", 2)
    );
}

/// Writes a column straight onto a row, past the services that trim what
/// they are given. What a facture is checked against is whatever is stored,
/// and a row can carry a blank from an import, from a restore of an older
/// file, or from a fix somebody made with a SQL client.
fn set_blank(conn: &mut SqliteConnection, table: &str, id: i32, column: &str, value: &str) {
    diesel::sql_query(format!(
        "UPDATE {table} SET {column} = ? WHERE id = ? AND shop_id = 1"
    ))
    .bind::<diesel::sql_types::Text, _>(value)
    .bind::<diesel::sql_types::Integer, _>(id)
    .execute(conn)
    .unwrap();
}

#[test]
fn a_blank_identifier_is_as_missing_as_no_identifier_at_all() {
    // Décret 05-468 art. 3 asks for the numbers themselves, and two spaces
    // in the RC box is a facture with no RC on it. The services trim what
    // they are handed, so a blank only gets into a row another way; the
    // check is what stands between that row and a facture that prints an
    // empty box.
    let (_dir, mut conn) = open_temp();
    seller_ready(&mut conn);
    let p = product(&mut conn, "Ciment", 100_000, 1900, Unit::Piece);

    // The buyer's side, a company: both identifiers blanked, both named.
    let company = party(
        &mut conn,
        "Entreprise Amrani",
        PartyKind::Company,
        Some("16/00-7654321 B 22"),
        Some("098216007654321"),
        None,
    );
    set_blank(&mut conn, "customers", company, "rc", "   ");
    set_blank(&mut conn, "customers", company, "nis", "\t\n ");
    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        facture(company, vec![line(p, 1_000)], PaymentMode::Cash),
    )
    .unwrap_err();
    match err {
        CoreError::PartyIds { side, ref missing } => {
            assert_eq!(side, PartySide::Buyer);
            assert_eq!(missing, &["rc", "nis"]);
        }
        other => panic!("{other:?}"),
    }

    // The buyer's side, a consumer: a name of spaces is no name, and the
    // address goes the same way (art. 3-2, last alinéa).
    let consumer = party(
        &mut conn,
        "Karim Belkacem",
        PartyKind::Consumer,
        None,
        None,
        Some("12 rue des Frères Bouadou, Bir Mourad Raïs"),
    );
    set_blank(&mut conn, "customers", consumer, "name", "   ");
    set_blank(&mut conn, "customers", consumer, "address", " ");
    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        facture(consumer, vec![line(p, 1_000)], PaymentMode::Cash),
    )
    .unwrap_err();
    match err {
        CoreError::PartyIds { side, ref missing } => {
            assert_eq!(side, PartySide::Buyer);
            assert_eq!(missing, &["name", "address"]);
        }
        other => panic!("{other:?}"),
    }

    // The seller's side, and it is read first: a shop whose own RC is a
    // space has nothing a cashier could fix on the customer's fiche.
    let ready = party(
        &mut conn,
        "Entreprise Kaci",
        PartyKind::Company,
        Some("16/00-1111111 B 23"),
        Some("098216001111111"),
        None,
    );
    diesel::sql_query("UPDATE shops SET rc = '  ' WHERE id = 1")
        .execute(&mut conn)
        .unwrap();
    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        facture(ready, vec![line(p, 1_000)], PaymentMode::Cash),
    )
    .unwrap_err();
    match err {
        CoreError::PartyIds { side, ref missing } => {
            assert_eq!(side, PartySide::Seller);
            assert_eq!(missing, &["rc"]);
        }
        other => panic!("{other:?}"),
    }

    // Three refusals, no document, and the facture series still stands at
    // its first number.
    assert!(documents::list(&mut conn, SHOP, None).unwrap().is_empty());
    assert_eq!(counter(&mut conn, "doc_facture"), 1);
}

#[test]
fn the_stamp_on_a_facture_follows_the_cash_and_not_the_paper() {
    // The droit de timbre is due on a cash payment, whatever the paper says
    // (stamp_progressive_tranches). A facture paid in cash carries it, and
    // the same facture on card or on credit carries none: the kind of
    // document has never decided this.
    let (_dir, mut conn) = open_temp();
    seller_ready(&mut conn);
    // Exempt of TVA, so the tranches are counted on a round 10 000,00 DA:
    // a hundred tranches of 100,00 at 1,00 each is 100,00 of stamp.
    let p = product(&mut conn, "Huile", 100_000, 0, Unit::Piece);
    let c = party(
        &mut conn,
        "Entreprise Amrani",
        PartyKind::Company,
        Some("16/00-7654321 B 22"),
        Some("098216007654321"),
        None,
    );

    let paid = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            tendered: Some(Money::centimes(2_000_000)),
            ..facture(c, vec![line(p, 10_000)], PaymentMode::Cash)
        },
    )
    .unwrap();
    assert_eq!(paid.kind, DocumentKind::Facture);
    assert_eq!(paid.totals.total_ttc, Money::centimes(1_000_000));
    assert_eq!(paid.totals.stamp, Money::centimes(10_000));
    assert_eq!(paid.totals.net_to_pay, Money::centimes(1_010_000));

    for mode in [PaymentMode::Card, PaymentMode::Credit] {
        let doc = issue_sale(
            &mut conn,
            SHOP,
            OWNER,
            facture(c, vec![line(p, 10_000)], mode),
        )
        .unwrap();
        assert_eq!(doc.kind, DocumentKind::Facture, "{mode:?}");
        assert_eq!(doc.totals.stamp, Money::ZERO, "{mode:?}");
        assert_eq!(
            doc.totals.net_to_pay,
            Money::centimes(1_000_000),
            "{mode:?}"
        );
    }
}

#[test]
fn an_override_the_party_ids_then_refuse_leaves_no_log_row_and_no_number() {
    // The override is a decision the owner takes past the credit limit, and
    // features.md §5 logs decisions that were actually taken. This facture
    // never happened: the identifiers refused it after the limit had been
    // waived, so the log has nothing to say and neither series moved.
    let (_dir, mut conn) = open_temp();
    seller_ready(&mut conn);
    let p = product(&mut conn, "Ciment", 100_000, 1900, Unit::Piece);
    // Zero credit, so the limit refuses the first centime unless overridden,
    // and no NIS, so the facture is refused whatever the limit says.
    let c = party(
        &mut conn,
        "Entreprise Amrani",
        PartyKind::Company,
        Some("16/00-7654321 B 22"),
        None,
        None,
    );
    diesel::sql_query("UPDATE customers SET credit_limit_centimes = 0 WHERE id = ?")
        .bind::<diesel::sql_types::Integer, _>(c)
        .execute(&mut conn)
        .unwrap();

    let err = issue_sale(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            override_credit: true,
            ..facture(c, vec![line(p, 1_000)], PaymentMode::Credit)
        },
    )
    .unwrap_err();
    match err {
        CoreError::PartyIds { side, ref missing } => {
            assert_eq!(side, PartySide::Buyer);
            assert_eq!(missing, &["nis"]);
        }
        other => panic!("{other:?}"),
    }

    assert!(
        !audit::list(&mut conn, SHOP)
            .unwrap()
            .iter()
            .any(|e| e.action == "sale.credit_override"),
        "an override was logged for a sale that was refused"
    );
    assert!(documents::list(&mut conn, SHOP, None).unwrap().is_empty());
    assert!(debt::ledger(&mut conn, SHOP, c).unwrap().is_empty());
    assert_eq!(counter(&mut conn, "doc_facture"), 1);
    assert_eq!(counter(&mut conn, "doc_ticket"), 1);
}
