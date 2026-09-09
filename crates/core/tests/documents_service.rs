// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Numbering and the stored document, against a real temp SQLite file.
//! features.md §3: one uninterrupted series per kind, assigned at issue and
//! never reused.

use chrono::NaiveDate;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::money::{Bps, Money, PaymentMode, Regime, Totals, TvaLine};
use dzpos_core::services::documents::{
    self, DocumentKind, DocumentStatus, NewDocument, NewDocumentLine, SellerBlock,
};
use dzpos_core::services::products;

const SHOP: i32 = 1;
const OWNER: i32 = 1;

fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let conn = dzpos_core::db::open(&path).unwrap();
    (dir, conn)
}

fn at(day: u32, hour: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, day)
        .unwrap()
        .and_hms_opt(hour, 0, 0)
        .unwrap()
}

fn a_product(conn: &mut SqliteConnection, name: &str) -> i32 {
    products::create(
        conn,
        SHOP,
        OWNER,
        NewProduct {
            name: name.to_string(),
            barcode: None,
            category_id: Some(1),
            unit: Unit::Piece,
            cost: Money::centimes(820),
            selling: Money::centimes(920),
            wholesale: None,
            qty_on_hand_milli: 10_000,
            low_stock_at_milli: 0,
            rate_bps: None,
            active: true,
        },
    )
    .unwrap()
    .id
}

/// A second shop with a product of its own, inserted raw: what is under test
/// is the service's scoping, never this seed.
fn seed_second_shop_product(conn: &mut SqliteConnection) -> i32 {
    #[derive(diesel::QueryableByName)]
    struct Id {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        id: i32,
    }
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO products (shop_id, name, unit, cost_centimes, selling_centimes, \
         qty_on_hand_milli, low_stock_at_milli, rate_bps) \
         VALUES (2, 'Ailleurs', 'piece', 0, 0, 0, 0, 1900)",
    )
    .execute(conn)
    .unwrap();
    let row: Id = diesel::sql_query("SELECT id FROM products WHERE shop_id = 2")
        .get_result(conn)
        .unwrap();
    row.id
}

fn totals(total: i64) -> Totals {
    Totals {
        total_ht: Money::centimes(total),
        discount: Money::ZERO,
        subtotal_ht: Money::centimes(total),
        tva_by_rate: vec![TvaLine {
            rate: Bps::new(1900).unwrap(),
            base: Money::centimes(total),
            amount: Money::centimes(total / 100 * 19),
        }],
        tva: Money::centimes(total / 100 * 19),
        total_ttc: Money::centimes(total),
        stamp: Money::ZERO,
        net_to_pay: Money::centimes(total),
    }
}

fn draft(kind: DocumentKind, product_id: Option<i32>, issued_at: NaiveDateTime) -> NewDocument {
    NewDocument {
        kind,
        issued_at,
        user_id: OWNER,
        regime: Regime::Reel,
        payment_mode: PaymentMode::Cash,
        seller: SellerBlock {
            name: "Mon magasin".to_string(),
            rc: Some("16/00-1234567 B 21".to_string()),
            nif: Some("000216001234567".to_string()),
            nis: None,
            ai: None,
            address: Some("Rue Didouche Mourad, Alger".to_string()),
            phone: None,
        },
        customer_id: None,
        totals: totals(10_000),
        tendered: Some(Money::centimes(10_000)),
        change: Some(Money::ZERO),
        lines: vec![NewDocumentLine {
            product_id,
            name: "Sucre".to_string(),
            barcode: Some("200001000007".to_string()),
            qty_milli: 1_000,
            unit_price: Money::centimes(10_000),
            line_discount: Money::ZERO,
            rate_bps: Bps::new(1900).unwrap(),
            line_total: Money::centimes(10_000),
        }],
    }
}

#[test]
fn two_tickets_take_the_number_after_the_last() {
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let first = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 10)),
    )
    .unwrap();
    let second = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 11)),
    )
    .unwrap();
    assert_eq!(first.number, 1);
    assert_eq!(second.number, 2);
    assert_eq!(first.series, "doc_ticket");
    assert_eq!(second.series, "doc_ticket");
    assert_eq!(first.status, DocumentStatus::Issued);
}

