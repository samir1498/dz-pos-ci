// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The cash position (features.md §1, Dashboard): the one place the figure is
//! computed, and it is computed from the ledgers every time.
//!
//! What is asserted here is which rows reach it and which do not. The
//! property test beside this file (`cash_prop.rs`) asserts that the parts add
//! up to the whole over sequences nobody wrote by hand.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::money::{Bps, Money, PaymentMode, Regime, Totals, TvaLine};
use dzpos_core::services::cancellation;
use dzpos_core::services::cash;
use dzpos_core::services::cash_refunds::Refund;
use dzpos_core::services::clock::{Month, Period};
use dzpos_core::services::debt::{self, DebtKind, NewDebtEntry, PaymentMethod};
use dzpos_core::services::documents::{
    self, DocumentKind, NewDocument, NewDocumentLine, SellerBlock,
};
use dzpos_core::services::expenses::{self, NewExpense};

mod common;
use common::{a_customer, open_temp};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

fn day(n: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, n).unwrap()
}

fn at(n: u32, hour: u32) -> NaiveDateTime {
    day(n).and_hms_opt(hour, 30, 0).unwrap()
}

/// A document written straight through `services::documents`, with the totals
/// stated rather than computed: what is under test is which rows the cash
/// position reads, not how a basket adds up.
#[allow(clippy::too_many_arguments)]
fn a_document(
    conn: &mut SqliteConnection,
    shop_id: i32,
    kind: DocumentKind,
    mode: PaymentMode,
    total_ttc: i64,
    stamp: i64,
    ref_document_id: Option<i32>,
    issued_at: NaiveDateTime,
) -> i32 {
    let ttc = Money::centimes(total_ttc);
    let stamp = Money::centimes(stamp);
    documents::issue(
        conn,
        shop_id,
        NewDocument {
            kind,
            issued_at,
            user_id: OWNER,
            regime: Regime::Ifu,
            payment_mode: mode,
            seller: SellerBlock {
                name: "Mon magasin".to_string(),
                rc: None,
                nif: None,
                nis: None,
                ai: None,
                address: None,
                phone: None,
            },
            customer: None,
            buyer: None,
            ref_document_id,
            balance: None,
            totals: Totals {
                total_ht: ttc,
                discount: Money::ZERO,
                subtotal_ht: ttc,
                tva_by_rate: vec![TvaLine {
                    rate: Bps::ZERO,
                    base: ttc,
                    amount: Money::ZERO,
                }],
                tva: Money::ZERO,
                total_ttc: ttc,
                stamp,
                net_to_pay: ttc.checked_add(stamp).unwrap(),
            },
            tendered: None,
            change: None,
            lines: vec![NewDocumentLine {
                product_id: None,
                name: "Article".to_string(),
                barcode: None,
                qty_milli: 1_000,
                unit_price: ttc,
                line_discount: Money::ZERO,
                rate_bps: Bps::ZERO,
                line_total: ttc,
                ref_line_id: None,
            }],
        },
    )
    .unwrap()
    .id
}

/// A supplier and one payment to them. Written by hand: the supplier payment
/// service owns that, and what this file needs is the row on
/// `supplier_ledger` the cash position reads.
fn a_supplier(conn: &mut SqliteConnection, shop_id: i32, id: i32) {
    diesel::sql_query(format!(
        "INSERT INTO suppliers (id, shop_id, name) VALUES ({id}, {shop_id}, 'Fournisseur {id}')"
    ))
    .execute(conn)
    .unwrap();
}

fn pay_supplier(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
    centimes: i64,
    mode: &str,
    when: NaiveDateTime,
) {
    let stamped = when.format("%Y-%m-%d %H:%M:%S");
    diesel::sql_query(format!(
        "INSERT INTO supplier_ledger \
         (shop_id, supplier_id, kind, debit_centimes, credit_centimes, user_id, created_at, payment_mode) \
         VALUES ({shop_id}, {supplier_id}, 'payment', 0, {centimes}, {OWNER}, '{stamped}', '{mode}')"
    ))
    .execute(conn)
    .unwrap();
}

