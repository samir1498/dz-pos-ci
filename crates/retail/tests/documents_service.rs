// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Numbering and the stored document, against a real temp SQLite file.
//! features.md §3: one uninterrupted series per kind and per year, assigned at
//! issue and never reused.

use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::{TimeZone, Utc};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_retail::error::CoreError;
use dzpos_retail::models::product::{NewProduct, Unit};
use dzpos_retail::money::{Bps, Money, PaymentMode, Regime, Totals, TvaLine};
use dzpos_retail::services::customers;
use dzpos_retail::services::documents::{
    self, BalanceTriple, DocumentKind, DocumentStatus, NewDocument, NewDocumentLine, PartyBlock,
    PartyKind, SellerBlock,
};
use dzpos_retail::services::products;

const SHOP: i32 = 1;
const OWNER: i32 = 1;

mod common;

use common::open_temp;

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
        customer: None,
        buyer: None,
        ref_document_id: None,
        balance: None,
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
            ref_line_id: None,
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
    assert_eq!(first.series, "doc_ticket:2026");
    assert_eq!(second.series, "doc_ticket:2026");
    assert_eq!((first.series_year, second.series_year), (2026, 2026));
    assert_eq!(first.status, DocumentStatus::Issued);
}

/// The moment the shop's till is at when UTC reads `utc` on 31 December 2026.
/// Algeria is UTC+1 with no daylight saving, so the hour between 23:00 UTC
/// and midnight is already the new year in the shop (`services::clock`).
fn on_new_years_eve(hour: u32, minute: u32) -> NaiveDateTime {
    let utc = Utc
        .with_ymd_and_hms(2026, 12, 31, hour, minute, 0)
        .single()
        .unwrap();
    dzpos_kernel::services::clock::shop_time(utc)
}

