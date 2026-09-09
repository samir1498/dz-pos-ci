// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The avoir (features.md §3), against a real temp SQLite file. What is
//! asserted is the stored avoir, the stored stock and the stored ledger, never
//! a return value alone: an avoir that answered correctly and wrote nothing
//! would be a credit note the customer holds and the shop has no record of.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::models::shop::StoreBlock;
use dzpos_core::models::stock::MovementKind;
use dzpos_core::money::{Bps, Money, PaymentMode};
use dzpos_core::services::avoir::{self, AvoirLine};
use dzpos_core::services::customers::{NewCustomer, PartyKind};
use dzpos_core::services::debt::{DebtKind, PaymentMethod};
use dzpos_core::services::documents::{Document, DocumentKind, DocumentStatus};
use dzpos_core::services::sales::{self, NewSale, NewSaleLine, SaleKind};
use dzpos_core::services::{audit, customers, debt, documents, products, shops, stock};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = dzpos_core::db::open(&path).unwrap();
    // A facture needs the seller's own identifiers before it may take a
    // number (`facture_requires_party_ids`).
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

/// A facture on credit, which is what an avoir is written against.
fn a_facture(
    conn: &mut SqliteConnection,
    customer_id: i32,
    lines: Vec<NewSaleLine>,
    mode: PaymentMode,
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
                PaymentMode::Cash => Some(Money::centimes(1_000_000)),
                _ => None,
            },
            customer_id: Some(customer_id),
            override_credit: false,
            kind: SaleKind::Facture,
            issued_at: Some(at(day)),
        },
    )
    .unwrap()
    .document
}

fn movements(
    conn: &mut SqliteConnection,
    product_id: i32,
) -> Vec<(MovementKind, i64, Option<i32>)> {
    stock::list_for_product(conn, SHOP, product_id)
        .unwrap()
        .into_iter()
        .map(|m| (m.kind, m.qty_milli, m.document_id))
        .collect()
}

#[test]
fn a_whole_avoir_credits_the_facture_takes_its_own_number_and_carries_no_stamp() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 1900);
    let c = a_customer(&mut conn);
    // A cash facture, so the facture itself carries the droit de timbre and
    // the avoir's own zero has to be the rule doing it rather than the mode.
    let facture = a_facture(&mut conn, c, vec![line(p, 3_000)], PaymentMode::Cash, 10);
    assert!(facture.totals.stamp > Money::ZERO, "no stamp to not refund");

    let avoir = avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        None,
        Some("retour marchandise".to_string()),
        Some(at(11)),
    )
    .unwrap();

    assert_eq!(avoir.kind, DocumentKind::Avoir);
    assert_eq!(avoir.series, "doc_avoir");
    assert_eq!(avoir.number, 1, "the avoir took a number of its own series");
    assert_eq!(avoir.ref_document_id, Some(facture.id));
    assert_eq!(avoir.customer_id, Some(c));
    assert_eq!(avoir.issued_at, at(11));
    // Every line of the facture, at the price and the rate it was sold at.
    assert_eq!(avoir.lines.len(), 1);
    assert_eq!(avoir.lines[0].qty_milli, 3_000);
    assert_eq!(avoir.lines[0].unit_price, facture.lines[0].unit_price);
    assert_eq!(avoir.lines[0].rate_bps, facture.lines[0].rate_bps);
    assert_eq!(avoir.lines[0].ref_line_id, Some(facture.lines[0].id));

    // The TVA of the facture comes back with it, per rate on its own lines,
    // and the stamp does not: a droit de timbre is paid on the money that
    // changed hands and is not refunded with the goods.
    assert_eq!(avoir.totals.total_ht, facture.totals.total_ht);
    assert_eq!(avoir.totals.tva, facture.totals.tva);
    assert_eq!(avoir.totals.tva_by_rate, facture.totals.tva_by_rate);
    assert_eq!(avoir.totals.total_ttc, facture.totals.total_ttc);
    assert_eq!(avoir.totals.stamp, Money::ZERO);
    assert_eq!(avoir.totals.net_to_pay, facture.totals.total_ttc);

    // The triple on an avoir is its whole effect on the account, and its
    // three figures agree with each other on every avoir the shop issues.
    let balance = avoir.balance.expect("an avoir names a customer");
    assert_eq!(
        balance.remaining_debt,
        Money::ZERO.checked_sub(avoir.totals.net_to_pay).unwrap()
    );
    assert_eq!(
        balance
            .old_balance
            .checked_add(balance.remaining_debt)
            .unwrap(),
        balance.total_debt
    );

    // The goods came back as a return movement naming the avoir.
    let moves = movements(&mut conn, p);
    assert_eq!(
        moves,
        vec![
            // The opening count the product was created with, then the sale,
            // then the goods coming back on the avoir.
            (MovementKind::Opening, 100_000, None),
            (MovementKind::Sale, -3_000, Some(facture.id)),
            (MovementKind::Return, 3_000, Some(avoir.id)),
        ]
    );
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        100_000
    );
    assert_eq!(documents::get(&mut conn, SHOP, avoir.id).unwrap(), avoir);
}