fn spend(conn: &mut SqliteConnection, centimes: i64, on: NaiveDate) {
    spend_in(conn, SHOP, centimes, on);
}

/// The same, on whichever shop's books. Only the second-shop cases name a
/// shop; everything else spends on this shop and reads better for it.
fn spend_in(conn: &mut SqliteConnection, shop_id: i32, centimes: i64, on: NaiveDate) {
    let category = expenses::categories(conn, shop_id).unwrap()[0].id;
    expenses::create(
        conn,
        shop_id,
        OWNER,
        NewExpense {
            category_id: category,
            amount: Money::centimes(centimes),
            expense_date: on,
            note: None,
        },
    )
    .unwrap();
}

#[test]
fn a_day_with_nothing_on_it_answers_zero_everywhere_rather_than_nothing() {
    let (_dir, mut conn) = open_temp();
    let empty = cash::position(&mut conn, SHOP, Period::Day(day(10))).unwrap();
    assert_eq!(empty.cash_in.sales, Money::ZERO);
    assert_eq!(empty.cash_in.stamp, Money::ZERO);
    assert_eq!(empty.cash_in.customer_payments, Money::ZERO);
    // Zero and not nothing: `SUM` over no rows answers NULL, and every one of
    // these six reads is coalesced. `cash_refunds` is the newest of them and
    // the easiest to add without one.
    assert_eq!(empty.cash_out.refunds, Money::ZERO);
    assert_eq!(empty.cash_out.supplier_payments, Money::ZERO);
    assert_eq!(empty.cash_out.expenses, Money::ZERO);
    assert_eq!(empty.cash, Money::ZERO);
    assert_eq!(empty.card_in.sales, Money::ZERO);
    assert_eq!(empty.card_in.stamp, Money::ZERO);
    assert_eq!(empty.card_in.customer_payments, Money::ZERO);
    assert_eq!(empty.from, day(10));
    assert_eq!(empty.to, day(10));
}

