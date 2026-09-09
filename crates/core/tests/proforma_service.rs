// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The proforma (features.md §3): a quotation that looks like a facture and
//! moves nothing. What is asserted here is mostly what is absent, because
//! that is the whole rule: the stock table and the debt ledger have to come
//! out of a proforma exactly as they went in.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::money::{Bps, Money, PaymentMode};
use dzpos_core::services::customers::{NewCustomer, PartyKind};
use dzpos_core::services::documents::{Document, DocumentKind};
use dzpos_core::services::sales::{self, NewSale, NewSaleLine, SaleKind};
use dzpos_core::services::{customers, debt, documents, products};

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

fn product(conn: &mut SqliteConnection, name: &str, selling: i64, rate_bps: u32) -> i32 {
    products::create(
        conn,
        SHOP,
        OWNER,
        NewProduct {
            name: name.to_string(),
            barcode: None,
            category_id: None,
            unit: Unit::Piece,
            cost: Money::centimes(selling / 2),
            selling: Money::centimes(selling),
            wholesale: None,
            qty_on_hand_milli: 100_000,
            low_stock_at_milli: 0,
            rate_bps: Some(Bps::new(rate_bps).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id
}

fn a_fiche(name: &str) -> NewCustomer {
    NewCustomer {
        name: name.to_string(),
        party_kind: PartyKind::Company,
        phone: None,
        address: None,
        rc: None,
        nif: None,
        nis: None,
        ai: None,
        credit_limit: None,
        warn_threshold: None,
        notes: None,
        active: true,
    }
}

fn a_customer(conn: &mut SqliteConnection, name: &str) -> i32 {
    customers::create(conn, SHOP, OWNER, a_fiche(name), None)
        .unwrap()
        .id
}

fn quotation(customer_id: Option<i32>, product_id: i32, mode: PaymentMode) -> NewSale {
    NewSale {
        lines: vec![NewSaleLine {
            product_id,
            qty_milli: 2_000,
            unit_price: None,
            line_discount: Money::ZERO,
        }],
        global_discount: Money::ZERO,
        payment_mode: mode,
        tendered: None,
        customer_id,
        override_credit: false,
        kind: SaleKind::Proforma,
        issued_at: Some(at(10)),
    }
}

fn issue(conn: &mut SqliteConnection, new: NewSale) -> Result<Document, CoreError> {
    sales::issue(conn, SHOP, OWNER, new).map(|s| s.document)
}

fn rows(conn: &mut SqliteConnection, table: &str) -> i32 {
    #[derive(QueryableByName)]
    struct Count {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        n: i32,
    }
    diesel::sql_query(format!("SELECT COUNT(*) AS n FROM {table}"))
        .get_result::<Count>(conn)
        .unwrap()
        .n
}

#[test]
fn a_proforma_takes_its_own_number_and_moves_neither_stock_nor_debt() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 1900);
    let c = a_customer(&mut conn, "Entreprise Benali");
    let stock_before = rows(&mut conn, "stock_movements");
    let ledger_before = rows(&mut conn, "debt_ledger");
    let qty_before = products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli;

    let doc = issue(&mut conn, quotation(Some(c), p, PaymentMode::Credit)).unwrap();

    assert_eq!(doc.kind, DocumentKind::Proforma);
    assert_eq!(doc.series, "doc_proforma");
    assert_eq!(doc.number, 1);
    assert_eq!(doc.customer_id, Some(c));
    assert_eq!(doc.tendered, None);
    assert_eq!(doc.change, None);
    // The basket is priced the way the till would price it.
    assert_eq!(doc.totals.total_ht, Money::centimes(200_000));
    assert_eq!(doc.totals.tva, Money::centimes(38_000));
    assert_eq!(doc.totals.total_ttc, Money::centimes(238_000));

    // The buyer block is snapshotted the way a facture's is.
    let buyer = doc.buyer.clone().expect("a proforma names a customer");
    assert_eq!(buyer.name, "Entreprise Benali");
    assert_eq!(buyer.party_kind, PartyKind::Company);

    // Three zeros and not a missing triple: the customer is named, so the
    // paper has a debt block, and what it says is that this changes nothing.
    let balance = doc.balance.expect("a proforma carries the triple");
    assert_eq!(balance.old_balance, Money::ZERO);
    assert_eq!(balance.remaining_debt, Money::ZERO);
    assert_eq!(balance.total_debt, Money::ZERO);

    // And the point of the whole thing: nothing moved.
    assert_eq!(rows(&mut conn, "stock_movements"), stock_before);
    assert_eq!(rows(&mut conn, "debt_ledger"), ledger_before);
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        qty_before
    );
    assert_eq!(debt::balance(&mut conn, SHOP, c).unwrap(), Money::ZERO);
    assert_eq!(documents::get(&mut conn, SHOP, doc.id).unwrap(), doc);
}