#[test]
fn the_series_restarts_at_one_in_the_new_year_and_the_old_one_keeps_its_numbers() {
    // features.md §4, Numbering: a series restarts each year and the number
    // carries the year. The two tickets are an hour apart on the same UTC
    // evening and the shop's clock puts them in different years, which is the
    // case a machine-clock year would get wrong.
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let last_of_the_year = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), on_new_years_eve(22, 30)),
    )
    .unwrap();
    let first_of_the_next = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), on_new_years_eve(23, 30)),
    )
    .unwrap();

    assert_eq!(
        (
            last_of_the_year.number,
            last_of_the_year.series_year,
            last_of_the_year.series.as_str()
        ),
        (1, 2026, "doc_ticket:2026")
    );
    assert_eq!(
        (
            first_of_the_next.number,
            first_of_the_next.series_year,
            first_of_the_next.series.as_str()
        ),
        (1, 2027, "doc_ticket:2027"),
        "the first ticket of the new year did not start the series again"
    );
    // The old year's paper is untouched: it keeps the number it was handed
    // and the two read back as two different documents.
    let stored = documents::get(&mut conn, SHOP, last_of_the_year.id).unwrap();
    assert_eq!((stored.number, stored.series_year), (1, 2026));
    assert_ne!(
        dzpos_retail::print::number(&stored),
        dzpos_retail::print::number(&first_of_the_next),
        "two tickets numbered 1 print the same number"
    );
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
    assert_eq!(
        (ticket.number, ticket.series.as_str()),
        (1, "doc_ticket:2026")
    );
    assert_eq!(
        (facture.number, facture.series.as_str()),
        (1, "doc_facture:2026")
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
fn two_documents_in_the_same_second_show_the_higher_id_first() {
    // issued_at is whole seconds and a busy till issues two tickets inside
    // one, so the till expects the later sale first. The ids are written by
    // hand, the higher one first, so the order under test is the reverse of
    // rowid order and a query that fell back to the file's own order would
    // hand back 10 before 20.
    //
    // This pins what the screen shows, not the `id DESC` clause that
    // promises it: with the clause removed the query still passes, because
    // idx_documents_shop_issued is scanned backwards and equal issued_at
    // values come out by falling rowid anyway. Drop that index as well and
    // the sort falls back to rowid order and this goes red.
    let (_dir, mut conn) = open_temp();
    for (id, number) in [(20, 1), (10, 2)] {
        diesel::sql_query(format!(
            "INSERT INTO documents (id, shop_id, kind, series, number, issued_at, user_id, \
             regime, payment_mode, seller_name, total_ht_centimes, discount_centimes, \
             subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes, \
             net_to_pay_centimes, status) \
             VALUES ({id}, 1, 'ticket', 'doc_ticket', {number}, '2026-09-09 10:00:00', 1, \
             'reel', 'cash', 'Mon magasin', 0, 0, 0, 0, 0, 0, 0, 'issued')"
        ))
        .execute(&mut conn)
        .unwrap();
    }
    let ids: Vec<i32> = documents::list(&mut conn, SHOP, None)
        .unwrap()
        .iter()
        .map(|d| d.id)
        .collect();
    assert_eq!(ids, vec![20, 10]);
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
///
/// Every subquery is COALESCEd, because `SUM` over no rows is NULL and every
/// comparison with NULL is NULL, which a WHERE reads as false. A document
/// whose lines were deleted, or a réel document whose recap rows were, would
/// otherwise be the one thing this query is blind to: the emptier the row set
/// the more certainly it passed.
fn totals_that_disagree_with_their_lines(conn: &mut SqliteConnection) -> Vec<i32> {
    #[derive(diesel::QueryableByName)]
    struct Id {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        id: i32,
    }
    let rows: Vec<Id> = diesel::sql_query(
        "SELECT d.id FROM documents d WHERE \
         d.total_ht_centimes != COALESCE((SELECT SUM(line_total_centimes) \
             FROM document_lines l WHERE l.document_id = d.id), 0) \
         OR d.subtotal_ht_centimes != d.total_ht_centimes - d.discount_centimes \
         OR d.total_ttc_centimes != d.subtotal_ht_centimes + d.tva_centimes \
         OR d.net_to_pay_centimes != d.total_ttc_centimes + d.stamp_centimes \
         OR d.tva_centimes != COALESCE((SELECT SUM(amount_centimes) FROM document_tva t \
             WHERE t.document_id = d.id), 0) \
         OR (d.regime = 'reel' AND d.subtotal_ht_centimes != \
             COALESCE((SELECT SUM(base_centimes) FROM document_tva t \
                 WHERE t.document_id = d.id), 0)) \
         OR (d.regime = 'ifu' AND EXISTS (SELECT 1 FROM document_tva t \
             WHERE t.document_id = d.id))",
    )
    .load(conn)
    .unwrap();
    rows.into_iter().map(|r| r.id).collect()
}

#[test]
fn a_stored_total_always_adds_up_from_the_stored_lines() {
    use dzpos_kernel::services::settings;
    use dzpos_retail::models::product::Unit;
    use dzpos_retail::services::sales::{self, NewSale, NewSaleLine, SaleKind};

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
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(at(9, 10)),
        },
    )
    .unwrap();
    // A card sale: no stamp, nothing tendered.
    let card = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![sold(high, 1_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Card,
            tendered: None,
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(at(9, 11)),
        },
    )
    .unwrap()
    .document;
    // Réel at 0%: one recap row, a base equal to the subtotal and an amount of
    // zero. It is the only document whose recap the tva_centimes disjunct
    // cannot speak for, so it is what proves the base-sum disjunct on its own.
    let exempt_product = priced(&mut conn, "Semoule", 12_000, 0);
    let exempt = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![sold(exempt_product, 2_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(100_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(at(9, 12)),
        },
    )
    .unwrap()
    .document;
    // Under the IFU: no recap row at all, and the stamp still applies.
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, at(9, 13)).unwrap();
    let ifu = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![sold(high, 1_000), sold(low, 3_000)],
            global_discount: Money::centimes(500),
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(100_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(at(9, 14)),
        },
    )
    .unwrap()
    .document;

    assert_eq!(
        totals_that_disagree_with_their_lines(&mut conn),
        Vec::<i32>::new()
    );

    // The query bites, on each of the seven columns a paper document is made
    // of and one at a time. A query that compared nothing, or that lost one
    // of its disjuncts to a later edit, would pass the sweep above and go red
    // on whichever column that disjunct is the only one to speak for.
    for column in [
        "total_ht_centimes",
        "discount_centimes",
        "subtotal_ht_centimes",
        "tva_centimes",
        "total_ttc_centimes",
        "stamp_centimes",
        "net_to_pay_centimes",
    ] {
        let moved = |conn: &mut SqliteConnection, by: &str| {
            diesel::sql_query(format!(
                "UPDATE documents SET {column} = {column} {by} 1 WHERE id = {}",
                ifu.id
            ))
            .execute(conn)
            .unwrap();
        };
        moved(&mut conn, "+");
        assert_eq!(
            totals_that_disagree_with_their_lines(&mut conn),
            vec![ifu.id],
            "one centime on {column} left the document standing"
        );
        moved(&mut conn, "-");
        assert_eq!(
            totals_that_disagree_with_their_lines(&mut conn),
            Vec::<i32>::new(),
            "{column} was not put back"
        );
    }

    // A document with no lines at all. SUM over no rows is NULL and a
    // comparison with NULL is not true, so without the COALESCE this is the
    // document the query cannot see: its totals stand and nothing backs them.
    diesel::sql_query(format!(
        "DELETE FROM document_lines WHERE document_id = {}",
        card.id
    ))
    .execute(&mut conn)
    .unwrap();
    assert_eq!(
        totals_that_disagree_with_their_lines(&mut conn),
        vec![card.id],
        "a document whose lines are gone was not named"
    );
    diesel::sql_query(format!(
        "INSERT INTO document_lines (shop_id, document_id, position, name, qty_milli, \
         unit_price_centimes, line_discount_centimes, rate_bps, line_total_centimes) \
         SELECT {SHOP}, {0}, 0, 'Sucre', 1000, total_ht_centimes, 0, 1900, total_ht_centimes \
         FROM documents WHERE id = {0}",
        card.id
    ))
    .execute(&mut conn)
    .unwrap();
    assert_eq!(
        totals_that_disagree_with_their_lines(&mut conn),
        Vec::<i32>::new()
    );

    // A réel document whose recap rows are gone. Its TVA is zero, so the
    // tva_centimes disjunct still agrees with the empty recap and only the
    // base sum can name it: the same NULL, on the other subquery.
    diesel::sql_query(format!(
        "DELETE FROM document_tva WHERE document_id = {}",
        exempt.id
    ))
    .execute(&mut conn)
    .unwrap();
    assert_eq!(
        totals_that_disagree_with_their_lines(&mut conn),
        vec![exempt.id],
        "a réel document with no recap left was not named"
    );
}