#[test]
fn the_day_counts_the_cash_that_moved_on_it_and_nothing_else() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    a_supplier(&mut conn, SHOP, 1);

    // Money in. The cash facture carries a droit de timbre of 20,00, and the
    // customer handed that over with the rest: the sales column reads
    // `net_to_pay`, and the same 20,00 shows again on its own so a screen can
    // take the tax back out.
    let ticket = a_document(
        &mut conn,
        SHOP,
        DocumentKind::Ticket,
        PaymentMode::Cash,
        100_000,
        0,
        None,
        at(10, 9),
    );
    a_document(
        &mut conn,
        SHOP,
        DocumentKind::Facture,
        PaymentMode::Cash,
        200_000,
        2_000,
        None,
        at(10, 10),
    );
    a_document(
        &mut conn,
        SHOP,
        DocumentKind::Ticket,
        PaymentMode::Card,
        50_000,
        0,
        None,
        at(10, 11),
    );
    // Not money in the drawer: a credit sale, a proforma, an avoir, and a
    // ticket that was cancelled after it was rung up.
    a_document(
        &mut conn,
        SHOP,
        DocumentKind::Facture,
        PaymentMode::Credit,
        900_000,
        0,
        None,
        at(10, 12),
    );
    a_document(
        &mut conn,
        SHOP,
        DocumentKind::Proforma,
        PaymentMode::Cash,
        900_000,
        0,
        None,
        at(10, 12),
    );
    a_document(
        &mut conn,
        SHOP,
        DocumentKind::Avoir,
        PaymentMode::Cash,
        900_000,
        0,
        Some(ticket),
        at(10, 13),
    );
    let cancelled = a_document(
        &mut conn,
        SHOP,
        DocumentKind::Ticket,
        PaymentMode::Cash,
        900_000,
        0,
        None,
        at(10, 14),
    );
    cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        cancelled,
        "erreur de caisse".to_string(),
        Some(at(10, 15)),
    )
    .unwrap();

    // The credit facture leaves a debt (written here, because `issue` does
    // not touch the ledger; the till's own service does). Some of it comes
    // back in cash and some on the card.
    debt::append_at(
        &mut conn,
        SHOP,
        NewDebtEntry {
            customer_id: customer,
            document_id: None,
            kind: DebtKind::Sale,
            debit: Money::centimes(900_000),
            credit: Money::ZERO,
            user_id: OWNER,
            note: None,
        },
        Some(at(10, 12)),
    )
    .unwrap();
    debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(30_000),
        PaymentMethod::Cash,
        None,
        at(10, 16),
    )
    .unwrap();
    debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(7_000),
        PaymentMethod::Card,
        None,
        at(10, 16),
    )
    .unwrap();

    // Money out. A cash ticket of 150,00 rung the same day and handed back
    // over the counter: the drawer took it and the drawer gave it back, so it
    // is in the takings above and out again here.
    let refunded = a_document(
        &mut conn,
        SHOP,
        DocumentKind::Ticket,
        PaymentMode::Cash,
        15_000,
        0,
        None,
        at(10, 18),
    );
    cancellation::cancel_settling(
        &mut conn,
        SHOP,
        OWNER,
        refunded,
        "rendu".to_string(),
        Some(at(10, 19)),
        Refund::Cash,
    )
    .unwrap();
    pay_supplier(&mut conn, SHOP, 1, 40_000, "cash", at(10, 17));
    pay_supplier(&mut conn, SHOP, 1, 11_000, "card", at(10, 17));
    spend(&mut conn, 25_000, day(10));
    spend(&mut conn, 5_000, day(10));

    // The day before and the day after, and another shop's whole day.
    a_document(
        &mut conn,
        SHOP,
        DocumentKind::Ticket,
        PaymentMode::Cash,
        777_000,
        0,
        None,
        at(9, 23),
    );
    a_document(
        &mut conn,
        SHOP,
        DocumentKind::Ticket,
        PaymentMode::Cash,
        777_000,
        0,
        None,
        at(11, 0),
    );
    // One on each side. Without the day-before spend, pulling the expense
    // window back a whole day left every test in this file green.
    spend(&mut conn, 777_000, day(9));
    spend(&mut conn, 777_000, day(11));
    pay_supplier(&mut conn, SHOP, 1, 777_000, "cash", at(11, 8));

    let position = cash::position(&mut conn, SHOP, Period::Day(day(10))).unwrap();
    // 1 000,00 of ticket, 2 020,00 of facture (its 2 000,00 plus the 20,00
    // stamp that came over the counter with it) and the 150,00 ticket that
    // was handed back. The refunded one stays in the takings: the drawer did
    // take that money on this day, and what it gave back is `refunds` below.
    // Drop it from the sales column and the day is short of it twice.
    assert_eq!(position.cash_in.sales, Money::centimes(317_000));
    assert_eq!(position.cash_in.stamp, Money::centimes(2_000));
    assert_eq!(position.cash_in.customer_payments, Money::centimes(30_000));
    // The notes that went back over the counter, on the day they went. The
    // avoir written against the standing facture above is not here: that one
    // was settled on the customer's ledger and no drawer opened for it.
    assert_eq!(position.cash_out.refunds, Money::centimes(15_000));
    assert_eq!(position.cash_out.supplier_payments, Money::centimes(40_000));
    assert_eq!(position.cash_out.expenses, Money::centimes(30_000));
    // 317 000 + 30 000 taken, 15 000 + 40 000 + 30 000 paid out.
    assert_eq!(position.cash, Money::centimes(262_000));
    assert_eq!(position.card_in.sales, Money::centimes(50_000));
    // The app writes no stamp on a card document: the droit de timbre is due
    // on a cash payment and on nothing else.
    assert_eq!(position.card_in.stamp, Money::ZERO);
    assert_eq!(position.card_in.customer_payments, Money::centimes(7_000));
}