#[test]
fn an_avoir_reverses_the_unpaid_part_of_the_facture_first() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = a_customer(&mut conn);
    // 3 000,00 on credit, 1 000,00 paid: 2 000,00 is still owed on the paper.
    let facture = a_facture(&mut conn, c, vec![line(p, 3_000)], PaymentMode::Credit, 10);
    assert_eq!(facture.totals.net_to_pay, Money::centimes(300_000));
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

    // One line of 500,00 back.
    let avoir = avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        Some(vec![AvoirLine {
            document_line_id: facture.lines[0].id,
            qty_milli: 500,
        }]),
        None,
        Some(at(12)),
    )
    .unwrap();
    assert_eq!(avoir.totals.net_to_pay, Money::centimes(50_000));

    // The unpaid part of the facture comes down by it, and so does what the
    // customer owes: 2 000,00 less 500,00.
    assert_eq!(
        documents::get(&mut conn, SHOP, facture.id)
            .unwrap()
            .balance
            .unwrap()
            .remaining_debt,
        Money::centimes(150_000)
    );
    assert_eq!(
        debt::balance(&mut conn, SHOP, c).unwrap(),
        Money::centimes(150_000)
    );

    // One credit movement, kind avoir, naming the avoir it came from.
    let ledger = debt::ledger(&mut conn, SHOP, c).unwrap();
    let row = ledger
        .iter()
        .find(|e| e.kind == DebtKind::Avoir)
        .expect("the avoir wrote no movement");
    assert_eq!(row.credit, Money::centimes(50_000));
    assert_eq!(row.debit, Money::ZERO);
    assert_eq!(row.document_id, Some(avoir.id));

    // And it was placed on the facture it credits, so the ledger says which
    // paper the money went back to.
    let placed = debt::allocations(&mut conn, SHOP, facture.id).unwrap();
    assert_eq!(
        placed.iter().map(|a| a.amount).collect::<Vec<Money>>(),
        [Money::centimes(100_000), Money::centimes(50_000)],
        "the payment then the avoir"
    );
}

#[test]
fn an_avoir_past_the_unpaid_part_leaves_the_customer_holding_credit() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = a_customer(&mut conn);
    // 1 000,00 on credit, paid in full: the facture owes nothing.
    let facture = a_facture(&mut conn, c, vec![line(p, 1_000)], PaymentMode::Credit, 10);
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
    assert_eq!(debt::balance(&mut conn, SHOP, c).unwrap(), Money::ZERO);

    let avoir = avoir::issue(&mut conn, SHOP, OWNER, facture.id, None, None, Some(at(12))).unwrap();

    // Nothing was owed, so the whole of it is credit the shop is holding:
    // the only way a customer's balance goes below zero (features.md §2).
    assert_eq!(avoir.totals.net_to_pay, Money::centimes(100_000));
    assert_eq!(
        debt::balance(&mut conn, SHOP, c).unwrap(),
        Money::centimes(-100_000)
    );
    // The facture was already settled, so the avoir placed nothing on it: a
    // document cannot be settled for more than it asked for.
    assert_eq!(
        debt::allocations(&mut conn, SHOP, facture.id)
            .unwrap()
            .iter()
            .map(|a| a.amount)
            .collect::<Vec<Money>>(),
        [Money::centimes(100_000)]
    );
    // The avoir's own triple reads as what it did to the account: what was
    // owed before it, its whole effect on that, and what is owed after. The
    // middle figure is the negative of the avoir's net and stays that way,
    // because the paper is a statement about the day it was issued and a
    // later payment on some other facture does not change what this credit
    // note was worth.
    let balance = avoir.balance.expect("an avoir names a customer");
    assert_eq!(balance.old_balance, Money::ZERO);
    assert_eq!(balance.remaining_debt, Money::centimes(-100_000));
    assert_eq!(balance.total_debt, Money::centimes(-100_000));
    assert_eq!(
        balance
            .old_balance
            .checked_add(balance.remaining_debt)
            .unwrap(),
        balance.total_debt
    );
}