#[test]
fn a_proforma_needs_a_customer() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 1900);
    let err = issue(&mut conn, quotation(None, p, PaymentMode::Cash)).unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "customer_id"),
        "{err:?}"
    );
    // And the refusal burned no number: the counter is untouched.
    assert_eq!(
        documents::list(&mut conn, SHOP, Some(DocumentKind::Proforma))
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn a_proforma_quotes_the_stamp_the_facture_would_carry() {
    // The droit de timbre follows the cash and not the paper, so a quotation
    // for a cash purchase has to quote it: a facture that came to more than
    // the customer was told is a quotation that was wrong.
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = a_customer(&mut conn, "Entreprise Benali");

    let cash = issue(&mut conn, quotation(Some(c), p, PaymentMode::Cash)).unwrap();
    assert!(cash.totals.stamp > Money::ZERO);
    assert_eq!(
        cash.totals.net_to_pay,
        cash.totals
            .total_ttc
            .checked_add(cash.totals.stamp)
            .unwrap()
    );

    let credit = issue(&mut conn, quotation(Some(c), p, PaymentMode::Credit)).unwrap();
    assert_eq!(credit.totals.stamp, Money::ZERO);
    assert_eq!(credit.number, 2, "the proforma series runs on");
}

#[test]
fn a_proforma_takes_no_amount_tendered_and_no_closed_fiche() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = a_customer(&mut conn, "Entreprise Benali");

    let mut with_cash = quotation(Some(c), p, PaymentMode::Cash);
    with_cash.tendered = Some(Money::centimes(500_000));
    let err = issue(&mut conn, with_cash).unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "tendered"),
        "{err:?}"
    );

    let mut closed_fiche = a_fiche("Entreprise Benali");
    closed_fiche.active = false;
    customers::update(&mut conn, SHOP, OWNER, c, closed_fiche, None).unwrap();
    let closed = issue(&mut conn, quotation(Some(c), p, PaymentMode::Cash)).unwrap_err();
    assert!(
        matches!(closed, CoreError::Validation { ref field, .. } if field == "customer_id"),
        "{closed:?}"
    );
}

#[test]
fn the_proforma_and_the_facture_series_do_not_touch() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = a_customer(&mut conn, "Entreprise Benali");
    issue(&mut conn, quotation(Some(c), p, PaymentMode::Cash)).unwrap();
    issue(&mut conn, quotation(Some(c), p, PaymentMode::Cash)).unwrap();

    // A quotation burns only its own number, so the ticket the till rings up
    // next is the first of its own series.
    let ticket = sales::issue(
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
            tendered: Some(Money::centimes(1_000_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(at(11)),
        },
    )
    .unwrap()
    .document;
    assert_eq!(ticket.number, 1);
    assert_eq!(
        documents::list(&mut conn, SHOP, Some(DocumentKind::Proforma))
            .unwrap()
            .iter()
            .map(|d| d.number)
            .collect::<Vec<i64>>(),
        [2, 1],
        "newest first"
    );
}
