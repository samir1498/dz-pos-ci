// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Cancelling a document (features.md §3). A cancelled document keeps its
//! number and its row so the series never gaps, and what it stops doing is
//! asking for its amount and holding the goods off the shelf.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::services::audit;
use dzpos_retail::error::{CoreError, RetailError};
use dzpos_retail::models::product::{NewProduct, Unit};
use dzpos_retail::models::stock::MovementKind;
use dzpos_retail::money::{Bps, Money, PaymentMode};
use dzpos_retail::services::customers::NewCustomer;
use dzpos_retail::services::debt::{DebtKind, PaymentMethod};
use dzpos_retail::services::documents::{Document, DocumentKind, DocumentStatus};
use dzpos_retail::services::sales::{self, NewSale, NewSaleLine, SaleKind};
use dzpos_retail::services::{avoir, cancellation, customers, debt, documents, products, stock};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

use common::open_temp_selling_factures as open_temp;

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

mod common;

/// The buyer of every facture in this file, carrying the identifiers
/// décret 05-468 art. 3 asks of one.
fn a_customer(conn: &mut SqliteConnection) -> i32 {
    common::an_identified_customer(conn, "Entreprise Benali")
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

    let cancelled = cancellation::cancel(
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

    let cancelled = cancellation::cancel(
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

    cancellation::cancel(
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
    let err = cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        avoir.id,
        "erreur".to_string(),
        Some(at(12)),
    )
    .unwrap_err();
    assert!(
        matches!(err, dzpos_retail::error::RetailError::Kernel(CoreError::Validation { ref field, .. }) if field == "document_id"),
        "{err:?}"
    );

    // A second cancellation of the same document is refused: the first one is
    // what the paper says, and a second would overwrite who took it and why.
    cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "commande annulée".to_string(),
        Some(at(12)),
    )
    .unwrap();
    let again = cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "encore".to_string(),
        Some(at(13)),
    )
    .unwrap_err();
    assert!(
        matches!(again, dzpos_retail::error::RetailError::Kernel(CoreError::Validation { ref field, .. }) if field == "document_id"),
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

    let cancelled = cancellation::cancel(
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

    let err = cancellation::cancel(
        &mut conn,
        2,
        OWNER,
        ticket.id,
        "x".to_string(),
        Some(at(11)),
    )
    .unwrap_err();
    assert!(
        matches!(err, dzpos_retail::error::RetailError::Kernel(CoreError::NotFound { entity, .. }) if entity == "document"),
        "{err:?}"
    );

    cancellation::cancel(
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
    let err = cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        ticket.id,
        "   ".to_string(),
        Some(at(11)),
    )
    .unwrap_err();
    assert!(
        matches!(err, dzpos_retail::error::RetailError::Kernel(CoreError::Validation { ref field, .. }) if field == "reason"),
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

/// A credit ticket is a real document: the credit sale issues one when the
/// buyer is not a company and the shop is putting it on the slate anyway.
/// There is no avoir to write against it, because an avoir is written against
/// a facture, so the money comes off the account as a ledger row that names
/// the ticket and carries no number of its own.
#[test]
fn a_credit_ticket_is_cancelled_by_a_ledger_row_and_not_by_an_avoir() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000);
    let c = a_customer(&mut conn);
    let ticket = sell(
        &mut conn,
        Some(c),
        vec![line(p, 3_000)],
        PaymentMode::Credit,
        SaleKind::Ticket,
        10,
    );
    assert_eq!(ticket.kind, DocumentKind::Ticket);
    assert_eq!(
        debt::balance(&mut conn, SHOP, c).unwrap(),
        Money::centimes(300_000)
    );

    let cancelled = cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        ticket.id,
        "erreur de caisse".to_string(),
        Some(at(11)),
    )
    .unwrap();

    assert_eq!(cancelled.status, DocumentStatus::Cancelled);
    assert_eq!(cancelled.number, ticket.number, "the number stays");
    assert_eq!(
        cancelled
            .cancellation
            .as_ref()
            .and_then(|c| c.avoir_document_id),
        None,
        "no numbered credit note is written against a ticket"
    );
    assert_eq!(
        documents::list(&mut conn, SHOP, Some(DocumentKind::Avoir))
            .unwrap()
            .len(),
        0
    );

    // The goods are back and the account is square.
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        100_000
    );
    assert!(kinds(&mut conn, p).contains(&(MovementKind::Return, 3_000)));
    assert_eq!(debt::balance(&mut conn, SHOP, c).unwrap(), Money::ZERO);
    assert_eq!(
        documents::get(&mut conn, SHOP, ticket.id)
            .unwrap()
            .balance
            .map(|b| b.remaining_debt),
        Some(Money::ZERO)
    );

    // The reversal is one ledger row of kind avoir naming the ticket.
    let rows = debt::ledger(&mut conn, SHOP, c).unwrap();
    let reversal = rows
        .iter()
        .find(|e| e.kind == DebtKind::Avoir)
        .expect("the ticket's money came off the account");
    assert_eq!(reversal.credit, Money::centimes(300_000));
    assert_eq!(reversal.document_id, Some(ticket.id));
}