/// A customer to make a facture out to. Inserted raw: what is under test here
/// is the document, and the customer service has its own file.
fn a_customer(conn: &mut SqliteConnection) -> i32 {
    #[derive(diesel::QueryableByName)]
    struct Id {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        id: i32,
    }
    diesel::sql_query(
        "INSERT INTO customers (shop_id, name, party_kind, rc, nif) \
         VALUES (1, 'Entreprise Benali', 'company', '16/00-7654321 B 22', '000216007654321')",
    )
    .execute(conn)
    .unwrap();
    let row: Id = diesel::sql_query("SELECT MAX(id) AS id FROM customers WHERE shop_id = 1")
        .get_result(conn)
        .unwrap();
    row.id
}

#[test]
fn a_facture_stores_its_buyer_block_and_its_balance_and_reads_them_back() {
    // The buyer block is a snapshot, so what matters is that every field
    // reaches the file and comes back: a field dropped between the write and
    // the read is invisible until a comptable reads the paper.
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let customer = a_customer(&mut conn);
    let mut new = draft(DocumentKind::Facture, Some(p), at(9, 10));
    new.customer = Some(customers::prove(&mut conn, SHOP, customer).unwrap());
    new.buyer = Some(PartyBlock {
        name: "Entreprise Benali".to_string(),
        party_kind: PartyKind::Company,
        rc: Some("16/00-7654321 B 22".to_string()),
        nif: Some("000216007654321".to_string()),
        nis: Some("000216007654321000".to_string()),
        ai: None,
        address: Some("Zone industrielle, Rouiba".to_string()),
    });
    new.balance = Some(BalanceTriple {
        old_balance: Money::centimes(250_000),
        remaining_debt: Money::centimes(260_000),
        total_debt: Money::centimes(260_000),
    });
    let issued = documents::issue(&mut conn, SHOP, new).unwrap();

    let read = documents::get(&mut conn, SHOP, issued.id).unwrap();
    assert_eq!(read.customer_id, Some(customer));
    assert_eq!(read.buyer, issued.buyer);
    assert_eq!(
        read.buyer.as_ref().map(|b| b.party_kind),
        Some(PartyKind::Company)
    );
    assert_eq!(
        read.buyer.as_ref().and_then(|b| b.nis.as_deref()),
        Some("000216007654321000"),
        "a buyer identifier was lost between the write and the read"
    );
    assert_eq!(
        read.balance.map(|b| b.old_balance),
        Some(Money::centimes(250_000))
    );
    assert_eq!(
        read.balance.map(|b| b.remaining_debt),
        Some(Money::centimes(260_000))
    );
    assert_eq!(read.ref_document_id, None);
}

