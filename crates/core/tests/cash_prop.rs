// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The cash position adds up, over days nobody wrote by hand.
//!
//! The property is not "the total is the sum of the parts", which the code
//! computing it that way would satisfy by construction. It is that each part
//! is the sum of exactly the rows that belong in it: the case builds a random
//! day out of every kind of row the shop can write, keeps its own tally as it
//! writes them, and compares that tally column for column with what the
//! service reads back out of the file. A filter that let a cancelled ticket,
//! a credit sale, another shop's row or the day before through would show up
//! here as a column that disagrees.
//!
//! Against a real temp SQLite file, one per case, because the filters under
//! test are SQL.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::money::{Bps, Money, PaymentMode, Regime, Totals, TvaLine};
use dzpos_core::services::cash;
use dzpos_core::services::clock::Period;
use dzpos_core::services::debt::{self, DebtKind, NewDebtEntry, PaymentMethod};
use dzpos_core::services::documents::{
    self, DocumentKind, NewDocument, NewDocumentLine, SellerBlock,
};
use dzpos_core::services::expenses::{self, NewExpense};
use proptest::prelude::*;

mod common;

const SHOP: i32 = 1;
const OWNER: i32 = 1;
/// The day the position is asked for. Rows land on it, on the day before and
/// on the day after, so the bounds are under test on every case.
const THE_DAY: u32 = 15;

/// One thing the shop does. The amounts are whole dinars, small enough that a
/// long day still fits an i64 with room to spare.
#[derive(Debug, Clone, Copy)]
enum Event {
    /// A sale: its kind, how it was paid, what it came to and the stamp
    /// beside it.
    Sell(DocumentKind, PaymentMode, i64, i64),
    /// A sale rung up and then annulled.
    SellThenCancel(PaymentMode, i64),
    /// A credit facture, then money against the debt.
    Settle(PaymentMethod, i64),
    /// Money to a supplier.
    PaySupplier(PaymentMethod, i64),
    /// Money out that is not stock.
    Spend(i64),
    /// The same day, in another shop's half of the file.
    Elsewhere(i64),
}

/// Where the event lands: the day asked for, the day before, or the day
/// after. Only the first counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum When {
    Before,
    OnTheDay,
    After,
}

impl When {
    const fn day(self) -> u32 {
        match self {
            When::Before => THE_DAY - 1,
            When::OnTheDay => THE_DAY,
            When::After => THE_DAY + 1,
        }
    }
}

/// The tally the case keeps as it writes, which is what the service has to
/// agree with.
#[derive(Debug, Default, Clone, Copy)]
struct Expected {
    cash_sales: i64,
    cash_stamp: i64,
    cash_customer_payments: i64,
    supplier_cash: i64,
    expenses: i64,
    card_sales: i64,
    card_customer_payments: i64,
}

