// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Cancelling a document (features.md §3). A cancelled document keeps its
//! number and its row so the series never gaps, and what it stops doing is
//! asking for its amount and holding the goods off the shelf.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::models::shop::StoreBlock;
use dzpos_core::models::stock::MovementKind;
use dzpos_core::money::{Bps, Money, PaymentMode};
use dzpos_core::services::customers::{NewCustomer, PartyKind};
use dzpos_core::services::debt::{DebtKind, PaymentMethod};
use dzpos_core::services::documents::{Document, DocumentKind, DocumentStatus};
use dzpos_core::services::sales::{self, NewSale, NewSaleLine, SaleKind};
use dzpos_core::services::{audit, avoir, customers, debt, documents, products, shops, stock};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = dzpos_core::db::open(&path).unwrap();
    shops::update_store(
        &mut conn,
        SHOP,
        OWNER,
        StoreBlock {
            name: "Mon magasin".to_string(),
            rc: Some("16/00-1234567 B 25".to_string()),
            nif: None,
            nis: Some("000216001234567 00".to_string()),
            ai: None,
            address: None,
            phone: None,
        },
    )
    .unwrap();
    (dir, conn)
}

fn at(day: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, day)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

fn product(conn: &mut SqliteConnection, name: &str, selling: i64) -> i32 {
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
            rate_bps: Some(Bps::new(0).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id
}

fn a_customer(conn: &mut SqliteConnection) -> i32 {
    customers::create(
        conn,
        SHOP,
        OWNER,
        NewCustomer {
            name: "Entreprise Benali".to_string(),
            party_kind: PartyKind::Company,
            phone: None,
            address: None,
            rc: Some("16/00-7654321 B 22".to_string()),
            nif: None,
            nis: Some("000216007654321 00".to_string()),
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

fn line(product_id: i32, qty_milli: i64) -> NewSaleLine {
    NewSaleLine {
        product_id,
        qty_milli,
        unit_price: None,
        line_discount: Money::ZERO,
    }
}

fn sell(
    conn: &mut SqliteConnection,
    customer_id: Option<i32>,
    lines: Vec<NewSaleLine>,
    mode: PaymentMode,
    kind: SaleKind,
    day: u32,
) -> Document {
    sales::issue(
        conn,
        SHOP,
        OWNER,
        NewSale {
            lines,
            global_discount: Money::ZERO,
            payment_mode: mode,
            tendered: match mode {
                PaymentMode::Cash => Some(Money::centimes(10_000_000)),
                _ => None,
            },
            customer_id,
            override_credit: false,
            kind,
            issued_at: Some(at(day)),
        },
    )
    .unwrap()
    .document
}

fn kinds(conn: &mut SqliteConnection, product_id: i32) -> Vec<(MovementKind, i64)> {
    stock::list_for_product(conn, SHOP, product_id)
        .unwrap()
        .into_iter()
        .map(|m| (m.kind, m.qty_milli))
        .collect()
}

#[test]
fn a_cash_ticket_is_cancelled_the_stock_comes_back_and_the_number_stays() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000);
    let ticket = sell(
        &mut conn,
        None,
        vec![line(p, 2_000)],
        PaymentMode::Cash,
        SaleKind::Ticket,
        10,
    );

    let cancelled = documents::cancel(
        &mut conn,
        SHOP,
        OWNER,
        ticket.id,
        "erreur de saisie".to_string(),
        Some(at(11)),
    )
    .unwrap();

    assert_eq!(cancelled.status, DocumentStatus::Cancelled);
    assert_eq!(
        cancelled.number, ticket.number,
        "a cancelled document keeps its number so the series never gaps"
    );
    let block = cancelled
        .cancellation
        .clone()
        .expect("the block is written");
    assert_eq!(block.at, at(11));
    assert_eq!(block.by, OWNER);
    assert_eq!(block.reason, "erreur de saisie");
    // Nobody owed anything on a cash ticket, so there is nothing to write a
    // credit note for: the goods coming back are the whole of it.
    assert_eq!(block.avoir_document_id, None);
    assert_eq!(
        documents::list(&mut conn, SHOP, Some(DocumentKind::Avoir))
            .unwrap()
            .len(),
        0
    );

    assert_eq!(
        kinds(&mut conn, p),
        vec![
            (MovementKind::Opening, 100_000),
            (MovementKind::Sale, -2_000),
            (MovementKind::Return, 2_000),
        ]
    );
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        100_000
    );
    assert_eq!(
        documents::get(&mut conn, SHOP, ticket.id).unwrap(),
        cancelled
    );
}

#[test]
fn a_facture_carrying_debt_is_cancelled_through_a_whole_avoir() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000);
    let c = a_customer(&mut conn);
    let facture = sell(
        &mut conn,
        Some(c),
        vec![line(p, 2_000)],
        PaymentMode::Credit,
        SaleKind::Facture,
        10,
    );
    assert_eq!(
        debt::balance(&mut conn, SHOP, c).unwrap(),
        Money::centimes(200_000)
    );

    let cancelled = documents::cancel(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "commande annulée".to_string(),
        Some(at(11)),
    )
    .unwrap();

    // The number stays and the block names the avoir the cancellation wrote.
    assert_eq!(cancelled.status, DocumentStatus::Cancelled);
    assert_eq!(cancelled.number, facture.number);
    let block = cancelled
        .cancellation
        .clone()
        .expect("the block is written");
    let avoir_id = block
        .avoir_document_id
        .expect("a facture carrying debt is cancelled through an avoir");
    let avoir = documents::get(&mut conn, SHOP, avoir_id).unwrap();
    assert_eq!(avoir.kind, DocumentKind::Avoir);
    assert_eq!(avoir.ref_document_id, Some(facture.id));
    assert_eq!(avoir.totals.net_to_pay, facture.totals.net_to_pay);
    assert_eq!(
        avoir.issued_at,
        at(11),
        "the avoir is dated as the cancellation"
    );

    // The debt is gone and the goods are back, both through the avoir.
    assert_eq!(debt::balance(&mut conn, SHOP, c).unwrap(), Money::ZERO);
    assert_eq!(
        debt::ledger(&mut conn, SHOP, c)
            .unwrap()
            .iter()
            .filter(|e| e.kind == DebtKind::Avoir)
            .count(),
        1
    );
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        100_000
    );
    // And the stock came back once, on the avoir, not twice.
    assert_eq!(
        kinds(&mut conn, p)
            .iter()
            .filter(|(k, _)| *k == MovementKind::Return)
            .count(),
        1
    );
}