/// What the customer had already paid on a credit ticket does not vanish with
/// it: the goods go back, the whole of the ticket comes off the account, and
/// the money they handed over is theirs to spend, the same way an avoir's
/// excess is.
#[test]
fn a_paid_credit_ticket_leaves_the_customer_holding_what_they_paid() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000);
    let c = a_customer(&mut conn);
    let ticket = sell(
        &mut conn,
        Some(c),
        vec![line(p, 3_000)],
        PaymentMode::Credit,
        SaleKind::Ticket,
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
    assert_eq!(
        debt::balance(&mut conn, SHOP, c).unwrap(),
        Money::centimes(200_000)
    );

    cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        ticket.id,
        "erreur de caisse".to_string(),
        Some(at(12)),
    )
    .unwrap();

    assert_eq!(
        debt::balance(&mut conn, SHOP, c).unwrap(),
        Money::centimes(-100_000),
        "the 1 000,00 they paid is credit the shop is holding"
    );
}

/// The kinds a cancellation takes are named, not the ones it refuses. A
/// quittance and a bon are papers about something that already happened, and
/// nothing here knows how to undo one.
#[test]
fn only_a_ticket_and_a_facture_are_cancelled() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000);
    let c = a_customer(&mut conn);
    let quittance = a_document_of_kind(&mut conn, DocumentKind::Quittance, c);
    let err = cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        quittance,
        "erreur".to_string(),
        Some(at(11)),
    )
    .unwrap_err();
    assert!(
        matches!(err, dzpos_retail::error::RetailError::Kernel(CoreError::Validation { ref field, .. }) if field == "document_id"),
        "{err:?}"
    );

    // And a ticket and a facture still go through.
    let ticket = sell(
        &mut conn,
        None,
        vec![line(p, 1_000)],
        PaymentMode::Cash,
        SaleKind::Ticket,
        10,
    );
    cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        ticket.id,
        "erreur".to_string(),
        Some(at(11)),
    )
    .unwrap();
}

/// What the screen has to say before it asks. Read off the core, because a
/// screen re-deriving it gets a fully credited facture wrong: it carries debt
/// and still nothing happens.
#[test]
fn the_effect_of_a_cancellation_is_answered_before_it_is_taken() {
    use dzpos_retail::services::cancellation::CancelEffect;
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Bougie", 50);
    let c = a_customer(&mut conn);

    // A cash ticket: the goods and nothing else.
    let ticket = sell(
        &mut conn,
        None,
        vec![line(p, 3_000)],
        PaymentMode::Cash,
        SaleKind::Ticket,
        10,
    );
    assert_eq!(
        cancellation::cancel_effect(&mut conn, SHOP, ticket.id).unwrap(),
        CancelEffect::StockBack
    );

    // A facture on credit: the goods and a credit note of what is left.
    let facture = sell(
        &mut conn,
        Some(c),
        vec![line(p, 3_000)],
        PaymentMode::Credit,
        SaleKind::Facture,
        11,
    );
    assert_eq!(
        cancellation::cancel_effect(&mut conn, SHOP, facture.id).unwrap(),
        CancelEffect::StockBackAndAvoir {
            amount: facture.totals.total_ttc
        }
    );

    // Credited in part, and the figure follows what is left rather than what
    // a slice of the remaining line would come to: 150 - 100 and not 50.
    avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        Some(vec![dzpos_retail::services::avoir::AvoirLine {
            document_line_id: facture.lines[0].id,
            qty_milli: 2_000,
        }]),
        None,
        Some(at(12)),
    )
    .unwrap();
    let left = cancellation::cancel_effect(&mut conn, SHOP, facture.id).unwrap();
    let credited: i64 = avoir::list_for(&mut conn, SHOP, facture.id)
        .unwrap()
        .iter()
        .map(|a| a.totals.net_to_pay.as_centimes())
        .sum();
    assert_eq!(
        left,
        CancelEffect::StockBackAndAvoir {
            amount: Money::centimes(facture.totals.total_ttc.as_centimes() - credited)
        }
    );

    // Credited in full: nothing left to undo at all.
    avoir::issue(&mut conn, SHOP, OWNER, facture.id, None, None, Some(at(13))).unwrap();
    assert_eq!(
        cancellation::cancel_effect(&mut conn, SHOP, facture.id).unwrap(),
        CancelEffect::NothingToReverse
    );
}