#[test]
fn a_refused_line_burns_no_number() {
    // The number is taken before the lines are written, so the rollback is
    // the only thing keeping the series uninterrupted. Without it the next
    // ticket would be number 2 and number 1 would exist nowhere.
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let elsewhere = seed_second_shop_product(&mut conn);
    let err = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(elsewhere), at(9, 10)),
    )
    .unwrap_err();
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
    let next = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 11)),
    )
    .unwrap();
    assert_eq!(next.number, 1, "the refused ticket burned a number");
    assert_eq!(documents::list(&mut conn, SHOP, None).unwrap().len(), 1);
}

#[test]
fn each_kind_counts_in_its_own_series() {
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let ticket = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 10)),
    )
    .unwrap();
    let facture = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Facture, Some(p), at(9, 11)),
    )
    .unwrap();
    let ticket2 = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 12)),
    )
    .unwrap();
    assert_eq!((ticket.number, ticket.series.as_str()), (1, "doc_ticket"));
    assert_eq!(
        (facture.number, facture.series.as_str()),
        (1, "doc_facture")
    );
    assert_eq!(ticket2.number, 2);
}

#[test]
fn a_document_reads_back_with_its_lines_its_tva_rows_and_its_seller_block() {
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let made = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 10)),
    )
    .unwrap();
    let read = documents::get(&mut conn, SHOP, made.id).unwrap();
    assert_eq!(read, made);
    assert_eq!(read.lines.len(), 1);
    assert_eq!(read.lines[0].position, 0);
    assert_eq!(read.lines[0].name, "Sucre");
    assert_eq!(read.lines[0].qty_milli, 1_000);
    assert_eq!(read.totals.tva_by_rate.len(), 1);
    assert_eq!(read.totals.tva_by_rate[0].rate, Bps::new(1900).unwrap());
    assert_eq!(read.seller.rc.as_deref(), Some("16/00-1234567 B 21"));
    assert_eq!(read.seller.nis, None);
    assert_eq!(read.regime, Regime::Reel);
    assert_eq!(read.payment_mode, PaymentMode::Cash);
    assert_eq!(read.tendered, Some(Money::centimes(10_000)));
    assert_eq!(read.issued_at, at(9, 10));
}

#[test]
fn a_document_of_another_shop_is_not_found() {
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let made = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 10)),
    )
    .unwrap();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut conn)
        .unwrap();
    let err = documents::get(&mut conn, 2, made.id).unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::NotFound {
                entity: "document",
                ..
            }
        ),
        "{err:?}"
    );
    assert!(documents::list(&mut conn, 2, None).unwrap().is_empty());
}

#[test]
fn the_list_is_newest_first_and_filters_by_kind() {
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let older = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(8, 10)),
    )
    .unwrap();
    let newer = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 10)),
    )
    .unwrap();
    let facture = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Facture, Some(p), at(8, 9)),
    )
    .unwrap();
    let all = documents::list(&mut conn, SHOP, None).unwrap();
    let ids: Vec<i32> = all.iter().map(|d| d.id).collect();
    assert_eq!(ids, vec![newer.id, older.id, facture.id]);
    let tickets = documents::list(&mut conn, SHOP, Some(DocumentKind::Ticket)).unwrap();
    assert_eq!(
        tickets.iter().map(|d| d.id).collect::<Vec<_>>(),
        vec![newer.id, older.id]
    );
}

#[test]
fn two_documents_in_the_same_second_list_by_the_later_insert() {
    // issued_at is whole seconds and a busy till issues two tickets inside
    // one, so the till expects the later sale first. This pins the order the
    // screen shows; SQLite happens to return it without the id tie-break
    // too, so the clause itself is not what this test proves.
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let first = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 10)),
    )
    .unwrap();
    let second = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 10)),
    )
    .unwrap();
    let ids: Vec<i32> = documents::list(&mut conn, SHOP, None)
        .unwrap()
        .iter()
        .map(|d| d.id)
        .collect();
    assert_eq!(ids, vec![second.id, first.id]);
}

#[test]
fn a_line_keeps_its_snapshot_when_the_product_is_deleted() {
    // features.md §3: the line carries a product snapshot. A deleted product
    // must not take the sold line with it.
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let made = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 10)),
    )
    .unwrap();
    // The ledger holds the product down first: creating it wrote an opening
    // movement, and stock_movements.product_id RESTRICTs. Only once the
    // ledger is gone can the delete reach the line's snapshot at all.
    assert!(
        diesel::sql_query(format!("DELETE FROM products WHERE id = {p}"))
            .execute(&mut conn)
            .is_err(),
        "a product with a movement was deleted"
    );
    diesel::sql_query(format!(
        "DELETE FROM stock_movements WHERE product_id = {p}"
    ))
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(format!("DELETE FROM products WHERE id = {p}"))
        .execute(&mut conn)
        .unwrap();
    let read = documents::get(&mut conn, SHOP, made.id).unwrap();
    assert_eq!(read.lines.len(), 1);
    assert_eq!(read.lines[0].name, "Sucre");
    assert_eq!(read.lines[0].product_id, None);
}