#[test]
fn a_line_cannot_be_credited_past_what_earlier_avoirs_left_on_it() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = a_customer(&mut conn);
    let facture = a_facture(&mut conn, c, vec![line(p, 3_000)], PaymentMode::Credit, 10);
    let line_id = facture.lines[0].id;

    avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        Some(vec![AvoirLine {
            document_line_id: line_id,
            qty_milli: 2_000,
        }]),
        None,
        Some(at(11)),
    )
    .unwrap();

    // One unit is left on the line, so two is a unit that was never sold.
    let err = avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        Some(vec![AvoirLine {
            document_line_id: line_id,
            qty_milli: 2_000,
        }]),
        None,
        Some(at(12)),
    )
    .unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "qty_milli"),
        "{err:?}"
    );

    // And the refusal cost nothing: no second avoir, no number burned, no
    // stock back.
    assert_eq!(
        documents::list(&mut conn, SHOP, Some(DocumentKind::Avoir))
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        99_000
    );

    // A whole avoir now credits what is left rather than the line again.
    let rest = avoir::issue(&mut conn, SHOP, OWNER, facture.id, None, None, Some(at(13))).unwrap();
    assert_eq!(rest.lines[0].qty_milli, 1_000);
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        100_000
    );
}

#[test]
fn the_running_total_of_avoirs_never_passes_what_the_facture_asked_for() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = a_customer(&mut conn);
    let facture = a_facture(&mut conn, c, vec![line(p, 2_000)], PaymentMode::Credit, 10);
    assert_eq!(facture.totals.net_to_pay, Money::centimes(200_000));

    // An avoir written straight into the file for the whole facture, with no
    // line of its own: the quantity check cannot see it, so what refuses the
    // next one has to be the running total the service keeps.
    diesel::sql_query(format!(
        "INSERT INTO documents (shop_id, kind, series, number, issued_at, user_id, regime, \
         payment_mode, seller_name, customer_id, ref_document_id, total_ht_centimes, \
         discount_centimes, subtotal_ht_centimes, tva_centimes, total_ttc_centimes, \
         stamp_centimes, net_to_pay_centimes, status) \
         VALUES (1, 'avoir', 'doc_avoir', 99, '2026-09-11 10:00:00', 1, 'reel', 'credit', \
         'Mon magasin', {c}, {}, 200000, 0, 200000, 0, 200000, 0, 200000, 'issued')",
        facture.id
    ))
    .execute(&mut conn)
    .unwrap();

    let err =
        avoir::issue(&mut conn, SHOP, OWNER, facture.id, None, None, Some(at(12))).unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "lines"),
        "{err:?}"
    );
    assert_eq!(
        documents::list(&mut conn, SHOP, Some(DocumentKind::Avoir))
            .unwrap()
            .len(),
        1,
        "the refused avoir was written anyway"
    );
}

#[test]
fn only_a_facture_that_still_stands_is_credited() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = a_customer(&mut conn);

    // A ticket is not a facture, so there is no such facture to credit.
    let ticket = sales::issue(
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
            kind: SaleKind::Ticket,
            issued_at: Some(at(10)),
        },
    )
    .unwrap()
    .document;
    let err =
        avoir::issue(&mut conn, SHOP, OWNER, ticket.id, None, None, Some(at(11))).unwrap_err();
    assert!(
        matches!(err, CoreError::NotFound { entity, .. } if entity == "facture"),
        "{err:?}"
    );

    // An avoir is not written against an avoir either.
    let facture = a_facture(&mut conn, c, vec![line(p, 1_000)], PaymentMode::Credit, 11);
    let avoir = avoir::issue(&mut conn, SHOP, OWNER, facture.id, None, None, Some(at(12))).unwrap();
    let err = avoir::issue(&mut conn, SHOP, OWNER, avoir.id, None, None, Some(at(13))).unwrap_err();
    assert!(
        matches!(err, CoreError::NotFound { entity, .. } if entity == "facture"),
        "{err:?}"
    );

    // Nor against another shop's facture (rule 3). The row is not visible to
    // the shop asking at all, so the answer names the document rather than the
    // kind: saying "no such facture" would confirm that the id is a facture
    // somewhere.
    let err = avoir::issue(&mut conn, 2, OWNER, facture.id, None, None, Some(at(13))).unwrap_err();
    assert!(
        matches!(err, CoreError::NotFound { entity, .. } if entity == "document"),
        "{err:?}"
    );
}