#[test]
fn a_facture_already_paid_is_cancelled_and_leaves_the_customer_in_credit() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000);
    let c = a_customer(&mut conn);
    let facture = sell(
        &mut conn,
        Some(c),
        vec![line(p, 1_000)],
        PaymentMode::Credit,
        SaleKind::Facture,
        10,
    );
    debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        c,
        Money::centimes(100_000),
        PaymentMethod::Cash,
        None,
        at(11),
    )
    .unwrap();

    documents::cancel(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "retour".to_string(),
        Some(at(12)),
    )
    .unwrap();

    // The customer paid and the paper was then annulled, so the shop is
    // holding their money: a payment is not undone, it is carried forward.
    assert_eq!(
        debt::balance(&mut conn, SHOP, c).unwrap(),
        Money::centimes(-100_000)
    );
}

#[test]
fn a_document_is_cancelled_once_and_an_avoir_or_a_proforma_never() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000);
    let c = a_customer(&mut conn);
    let facture = sell(
        &mut conn,
        Some(c),
        vec![line(p, 1_000)],
        PaymentMode::Credit,
        SaleKind::Facture,
        10,
    );
    let avoir = avoir::issue(&mut conn, SHOP, OWNER, facture.id, None, None, Some(at(11))).unwrap();

    // An avoir is the instrument that undoes a facture; undoing it in turn
    // would be a second reversal nobody can read against the first.
    let err = documents::cancel(
        &mut conn,
        SHOP,
        OWNER,
        avoir.id,
        "erreur".to_string(),
        Some(at(12)),
    )
    .unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "document_id"),
        "{err:?}"
    );

    // A second cancellation of the same document is refused: the first one is
    // what the paper says, and a second would overwrite who took it and why.
    documents::cancel(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "commande annulée".to_string(),
        Some(at(12)),
    )
    .unwrap();
    let again = documents::cancel(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "encore".to_string(),
        Some(at(13)),
    )
    .unwrap_err();
    assert!(
        matches!(again, CoreError::Validation { ref field, .. } if field == "document_id"),
        "{again:?}"
    );
    // The first cancellation is untouched.
    let stored = documents::get(&mut conn, SHOP, facture.id).unwrap();
    assert_eq!(
        stored.cancellation.expect("the block").reason,
        "commande annulée"
    );
}