/// A document of a kind the till never writes, put in the file by hand so the
/// allowlist can be asked about it.
fn a_document_of_kind(conn: &mut SqliteConnection, kind: DocumentKind, customer: i32) -> i32 {
    use diesel::prelude::*;
    diesel::sql_query(format!(
        "INSERT INTO documents (shop_id, kind, series, number, issued_at, user_id, regime, \
         payment_mode, seller_name, customer_id, total_ht_centimes, discount_centimes, \
         subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes, \
         net_to_pay_centimes, status) \
         VALUES (1, '{}', 'doc_{}', 1, '2026-09-10 10:00:00', 1, 'reel', 'cash', \
         'Mon magasin', {customer}, 0, 0, 0, 0, 0, 0, 0, 'issued')",
        kind.as_str(),
        kind.as_str()
    ))
    .execute(conn)
    .unwrap();
    documents::list(conn, SHOP, Some(kind)).unwrap()[0].id
}

/// Everything the log holds about one document comes back from one query,
/// because every row about a document says `document` in `entity` and names
/// it in `entity_id`. The override row used to say `sale`, so the history of
/// a facture was two queries and a reader had to know that.
///
/// The story is a mixed one on purpose, because a real customer's is: a
/// facture sold on credit past the limit, money taken against it, the balance
/// corrected by hand, the goods part-returned, the paper annulled, and the
/// fiche closed at the end with money still on it. Six kinds of event, three
/// entities, one read of the log, and every action name asserted — a rename
/// that missed one of them goes red here.
#[test]
fn the_life_of_a_document_reads_back_from_one_query() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000);
    let customer = a_customer(&mut conn);
    // A limit of nothing, so the sale needs the override to go through.
    let open = customers::get(&mut conn, SHOP, customer).unwrap();
    customers::update(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        NewCustomer {
            name: open.name,
            party_kind: open.party_kind,
            phone: open.phone,
            address: open.address,
            rc: open.rc,
            nif: open.nif,
            nis: open.nis,
            ai: open.ai,
            credit_limit: Some(Money::ZERO),
            warn_threshold: None,
            notes: open.notes,
            active: true,
        },
        None,
    )
    .unwrap();

    let facture = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(p, 2_000)],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Credit,
            tendered: None,
            customer_id: Some(customer),
            override_credit: true,
            kind: SaleKind::Facture,
            issued_at: Some(at(10)),
        },
    )
    .unwrap()
    .document;
    // Money against the facture, then a correction upwards on the balance
    // itself: one settles paper, the other does not, and they are two
    // different action names in the log.
    debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(50_000),
        PaymentMethod::Cash,
        Some("acompte".to_string()),
        at(10),
    )
    .unwrap();
    debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(25_000),
        Some("report du carnet".to_string()),
    )
    .unwrap();
    avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        Some(vec![dzpos_retail::services::avoir::AvoirLine {
            document_line_id: facture.lines[0].id,
            qty_milli: 1_000,
        }]),
        None,
        Some(at(11)),
    )
    .unwrap();
    cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "erreur de saisie".to_string(),
        Some(at(12)),
    )
    .unwrap();

    // The cancellation put the facture's own money back but left the
    // correction and the payment where they were, so the fiche still has an
    // account open and closing it has to say why.
    let open = customers::get(&mut conn, SHOP, customer).unwrap();
    customers::update(
        &mut conn,
        SHOP,
        OWNER,
        customer,
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
        Some("compte soldé au carnet".to_string()),
    )
    .unwrap();

    // One read of the log. Everything below is that one answer, sorted by the
    // entity each row names.
    let log = audit::list(&mut conn, SHOP).unwrap();
    let actions = |entity: &str, id: i32| -> Vec<String> {
        log.iter()
            .filter(|e| e.entity == entity && e.entity_id == Some(id))
            .map(|e| e.action.clone())
            .collect()
    };

    // Two credit notes: the partial the shop wrote, and the closing one the
    // cancellation wrote to take back what was left.
    //
    // The first row is the till tag: this fixture opens no shift before
    // ringing, so the sale falls inside none of its ringer's own windows and
    // `services::shifts::tag_if_outside_a_shift` marks it (plan
    // `till-shifts-a-float-and-a-count` ruling 2). It belongs in this list
    // rather than being filtered out of it — the point of the test is that
    // one query answers the whole life of the paper, and a tag saying its
    // money was never in anybody's counted drawer is part of that life.
    assert_eq!(
        actions("document", facture.id),
        [
            "till.sale_outside_shift",
            "document.issue_override",
            "document.avoir",
            "document.avoir",
            "document.cancel"
        ],
        "the life of a facture is not one query away"
    );
    // The money that moved on the account rather than on the paper.
    assert_eq!(
        actions("customer_debt", customer),
        ["debt.pay", "debt.adjust"],
        "the ledger's own events are not under the customer they belong to"
    );
    // And the fiche: opened, its limit set, closed over an account that was
    // still open. The close is its own name because it is its own decision.
    assert_eq!(
        actions("customer", customer),
        ["create", "update", "customer.close"],
        "the fiche's own events are not under the customer they belong to"
    );

    // The cancellation says what the paper is left asking for, so a reader can
    // tell a cancellation that moved money from one that moved none.
    let cancelled = log
        .iter()
        .find(|e| e.action == "document.cancel")
        .cloned()
        .expect("the cancellation is logged");
    let after: serde_json::Value =
        serde_json::from_str(&cancelled.after.unwrap_or_default()).unwrap();
    assert_eq!(after["remaining_debt_centimes"], 0);
    assert_eq!(after["reason"], "erreur de saisie");
}