#[test]
fn an_avoir_that_credits_nothing_is_refused() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = a_customer(&mut conn);
    let facture = a_facture(&mut conn, c, vec![line(p, 1_000)], PaymentMode::Credit, 10);
    avoir::issue(&mut conn, SHOP, OWNER, facture.id, None, None, Some(at(11))).unwrap();

    // Everything has come back already, so there is nothing left to write a
    // credit note for and no number is burned finding that out.
    let err =
        avoir::issue(&mut conn, SHOP, OWNER, facture.id, None, None, Some(at(12))).unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "lines"),
        "{err:?}"
    );
    let zero = avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        Some(vec![AvoirLine {
            document_line_id: facture.lines[0].id,
            qty_milli: 0,
        }]),
        None,
        Some(at(12)),
    )
    .unwrap_err();
    assert!(
        matches!(zero, CoreError::Validation { ref field, .. } if field == "qty_milli"),
        "{zero:?}"
    );
}

#[test]
fn a_line_of_another_facture_cannot_be_credited_on_this_one() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = a_customer(&mut conn);
    let one = a_facture(&mut conn, c, vec![line(p, 1_000)], PaymentMode::Credit, 10);
    let two = a_facture(&mut conn, c, vec![line(p, 1_000)], PaymentMode::Credit, 11);

    let err = avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        one.id,
        Some(vec![AvoirLine {
            document_line_id: two.lines[0].id,
            qty_milli: 500,
        }]),
        None,
        Some(at(12)),
    )
    .unwrap_err();
    assert!(
        matches!(err, CoreError::NotFound { entity, .. } if entity == "document_line"),
        "{err:?}"
    );
}

#[test]
fn the_avoir_is_audited_with_what_it_credited_and_what_it_left() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = a_customer(&mut conn);
    let facture = a_facture(&mut conn, c, vec![line(p, 1_000)], PaymentMode::Credit, 10);

    let avoir = avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        None,
        Some("retour".to_string()),
        Some(at(11)),
    )
    .unwrap();

    let entry = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .find(|e| e.action == "document.avoir")
        .expect("the avoir wrote no audit entry");
    assert_eq!(entry.entity, "document");
    assert_eq!(entry.entity_id, Some(facture.id));
    assert_eq!(entry.user_id, OWNER);
    let before: serde_json::Value = serde_json::from_str(&entry.before.unwrap()).unwrap();
    let after: serde_json::Value = serde_json::from_str(&entry.after.unwrap()).unwrap();
    assert_eq!(before["remaining_debt_centimes"], 100_000);
    assert_eq!(before["balance_centimes"], 100_000);
    assert_eq!(after["avoir_document_id"], avoir.id);
    assert_eq!(after["amount_centimes"], 100_000);
    assert_eq!(after["remaining_debt_centimes"], 0);
    assert_eq!(after["balance_centimes"], 0);
    assert_eq!(after["reason"], "retour");
}

#[test]
fn a_partial_avoir_prorates_the_discounts_of_the_line_it_credits() {
    // `avoir_partial_prorates_discounts_floor`: a line discount belongs to
    // the whole line, so crediting part of the line credits that part of the
    // discount, rounded down. Rounded down and not up because the discount is
    // what the customer did not pay: rounding it up would credit them for a
    // centime they were never charged.
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = a_customer(&mut conn);
    let facture = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![NewSaleLine {
                product_id: p,
                qty_milli: 3_000,
                unit_price: None,
                // 10,01 off three units: a third of it is 333,66 centimes.
                line_discount: Money::centimes(1_001),
            }],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Credit,
            tendered: None,
            customer_id: Some(c),
            override_credit: false,
            kind: SaleKind::Facture,
            issued_at: Some(at(10)),
        },
    )
    .unwrap()
    .document;

    let avoir = avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        Some(vec![AvoirLine {
            document_line_id: facture.lines[0].id,
            qty_milli: 1_000,
        }]),
        None,
        Some(at(11)),
    )
    .unwrap();
    assert_eq!(avoir.lines[0].line_discount, Money::centimes(333));
    assert_eq!(
        avoir.lines[0].line_total,
        Money::centimes(100_000 - 333),
        "one unit less its third of the discount"
    );
}