#[test]
fn the_month_covers_its_first_and_its_last_day_and_no_other_shops() {
    let (_dir, mut conn) = open_temp();
    a_supplier(&mut conn, SHOP, 1);
    a_document(
        &mut conn,
        SHOP,
        DocumentKind::Ticket,
        PaymentMode::Cash,
        10_000,
        0,
        None,
        at(1, 0),
    );
    a_document(
        &mut conn,
        SHOP,
        DocumentKind::Ticket,
        PaymentMode::Cash,
        20_000,
        0,
        None,
        at(30, 23),
    );
    spend(&mut conn, 1_000, day(1));
    spend(&mut conn, 2_000, day(30));
    // The last day of August and the first of October, so the month's own
    // edges are proved on the expense column and not only on the sales one.
    spend(
        &mut conn,
        999_000,
        NaiveDate::from_ymd_opt(2026, 8, 31).unwrap(),
    );
    spend(
        &mut conn,
        999_000,
        NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
    );
    // August and October, on either side of the month asked for.
    a_document(
        &mut conn,
        SHOP,
        DocumentKind::Ticket,
        PaymentMode::Cash,
        999_000,
        0,
        None,
        NaiveDate::from_ymd_opt(2026, 8, 31)
            .and_then(|d| d.and_hms_opt(23, 59, 59))
            .unwrap(),
    );
    a_document(
        &mut conn,
        SHOP,
        DocumentKind::Ticket,
        PaymentMode::Cash,
        999_000,
        0,
        None,
        NaiveDate::from_ymd_opt(2026, 10, 1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .unwrap(),
    );

    // A second shop on the same file, spending and selling on the same days.
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(&mut conn)
        .unwrap();
    a_supplier(&mut conn, 2, 2);
    a_document(
        &mut conn,
        2,
        DocumentKind::Ticket,
        PaymentMode::Cash,
        555_000,
        0,
        None,
        at(15, 12),
    );
    pay_supplier(&mut conn, 2, 2, 555_000, "cash", at(15, 12));
    // Inside the month asked for, on the other shop's books. The expense
    // total is read with a shop id like every other query, and until this
    // line no test made the other shop spend anything at all. The first
    // migration seeds categories for the shop it opens, so the second one
    // gets its own before it can spend.
    diesel::sql_query(
        "INSERT INTO expense_categories (shop_id, key, sort_order, active) \
         VALUES (2, 'rent', 1, 1)",
    )
    .execute(&mut conn)
    .unwrap();
    spend_in(&mut conn, 2, 555_000, day(15));

    let month =
        cash::position(&mut conn, SHOP, Period::Month(Month::new(2026, 9).unwrap())).unwrap();
    assert_eq!(month.from, day(1));
    assert_eq!(month.to, day(30));
    assert_eq!(month.cash_in.sales, Money::centimes(30_000));
    assert_eq!(month.cash_out.expenses, Money::centimes(3_000));
    assert_eq!(month.cash_out.supplier_payments, Money::ZERO);
    assert_eq!(month.cash, Money::centimes(27_000));
}

#[test]
fn the_last_second_of_a_day_and_the_first_of_the_next_are_two_days() {
    // The bound is half open: every moment of the day counts and the first
    // moment of the day after does not. A ticket rung up at 23:59:59 and one
    // rung up a second later are the two rows either side of it, and a `<=`
    // where the code has a `<` would put both on both days.
    let (_dir, mut conn) = open_temp();
    let last = day(10).and_hms_opt(23, 59, 59).unwrap();
    let first = day(11).and_hms_opt(0, 0, 0).unwrap();
    a_document(
        &mut conn,
        SHOP,
        DocumentKind::Ticket,
        PaymentMode::Cash,
        100_000,
        0,
        None,
        last,
    );
    a_document(
        &mut conn,
        SHOP,
        DocumentKind::Ticket,
        PaymentMode::Cash,
        200_000,
        0,
        None,
        first,
    );

    let tenth = cash::position(&mut conn, SHOP, Period::Day(day(10))).unwrap();
    assert_eq!(tenth.cash_in.sales, Money::centimes(100_000));
    let eleventh = cash::position(&mut conn, SHOP, Period::Day(day(11))).unwrap();
    assert_eq!(eleventh.cash_in.sales, Money::centimes(200_000));
    // And neither day carries the other's row: the two figures are the two
    // documents, once each.
    assert_eq!(
        tenth
            .cash_in
            .sales
            .checked_add(eleventh.cash_in.sales)
            .unwrap(),
        Money::centimes(300_000)
    );
}