fn events() -> impl Strategy<Value = Vec<(When, Event)>> {
    let amount = (1i64..=50).prop_map(|d| d * 10_000);
    let mode = prop_oneof![
        Just(PaymentMode::Cash),
        Just(PaymentMode::Card),
        Just(PaymentMode::Credit),
    ];
    let method = prop_oneof![Just(PaymentMethod::Cash), Just(PaymentMethod::Card)];
    let kind = prop_oneof![
        Just(DocumentKind::Ticket),
        Just(DocumentKind::Facture),
        Just(DocumentKind::Proforma),
    ];
    // A stamp only on a cash document: the droit de timbre is due on a cash
    // payment (features.md §3), so a stamped card or credit sale is a row the
    // app never writes and a generator that drew one would be proving the
    // query against a file that cannot exist.
    let event = prop_oneof![
        (kind, mode.clone(), amount.clone(), 0i64..=500).prop_map(|(k, m, a, s)| Event::Sell(
            k,
            m,
            a,
            if matches!(m, PaymentMode::Cash) { s } else { 0 }
        )),
        (mode, amount.clone()).prop_map(|(m, a)| Event::SellThenCancel(m, a)),
        (method.clone(), amount.clone()).prop_map(|(m, a)| Event::Settle(m, a)),
        (method, amount.clone()).prop_map(|(m, a)| Event::PaySupplier(m, a)),
        amount.clone().prop_map(Event::Spend),
        amount.prop_map(Event::Elsewhere),
    ];
    let when = prop_oneof![Just(When::Before), Just(When::OnTheDay), Just(When::After)];
    prop::collection::vec((when, event), 1..=14)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn every_column_of_the_position_is_the_sum_of_the_rows_that_belong_in_it(
        events in events()
    ) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let mut conn = dzpos_core::db::open(&path).unwrap();
        let customer = common::a_customer(&mut conn, "Entreprise Benali");
        a_second_shop(&mut conn);
        a_supplier(&mut conn, SHOP, 1);
        a_supplier(&mut conn, 2, 2);
        let mut expected = Expected::default();
        // Two documents can be issued inside one second and the hour only has
        // to stay on the right day, so it walks up and wraps.
        let mut hour = 0;

        for (when, event) in &events {
            let day = when.day();
            hour = (hour + 1) % 24;
            let counts = *when == When::OnTheDay;
            match *event {
                Event::Sell(kind, mode, total_ttc, stamp) => {
                    a_document(&mut conn, SHOP, kind, mode, total_ttc, stamp, at(day, hour), None);
                    // A proforma is a quotation and no money moved for it; a
                    // credit sale is a debt, not a drawer.
                    let sold = matches!(kind, DocumentKind::Ticket | DocumentKind::Facture);
                    if counts && sold {
                        match mode {
                            // What the drawer took is `net_to_pay`: the stamp
                            // came over the counter with the rest.
                            PaymentMode::Cash => {
                                expected.cash_sales += total_ttc + stamp;
                                expected.cash_stamp += stamp;
                            }
                            PaymentMode::Card => expected.card_sales += total_ttc,
                            PaymentMode::Credit => {}
                        }
                    }
                }
                Event::SellThenCancel(mode, total_ttc) => {
                    let id = a_document(
                        &mut conn, SHOP, DocumentKind::Ticket, mode, total_ttc, 0, at(day, hour), None,
                    );
                    documents::cancel(
                        &mut conn, SHOP, OWNER, id, "erreur".to_string(), Some(at(day, hour)),
                    ).unwrap();
                    // Nothing is added: an annulled ticket is money that never
                    // stayed in the drawer.
                }
                Event::Settle(method, centimes) => {
                    // The debt first, so the payment has something to land on.
                    // Written straight onto the ledger and dated before the
                    // range: what is under test is which payments the cash
                    // position counts, not how a credit sale gets there.
                    debt::append_at(
                        &mut conn,
                        SHOP,
                        NewDebtEntry {
                            customer_id: customer,
                            document_id: None,
                            kind: DebtKind::Sale,
                            debit: Money::centimes(centimes),
                            credit: Money::ZERO,
                            user_id: OWNER,
                            note: None,
                        },
                        Some(at(THE_DAY - 2, hour)),
                    ).unwrap();
                    debt::pay(
                        &mut conn, SHOP, OWNER, customer, Money::centimes(centimes),
                        method, None, at(day, hour),
                    ).unwrap();
                    if counts {
                        match method {
                            PaymentMethod::Cash => expected.cash_customer_payments += centimes,
                            PaymentMethod::Card => expected.card_customer_payments += centimes,
                        }
                    }
                }
                Event::PaySupplier(method, centimes) => {
                    pay_supplier(&mut conn, SHOP, 1, centimes, method, at(day, hour));
                    if counts && method == PaymentMethod::Cash {
                        expected.supplier_cash += centimes;
                    }
                }
                Event::Spend(centimes) => {
                    spend(&mut conn, centimes, calendar(day));
                    if counts {
                        expected.expenses += centimes;
                    }
                }
                Event::Elsewhere(centimes) => {
                    // The other shop sells, pays a supplier and spends on the
                    // same day. None of it is this shop's cash.
                    a_document(
                        &mut conn, 2, DocumentKind::Ticket, PaymentMode::Cash, centimes, 0,
                        at(day, hour), None,
                    );
                    pay_supplier(&mut conn, 2, 2, centimes, PaymentMethod::Cash, at(day, hour));
                }
            }
        }

        let position = cash::position(&mut conn, SHOP, Period::Day(calendar(THE_DAY))).unwrap();
        prop_assert_eq!(position.cash_in.sales.as_centimes(), expected.cash_sales);
        prop_assert_eq!(position.cash_in.stamp.as_centimes(), expected.cash_stamp);
        prop_assert_eq!(
            position.cash_in.customer_payments.as_centimes(),
            expected.cash_customer_payments
        );
        prop_assert_eq!(position.cash_out.refunds, Money::ZERO);
        prop_assert_eq!(
            position.cash_out.supplier_payments.as_centimes(),
            expected.supplier_cash
        );
        prop_assert_eq!(position.cash_out.expenses.as_centimes(), expected.expenses);
        prop_assert_eq!(position.card_in.sales.as_centimes(), expected.card_sales);
        // Nothing here writes a stamped card document, so the tax never
        // reaches this side.
        prop_assert_eq!(position.card_in.stamp, Money::ZERO);
        prop_assert_eq!(
            position.card_in.customer_payments.as_centimes(),
            expected.card_customer_payments
        );
        // And the whole is its parts, to the centime.
        prop_assert_eq!(
            position.cash.as_centimes(),
            expected.cash_sales + expected.cash_customer_payments
                - expected.supplier_cash
                - expected.expenses
        );
    }
}