#[test]
fn a_ticket_with_no_customer_carries_no_buyer_and_no_balance() {
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let issued = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 10)),
    )
    .unwrap();
    let read = documents::get(&mut conn, SHOP, issued.id).unwrap();
    assert_eq!(read.buyer, None);
    assert_eq!(read.balance, None);
}

#[test]
fn a_stored_balance_missing_one_of_its_three_amounts_is_refused() {
    // Two of three would let a facture print a closing balance its own
    // opening balance does not explain. The service writes all three or none,
    // so the only way in is a file written by something else, and that is an
    // error rather than a guess.
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let issued = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 10)),
    )
    .unwrap();
    diesel::sql_query(format!(
        "UPDATE documents SET old_balance_centimes = 1000 WHERE id = {}",
        issued.id
    ))
    .execute(&mut conn)
    .unwrap();

    let err = documents::get(&mut conn, SHOP, issued.id).unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "balance"),
        "a half-written balance triple read back as a document: {err}"
    );
}

#[test]
fn a_stored_buyer_name_without_a_party_kind_is_refused() {
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let issued = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Ticket, Some(p), at(9, 10)),
    )
    .unwrap();
    diesel::sql_query(format!(
        "UPDATE documents SET buyer_name = 'Benali' WHERE id = {}",
        issued.id
    ))
    .execute(&mut conn)
    .unwrap();

    let err = documents::get(&mut conn, SHOP, issued.id).unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "buyer"),
        "a half-written buyer block read back as a document: {err}"
    );
}

/// A customer and a document belonging to the second shop, inserted raw for
/// the same reason its product is: the seed is not what is under test.
fn seed_second_shop_customer_and_document(conn: &mut SqliteConnection) -> (i32, i32) {
    #[derive(diesel::QueryableByName)]
    struct Id {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        id: i32,
    }
    diesel::sql_query("INSERT OR IGNORE INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO customers (shop_id, name, party_kind) \
         VALUES (2, 'Client du voisin', 'company')",
    )
    .execute(conn)
    .unwrap();
    let customer: Id = diesel::sql_query("SELECT MAX(id) AS id FROM customers WHERE shop_id = 2")
        .get_result(conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO documents (shop_id, kind, series, number, issued_at, user_id, regime, \
         payment_mode, seller_name, total_ht_centimes, discount_centimes, \
         subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes, \
         net_to_pay_centimes) VALUES (2, 'facture', 'doc_facture', 1, \
         '2026-09-09 10:00:00', 1, 'reel', 'credit', 'Magasin du voisin', 10000, 0, 10000, \
         1900, 11900, 0, 11900)",
    )
    .execute(conn)
    .unwrap();
    let document: Id = diesel::sql_query("SELECT MAX(id) AS id FROM documents WHERE shop_id = 2")
        .get_result(conn)
        .unwrap();
    (customer.id, document.id)
}