#[test]
fn a_facture_already_credited_in_full_is_cancelled_without_a_second_avoir() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000);
    let c = a_customer(&mut conn);
    let facture = sell(
        &mut conn,
        Some(c),
        vec![line(p, 1_000)],
        PaymentMode::Credit,
        SaleKind::Facture,
        10,
    );
    avoir::issue(&mut conn, SHOP, OWNER, facture.id, None, None, Some(at(11))).unwrap();

    let cancelled = documents::cancel(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "déjà avoirée".to_string(),
        Some(at(12)),
    )
    .unwrap();

    // Everything was already carried back, so the cancellation writes no
    // second credit note and moves no second lot of stock: it says the paper
    // no longer stands and nothing more.
    assert_eq!(cancelled.status, DocumentStatus::Cancelled);
    assert_eq!(
        cancelled.cancellation.expect("the block").avoir_document_id,
        None
    );
    assert_eq!(
        documents::list(&mut conn, SHOP, Some(DocumentKind::Avoir))
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        100_000
    );
    assert_eq!(debt::balance(&mut conn, SHOP, c).unwrap(), Money::ZERO);
}

#[test]
fn a_cancellation_is_audited_and_another_shops_document_is_not_found() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000);
    let ticket = sell(
        &mut conn,
        None,
        vec![line(p, 1_000)],
        PaymentMode::Cash,
        SaleKind::Ticket,
        10,
    );

    let err = documents::cancel(
        &mut conn,
        2,
        OWNER,
        ticket.id,
        "x".to_string(),
        Some(at(11)),
    )
    .unwrap_err();
    assert!(
        matches!(err, CoreError::NotFound { entity, .. } if entity == "document"),
        "{err:?}"
    );

    documents::cancel(
        &mut conn,
        SHOP,
        OWNER,
        ticket.id,
        "erreur de saisie".to_string(),
        Some(at(11)),
    )
    .unwrap();

    let entry = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .find(|e| e.action == "document.cancel")
        .expect("the cancellation wrote no audit entry");
    assert_eq!(entry.entity, "document");
    assert_eq!(entry.entity_id, Some(ticket.id));
    assert_eq!(entry.user_id, OWNER);
    let before: serde_json::Value = serde_json::from_str(&entry.before.unwrap()).unwrap();
    let after: serde_json::Value = serde_json::from_str(&entry.after.unwrap()).unwrap();
    assert_eq!(before["status"], "issued");
    assert_eq!(after["status"], "cancelled");
    assert_eq!(after["reason"], "erreur de saisie");
    assert_eq!(after["avoir_document_id"], serde_json::Value::Null);
}

#[test]
fn a_cancellation_needs_a_reason() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000);
    let ticket = sell(
        &mut conn,
        None,
        vec![line(p, 1_000)],
        PaymentMode::Cash,
        SaleKind::Ticket,
        10,
    );
    // A document annulled for no stated reason is exactly what the log exists
    // to prevent, and a field of spaces is no reason.
    let err = documents::cancel(
        &mut conn,
        SHOP,
        OWNER,
        ticket.id,
        "   ".to_string(),
        Some(at(11)),
    )
    .unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "reason"),
        "{err:?}"
    );
    assert_eq!(
        documents::get(&mut conn, SHOP, ticket.id).unwrap().status,
        DocumentStatus::Issued
    );
    assert_eq!(
        kinds(&mut conn, p)
            .iter()
            .filter(|(k, _)| *k == MovementKind::Return)
            .count(),
        0,
        "a refused cancellation moved stock"
    );
}