/// Every document whose stored totals disagree with its stored lines or its
/// stored TVA recap. The columns a reprint reads are the ones a paper
/// document is made of, so nothing may drift between them: the query is the
/// invariant, and the test that runs it names no expected numbers at all.
fn totals_that_disagree_with_their_lines(conn: &mut SqliteConnection) -> Vec<i32> {
    #[derive(diesel::QueryableByName)]
    struct Id {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        id: i32,
    }
    let rows: Vec<Id> = diesel::sql_query(
        "SELECT d.id FROM documents d WHERE \
         d.total_ht_centimes != (SELECT SUM(line_total_centimes) FROM document_lines l \
             WHERE l.document_id = d.id) \
         OR d.subtotal_ht_centimes != d.total_ht_centimes - d.discount_centimes \
         OR d.total_ttc_centimes != d.subtotal_ht_centimes + d.tva_centimes \
         OR d.net_to_pay_centimes != d.total_ttc_centimes + d.stamp_centimes \
         OR d.tva_centimes != COALESCE((SELECT SUM(amount_centimes) FROM document_tva t \
             WHERE t.document_id = d.id), 0) \
         OR (d.regime = 'reel' AND d.subtotal_ht_centimes != (SELECT SUM(base_centimes) \
             FROM document_tva t WHERE t.document_id = d.id)) \
         OR (d.regime = 'ifu' AND EXISTS (SELECT 1 FROM document_tva t \
             WHERE t.document_id = d.id))",
    )
    .load(conn)
    .unwrap();
    rows.into_iter().map(|r| r.id).collect()
}

#[test]
fn a_stored_total_always_adds_up_from_the_stored_lines() {
    use dzpos_core::models::product::Unit;
    use dzpos_core::services::sales::{self, NewSale, NewSaleLine};
    use dzpos_core::services::settings;

    let priced = |conn: &mut SqliteConnection, name: &str, selling: i64, rate: u32| {
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
                rate_bps: Some(Bps::new(rate).unwrap()),
                active: true,
            },
        )
        .unwrap()
        .id
    };
    let sold = |product_id: i32, qty_milli: i64| NewSaleLine {
        product_id,
        qty_milli,
        unit_price: None,
        line_discount: Money::ZERO,
    };

    let (_dir, mut conn) = open_temp();
    let high = priced(&mut conn, "Sucre", 20_000, 1900);
    let low = priced(&mut conn, "Pain", 3_000, 900);

    // Réel, two rates, a global discount the recap has to spread.
    sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![sold(high, 1_500), sold(low, 2_000)],
            global_discount: Money::centimes(1_000),
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(100_000)),
            issued_at: Some(at(9, 10)),
        },
    )
    .unwrap();
    // A card sale: no stamp, nothing tendered.
    sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![sold(high, 1_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Card,
            tendered: None,
            issued_at: Some(at(9, 11)),
        },
    )
    .unwrap();
    // Under the IFU: no recap row at all, and the stamp still applies.
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, at(9, 12)).unwrap();
    let ifu = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![sold(high, 1_000), sold(low, 3_000)],
            global_discount: Money::centimes(500),
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(100_000)),
            issued_at: Some(at(9, 13)),
        },
    )
    .unwrap();

    assert_eq!(
        totals_that_disagree_with_their_lines(&mut conn),
        Vec::<i32>::new()
    );

    // The query bites: one centime moved on one stored column and the
    // document is named. Without this the test would pass on a query that
    // compares nothing.
    diesel::sql_query(format!(
        "UPDATE documents SET total_ht_centimes = total_ht_centimes + 1 WHERE id = {}",
        ifu.id
    ))
    .execute(&mut conn)
    .unwrap();
    assert_eq!(
        totals_that_disagree_with_their_lines(&mut conn),
        vec![ifu.id]
    );
    diesel::sql_query(format!(
        "UPDATE documents SET total_ht_centimes = total_ht_centimes - 1 WHERE id = {}",
        ifu.id
    ))
    .execute(&mut conn)
    .unwrap();
    assert_eq!(
        totals_that_disagree_with_their_lines(&mut conn),
        Vec::<i32>::new()
    );
}