fn calendar(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, day).unwrap()
}

fn at(day: u32, hour: u32) -> NaiveDateTime {
    calendar(day).and_hms_opt(hour, 30, 0).unwrap()
}

fn a_second_shop(conn: &mut SqliteConnection) {
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(conn)
        .unwrap();
}

fn a_supplier(conn: &mut SqliteConnection, shop_id: i32, id: i32) {
    diesel::sql_query(format!(
        "INSERT INTO suppliers (id, shop_id, name) VALUES ({id}, {shop_id}, 'Fournisseur {id}')"
    ))
    .execute(conn)
    .unwrap();
}

/// A payment to a supplier, written by hand: T2 owns that service, and what
/// this file needs is the row on `supplier_ledger` the cash position reads.
fn pay_supplier(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
    centimes: i64,
    method: PaymentMethod,
    when: NaiveDateTime,
) {
    let stamped = when.format("%Y-%m-%d %H:%M:%S");
    let mode = method.as_str();
    diesel::sql_query(format!(
        "INSERT INTO supplier_ledger \
         (shop_id, supplier_id, kind, debit_centimes, credit_centimes, user_id, created_at, payment_mode) \
         VALUES ({shop_id}, {supplier_id}, 'payment', 0, {centimes}, {OWNER}, '{stamped}', '{mode}')"
    ))
    .execute(conn)
    .unwrap();
}

fn spend(conn: &mut SqliteConnection, centimes: i64, on: NaiveDate) {
    let category = expenses::categories(conn, SHOP).unwrap()[0].id;
    expenses::create(
        conn,
        SHOP,
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

#[allow(clippy::too_many_arguments)]
fn a_document(
    conn: &mut SqliteConnection,
    shop_id: i32,
    kind: DocumentKind,
    mode: PaymentMode,
    total_ttc: i64,
    stamp: i64,
    issued_at: NaiveDateTime,
    customer_id: Option<i32>,
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
            customer_id,
            buyer: None,
            ref_document_id: None,
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