#[test]
fn a_cancelled_facture_is_credited_by_nothing_more() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = a_customer(&mut conn);
    let facture = a_facture(&mut conn, c, vec![line(p, 2_000)], PaymentMode::Credit, 10);
    diesel::sql_query(format!(
        "UPDATE documents SET status = 'cancelled', cancelled_at = '2026-09-11 09:00:00', \
         cancelled_by = {OWNER}, cancel_reason = 'erreur' WHERE id = {}",
        facture.id
    ))
    .execute(&mut conn)
    .unwrap();
    assert_eq!(
        documents::get(&mut conn, SHOP, facture.id).unwrap().status,
        DocumentStatus::Cancelled
    );

    let err =
        avoir::issue(&mut conn, SHOP, OWNER, facture.id, None, None, Some(at(12))).unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "document_id"),
        "{err:?}"
    );
}

/// The sum of every field of every avoir on a facture, for the invariant that
/// the papers add up: what came back on credit notes equals what the facture
/// asked for, less the droit de timbre it never refunds.
fn avoir_sum(conn: &mut SqliteConnection, facture_id: i32) -> (i64, i64, i64, i64, i64) {
    let mut ht = 0;
    let mut discount = 0;
    let mut subtotal = 0;
    let mut tva = 0;
    let mut ttc = 0;
    for a in avoir::list_for(conn, SHOP, facture_id).unwrap() {
        ht += a.totals.total_ht.as_centimes();
        discount += a.totals.discount.as_centimes();
        subtotal += a.totals.subtotal_ht.as_centimes();
        tva += a.totals.tva.as_centimes();
        ttc += a.totals.total_ttc.as_centimes();
        assert_eq!(a.totals.stamp, Money::ZERO, "an avoir carries no stamp");
        assert_eq!(a.totals.net_to_pay, a.totals.total_ttc);
    }
    (ht, discount, subtotal, tva, ttc)
}

/// 0,50 at 19 % three times over. Each single unit rounds its 9,5 centimes of
/// tax up to 10, so three slices come to 180 against a facture of 179: one
/// centime that was never charged, and under the old arithmetic the third
/// avoir was refused for it and the cancellation with it.
#[test]
fn three_one_unit_avoirs_add_up_to_the_facture_they_credit() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Bougie", 50, 1900);
    let c = a_customer(&mut conn);
    let facture = a_facture(&mut conn, c, vec![line(p, 3_000)], PaymentMode::Credit, 10);
    assert_eq!(facture.totals.net_to_pay, Money::centimes(179));
    let line_id = facture.lines[0].id;

    for day in 11..=13 {
        avoir::issue(
            &mut conn,
            SHOP,
            OWNER,
            facture.id,
            Some(vec![AvoirLine {
                document_line_id: line_id,
                qty_milli: 1_000,
            }]),
            None,
            Some(at(day)),
        )
        .unwrap_or_else(|e| panic!("the avoir of day {day} was refused: {e:?}"));
    }

    let (ht, discount, subtotal, tva, ttc) = avoir_sum(&mut conn, facture.id);
    assert_eq!(ht, facture.totals.total_ht.as_centimes());
    assert_eq!(discount, facture.totals.discount.as_centimes());
    assert_eq!(subtotal, facture.totals.subtotal_ht.as_centimes());
    assert_eq!(tva, facture.totals.tva.as_centimes());
    assert_eq!(ttc, facture.totals.total_ttc.as_centimes());

    // The closing avoir is the one that gave the centime back: 179 - 60 - 60.
    let avoirs = avoir::list_for(&mut conn, SHOP, facture.id).unwrap();
    assert_eq!(avoirs[0].totals.net_to_pay, Money::centimes(60));
    assert_eq!(avoirs[1].totals.net_to_pay, Money::centimes(60));
    assert_eq!(avoirs[2].totals.net_to_pay, Money::centimes(59));
}