/// Moves the fiche's cost the way a delivery does, leaving the rest of the
/// product as `product` created it.
fn move_the_cost(conn: &mut SqliteConnection, id: i32, name: &str, selling: i64, cost: i64) {
    products::update(
        conn,
        SHOP,
        OWNER,
        id,
        NewProduct {
            name: name.to_string(),
            barcode: None,
            category_id: None,
            unit: Unit::Piece,
            cost: Money::centimes(cost),
            selling: Money::centimes(selling),
            wholesale: None,
            qty_on_hand_milli: 0,
            low_stock_at_milli: 0,
            rate_bps: Some(Bps::new(0).unwrap()),
            active: true,
        },
    )
    .unwrap();
}

/// A cancellation puts the goods back at what they left on, not at what the
/// fiche says the day somebody annulled the ticket. Same rule as the avoir's,
/// and its own path: a ticket carries no credit note, so the movements are
/// written here rather than by `avoir::issue`.
#[test]
fn a_cancelled_ticket_returns_the_goods_at_the_cost_of_the_sale() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000);
    let sold_at = products::get(&mut conn, SHOP, p).unwrap().cost;
    let ticket = sell(
        &mut conn,
        None,
        vec![line(p, 2_000)],
        PaymentMode::Cash,
        SaleKind::Ticket,
        10,
    );

    let now_worth = Money::centimes(90_000);
    move_the_cost(&mut conn, p, "Ciment", 100_000, now_worth.as_centimes());
    assert_ne!(sold_at, now_worth, "the fiche has to have moved");

    cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        ticket.id,
        "erreur de saisie".to_string(),
        Some(at(11)),
    )
    .unwrap();

    let back = stock::list_for_product(&mut conn, SHOP, p)
        .unwrap()
        .into_iter()
        .find(|m| m.kind == MovementKind::Return)
        .expect("the goods came back on a return movement");
    assert_eq!(
        back.unit_cost, sold_at,
        "the cancellation carried today's cost instead of the one the goods left on"
    );
}

/// The cancellation's own path to the same rule: a ticket carries no credit
/// note, so the movements are written by `documents` rather than by
/// `avoir::issue`, and a sold line with no movement left is refused there too.
#[test]
fn a_cancelled_ticket_whose_movement_is_gone_is_refused_rather_than_priced_off_the_fiche() {
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
    diesel::sql_query(format!(
        "DELETE FROM stock_movements WHERE document_id = {} AND kind = 'sale'",
        ticket.id
    ))
    .execute(&mut conn)
    .unwrap();

    let refused = cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        ticket.id,
        "erreur de saisie".to_string(),
        Some(at(11)),
    );
    assert!(
        matches!(
            refused,
            Err(RetailError::UnpricedReversal { product_id, .. }) if product_id == p
        ),
        "a missing movement was priced off the fiche: {refused:?}"
    );
    // The whole cancellation rolled back: the ticket still stands.
    assert_eq!(
        documents::get(&mut conn, SHOP, ticket.id).unwrap().status,
        DocumentStatus::Issued
    );
}