#[test]
fn a_document_made_out_to_another_shops_customer_is_refused_and_burns_no_number() {
    // Rule 3: the foreign key would take the neighbour's fiche, and the
    // buyer block printed on the paper would name their customer.
    //
    // There are two doors now, and this walks both. `issue` takes a
    // `ProvedCustomer` rather than an id, so the first refusal is at
    // `customers::prove`: this shop cannot make a proof about a fiche it
    // does not have, and a caller with only the id has nothing `issue` will
    // accept. The second is the one the type cannot state, a proof made in
    // the neighbour's own shop and carried to a document of this one, which
    // `issue` refuses by comparing the shop the proof is about with its own.
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let (elsewhere, _) = seed_second_shop_customer_and_document(&mut conn);

    let refused = customers::prove(&mut conn, SHOP, elsewhere).unwrap_err();
    assert!(
        matches!(
            refused,
            CoreError::NotFound {
                entity: "customer",
                ..
            }
        ),
        "{refused:?}"
    );

    let theirs = customers::prove(&mut conn, 2, elsewhere).unwrap();
    assert_eq!(theirs.id(), elsewhere);
    assert_eq!(theirs.shop_id(), 2);
    let mut stolen = draft(DocumentKind::Facture, Some(p), at(9, 10));
    stolen.customer = Some(theirs);
    let err = documents::issue(&mut conn, SHOP, stolen).unwrap_err();
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

    let next = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Facture, Some(p), at(9, 11)),
    )
    .unwrap();
    assert_eq!(next.number, 1, "the refused facture burned a number");
    assert_eq!(documents::list(&mut conn, SHOP, None).unwrap().len(), 1);
}

#[test]
fn a_document_written_against_another_shops_document_is_refused_and_burns_no_number() {
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let (_, elsewhere) = seed_second_shop_customer_and_document(&mut conn);

    let mut stolen = draft(DocumentKind::Avoir, Some(p), at(9, 10));
    stolen.ref_document_id = Some(elsewhere);
    let err = documents::issue(&mut conn, SHOP, stolen).unwrap_err();
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

    // The avoir that proves the number was not burned is written against a
    // paper of this shop, because an avoir names the document it credits
    // (migration 7, the kind rules).
    let mine = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Facture, Some(p), at(9, 11)),
    )
    .unwrap();
    let mut next_avoir = draft(DocumentKind::Avoir, Some(p), at(9, 11));
    next_avoir.ref_document_id = Some(mine.id);
    let next = documents::issue(&mut conn, SHOP, next_avoir).unwrap();
    assert_eq!(next.number, 1, "the refused avoir burned a number");
}

#[test]
fn an_avoir_line_credits_a_line_of_the_document_its_avoir_is_written_against() {
    // What lets `avoir::Remaining` read one facture's credited quantities off
    // the avoirs written against that facture alone: a line that credits
    // another names a line of the document its own document refers to, and
    // nothing else can be written. Without this a line of facture A could be
    // credited on an avoir written against facture B, and A's remaining
    // quantities would have to be summed across every avoir in the shop.
    let (_dir, mut conn) = open_temp();
    let p = a_product(&mut conn, "Sucre");
    let a = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Facture, Some(p), at(9, 10)),
    )
    .unwrap();
    let b = documents::issue(
        &mut conn,
        SHOP,
        draft(DocumentKind::Facture, Some(p), at(9, 11)),
    )
    .unwrap();

    // An avoir written against B, crediting a line of A.
    let mut stolen = draft(DocumentKind::Avoir, Some(p), at(9, 12));
    stolen.ref_document_id = Some(b.id);
    stolen.lines[0].ref_line_id = Some(a.lines[0].id);
    let err = documents::issue(&mut conn, SHOP, stolen).unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::NotFound {
                entity: "document_line",
                ..
            }
        ),
        "{err:?}"
    );

    // And a credited line on a document written against nothing at all is
    // refused on the field the caller sent, because there is no document for
    // the line to belong to. On a ticket, since an avoir carrying no
    // reference is refused by the table itself (migration 7).
    let mut loose = draft(DocumentKind::Ticket, Some(p), at(9, 12));
    loose.lines[0].ref_line_id = Some(b.lines[0].id);
    let err = documents::issue(&mut conn, SHOP, loose).unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "ref_line_id"),
        "{err:?}"
    );

    // The line of B on an avoir written against B is the one that stands.
    let mut proper = draft(DocumentKind::Avoir, Some(p), at(9, 12));
    proper.ref_document_id = Some(b.id);
    proper.lines[0].ref_line_id = Some(b.lines[0].id);
    let avoir = documents::issue(&mut conn, SHOP, proper).unwrap();
    assert_eq!(avoir.lines[0].ref_line_id, Some(b.lines[0].id));
}