/// The same rounding the other way. 0,80 at 19 % is 15,2 centimes of tax a
/// unit, rounded down to 15, so three slices come to 285 against a facture of
/// 286: the facture would have been annulled still asking for a centime.
#[test]
fn a_facture_annulled_after_partial_avoirs_is_left_asking_for_nothing() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Savon", 80, 1900);
    let c = a_customer(&mut conn);
    let facture = a_facture(&mut conn, c, vec![line(p, 3_000)], PaymentMode::Credit, 10);
    assert_eq!(facture.totals.net_to_pay, Money::centimes(286));
    let line_id = facture.lines[0].id;

    for day in 11..=12 {
        avoir::issue(
            &mut conn,
            SHOP,
            OWNER,
            facture.id,
            Some(vec![AvoirLine {
                document_line_id: line_id,
                qty_milli: 1_000,
            }]),
            None,
            Some(at(day)),
        )
        .unwrap();
    }

    // The cancellation writes the closing avoir for what is left of the unit
    // and of the centime, and the facture asks for nothing afterwards.
    documents::cancel(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "commande annulée".to_string(),
        Some(at(13)),
    )
    .unwrap();

    let (_, _, _, _, ttc) = avoir_sum(&mut conn, facture.id);
    assert_eq!(ttc, facture.totals.total_ttc.as_centimes());
    let after = documents::get(&mut conn, SHOP, facture.id).unwrap();
    assert_eq!(
        after.balance.map(|b| b.remaining_debt),
        Some(Money::ZERO),
        "an annulled facture asks for nothing at all"
    );
    assert_eq!(debt::balance(&mut conn, SHOP, c).unwrap(), Money::ZERO);
}

/// Two partials that between them take every quantity: the second is the
/// closing one and carries the remainder of every field, so the two reproduce
/// the facture even across two rates and a global discount.
#[test]
fn two_partials_that_finish_a_facture_reproduce_every_field_of_it() {
    let (_dir, mut conn) = open_temp();
    let a = product(&mut conn, "Ciment", 333, 1900);
    let b = product(&mut conn, "Semoule", 777, 900);
    let c = a_customer(&mut conn);
    let facture = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![line(a, 3_000), line(b, 7_000)],
            global_discount: Money::centimes(101),
            payment_mode: PaymentMode::Credit,
            tendered: None,
            customer_id: Some(c),
            override_credit: false,
            kind: SaleKind::Facture,
            issued_at: Some(at(10)),
        },
    )
    .unwrap()
    .document;

    let first: Vec<AvoirLine> = facture
        .lines
        .iter()
        .map(|l| AvoirLine {
            document_line_id: l.id,
            qty_milli: 1_000,
        })
        .collect();
    avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        Some(first),
        None,
        Some(at(11)),
    )
    .unwrap();
    avoir::issue(&mut conn, SHOP, OWNER, facture.id, None, None, Some(at(12))).unwrap();

    let (ht, discount, subtotal, tva, ttc) = avoir_sum(&mut conn, facture.id);
    assert_eq!(ht, facture.totals.total_ht.as_centimes(), "HT");
    assert_eq!(discount, facture.totals.discount.as_centimes(), "discount");
    assert_eq!(
        subtotal,
        facture.totals.subtotal_ht.as_centimes(),
        "subtotal"
    );
    assert_eq!(tva, facture.totals.tva.as_centimes(), "TVA");
    assert_eq!(ttc, facture.totals.total_ttc.as_centimes(), "TTC");

    // And per rate, which is the figure a comptable adds up across the year.
    for row in &facture.totals.tva_by_rate {
        let mut base = 0;
        let mut amount = 0;
        for av in avoir::list_for(&mut conn, SHOP, facture.id).unwrap() {
            for r in &av.totals.tva_by_rate {
                if r.rate == row.rate {
                    base += r.base.as_centimes();
                    amount += r.amount.as_centimes();
                }
            }
        }
        assert_eq!(base, row.base.as_centimes(), "base at {:?}", row.rate);
        assert_eq!(amount, row.amount.as_centimes(), "tva at {:?}", row.rate);
    }
}