/// Every centime of a credit sale to a named customer, written out by hand
/// from features.md §1 and §3 rather than read off the code.
///
/// It lives here and not in `sales_service.rs` because this is the file that
/// owns `documents::issue`, and the sale is what carries a customer through
/// it. The reason it exists at all: `NewDocument::customer` stopped being an
/// `Option<i32>` and became a `ProvedCustomer`, which is meant to move no
/// figure whatsoever. A refactor that quietly shifted the TVA or the triple
/// would still pass every test that reads its expectation off
/// `doc.totals.net_to_pay`, so this one reads nothing off the document.
#[test]
fn a_credit_sale_to_a_named_customer_prints_the_figures_it_always_did() {
    use dzpos_retail::services::sales::{self, NewSale, NewSaleLine, SaleKind};

    let (_dir, mut conn) = open_temp();
    let p = products::create(
        &mut conn,
        SHOP,
        OWNER,
        NewProduct {
            name: "Ciment".to_string(),
            barcode: None,
            category_id: Some(1),
            unit: Unit::Kg,
            cost: Money::centimes(10_000),
            selling: Money::centimes(14_670),
            wholesale: None,
            qty_on_hand_milli: 10_000,
            low_stock_at_milli: 0,
            rate_bps: Some(Bps::new(1900).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id;
    let customer = a_customer(&mut conn);

    let sale = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![NewSaleLine {
                product_id: p,
                qty_milli: 1_500,
                unit_price: None,
                line_discount: Money::ZERO,
            }],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Credit,
            tendered: None,
            customer_id: Some(customer),
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(at(9, 10)),
        },
    )
    .unwrap();
    let doc = sale.document;

    // 146,70 DA the kilo and a kilo and a half of it: 220,05 DA HT, and the
    // line is exact, so nothing is rounded before the rate group.
    assert_eq!(doc.lines[0].line_total, Money::centimes(22_005));
    assert_eq!(doc.totals.total_ht, Money::centimes(22_005));
    assert_eq!(doc.totals.discount, Money::ZERO);
    assert_eq!(doc.totals.subtotal_ht, Money::centimes(22_005));
    // 19 % of 220,05 is 41,8095 DA, rounded once on the one rate group:
    // 41,81. The fraction is .95 and not a half, so what this pins is that
    // the rate group is rounded at all rather than truncated to 41,80; every
    // rounding mode agrees here. Which way a true half goes is
    // `Money::pct`'s own rule and the fixture it names,
    // `tva_rounding_once_per_rate`.
    assert_eq!(doc.totals.tva, Money::centimes(4_181));
    assert_eq!(doc.totals.total_ttc, Money::centimes(26_186));
    // A credit sale hands over no cash, and the droit de timbre is a tax on
    // cash, so none is due and the net is the TTC unchanged.
    assert_eq!(doc.totals.stamp, Money::ZERO);
    assert_eq!(doc.totals.net_to_pay, Money::centimes(26_186));
    assert_eq!(doc.tendered, None);
    assert_eq!(doc.change, None);

    // The document names the fiche it was proved against, and the triple is
    // "from the ledger at issue time" (features.md §3): nothing owed before,
    // the whole of this paper owed now.
    assert_eq!(doc.customer_id, Some(customer));
    let balance = doc.balance.expect("a named customer has a balance triple");
    assert_eq!(balance.old_balance, Money::ZERO);
    assert_eq!(balance.remaining_debt, Money::centimes(26_186));
    assert_eq!(balance.total_debt, Money::centimes(26_186));

    // Read back off the file, never trusted from the return value.
    assert_eq!(documents::get(&mut conn, SHOP, doc.id).unwrap(), doc);
}
