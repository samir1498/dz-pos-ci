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
use dzpos_core::services::clock::{Month, Period};
use dzpos_core::services::debt::{self, DebtKind, NewDebtEntry, PaymentMethod};
use dzpos_core::services::documents::{
    self, DocumentKind, NewDocument, NewDocumentLine, SellerBlock,
};
use dzpos_core::services::expenses::{self, NewExpense};
use proptest::prelude::*;

mod common;

const SHOP: i32 = 1;
const OWNER: i32 = 1;
/// The month the position is asked for, and the day inside it. Rows land on
/// that day, on the days on either side of it, and on the last day of the
/// month before and the first of the month after, so both sets of bounds are
/// under test on every case.
const YEAR: i32 = 2026;
const MONTH: u32 = 9;
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

/// Where the event lands. Five places, because the case asks the position
/// twice: for the day, where only `OnTheDay` counts, and for the month, where
/// the three September days count and the two rows on either side of the
/// month do not. The last day of August and the first of October are the
/// bounds a month query gets wrong if it works in whole months rather than in
/// days.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum When {
    MonthBefore,
    DayBefore,
    OnTheDay,
    DayAfter,
    MonthAfter,
}

impl When {
    fn day(self) -> NaiveDate {
        match self {
            When::MonthBefore => calendar_in(8, 31),
            When::DayBefore => calendar(THE_DAY - 1),
            When::OnTheDay => calendar(THE_DAY),
            When::DayAfter => calendar(THE_DAY + 1),
            When::MonthAfter => calendar_in(10, 1),
        }
    }

    /// Whether a row here reaches the day the case asks for.
    const fn in_the_day(self) -> bool {
        matches!(self, When::OnTheDay)
    }

    /// Whether it reaches the month the case asks for.
    const fn in_the_month(self) -> bool {
        matches!(self, When::DayBefore | When::OnTheDay | When::DayAfter)
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

impl Expected {
    /// Folds one row's effect in. The case keeps two of these, one per range
    /// it asks about, and a row is added to whichever ones it falls in.
    fn add(&mut self, other: Expected) {
        self.cash_sales += other.cash_sales;
        self.cash_stamp += other.cash_stamp;
        self.cash_customer_payments += other.cash_customer_payments;
        self.supplier_cash += other.supplier_cash;
        self.expenses += other.expenses;
        self.card_sales += other.card_sales;
        self.card_customer_payments += other.card_customer_payments;
    }
}

/// What the service answered for one range, against the tally the case kept
/// while it was writing. Every column on its own: a total alone would pass
/// with two filters wrong in opposite directions.
fn agrees(
    position: &cash::CashPosition,
    expected: Expected,
    range: &str,
) -> Result<(), TestCaseError> {
    prop_assert_eq!(
        position.cash_in.sales.as_centimes(),
        expected.cash_sales,
        "{}: cash sales",
        range
    );
    prop_assert_eq!(
        position.cash_in.stamp.as_centimes(),
        expected.cash_stamp,
        "{}: stamp",
        range
    );
    prop_assert_eq!(
        position.cash_in.customer_payments.as_centimes(),
        expected.cash_customer_payments,
        "{}: cash customer payments",
        range
    );
    prop_assert_eq!(position.cash_out.refunds, Money::ZERO, "{}: refunds", range);
    prop_assert_eq!(
        position.cash_out.supplier_payments.as_centimes(),
        expected.supplier_cash,
        "{}: supplier cash",
        range
    );
    prop_assert_eq!(
        position.cash_out.expenses.as_centimes(),
        expected.expenses,
        "{}: expenses",
        range
    );
    prop_assert_eq!(
        position.card_in.sales.as_centimes(),
        expected.card_sales,
        "{}: card sales",
        range
    );
    // Nothing here writes a stamped card document, so the tax never reaches
    // this side.
    prop_assert_eq!(position.card_in.stamp, Money::ZERO, "{}: card stamp", range);
    prop_assert_eq!(
        position.card_in.customer_payments.as_centimes(),
        expected.card_customer_payments,
        "{}: card customer payments",
        range
    );
    // Redundant given the columns above, and kept: it is the one line that
    // reads the service's own subtraction rather than its filters, so a
    // checked_sub written the wrong way round fails here and nowhere else.
    prop_assert_eq!(
        position.cash.as_centimes(),
        expected.cash_sales + expected.cash_customer_payments
            - expected.supplier_cash
            - expected.expenses,
        "{}: the net",
        range
    );
    Ok(())
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
    let when = prop_oneof![
        Just(When::MonthBefore),
        Just(When::DayBefore),
        Just(When::OnTheDay),
        Just(When::DayAfter),
        Just(When::MonthAfter),
    ];
    prop::collection::vec((when, event), 1..=14)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn every_column_of_the_day_and_of_the_month_is_the_sum_of_the_rows_that_belong_in_it(
        events in events()
    ) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let mut conn = dzpos_core::db::open(&path).unwrap();
        let customer = common::a_customer(&mut conn, "Entreprise Benali");
        a_second_shop(&mut conn);
        a_supplier(&mut conn, SHOP, 1);
        a_supplier(&mut conn, 2, 2);
        // One tally per range the case asks about. A row is folded into
        // whichever of them it falls in, so the two are built by the same
        // walk and cannot drift apart.
        let mut for_the_day = Expected::default();
        let mut for_the_month = Expected::default();
        // Two documents can be issued inside one second and the hour only has
        // to stay on the right day, so it walks up and wraps.
        let mut hour = 0;

        for (when, event) in &events {
            let day = when.day();
            hour = (hour + 1) % 24;
            let mut delta = Expected::default();
            match *event {
                Event::Sell(kind, mode, total_ttc, stamp) => {
                    a_document(&mut conn, SHOP, kind, mode, total_ttc, stamp, moment(day, hour), None);
                    // A proforma is a quotation and no money moved for it; a
                    // credit sale is a debt, not a drawer.
                    let sold = matches!(kind, DocumentKind::Ticket | DocumentKind::Facture);
                    if sold {
                        match mode {
                            // What the drawer took is `net_to_pay`: the stamp
                            // came over the counter with the rest.
                            PaymentMode::Cash => {
                                delta.cash_sales = total_ttc + stamp;
                                delta.cash_stamp = stamp;
                            }
                            PaymentMode::Card => delta.card_sales = total_ttc,
                            PaymentMode::Credit => {}
                        }
                    }
                }
                Event::SellThenCancel(mode, total_ttc) => {
                    let id = a_document(
                        &mut conn, SHOP, DocumentKind::Ticket, mode, total_ttc, 0, moment(day, hour), None,
                    );
                    documents::cancel(
                        &mut conn, SHOP, OWNER, id, "erreur".to_string(), Some(moment(day, hour)),
                    ).unwrap();
                    // The delta stays empty: an annulled ticket is money that
                    // never stayed in the drawer.
                }
                Event::Settle(method, centimes) => {
                    // The debt first, so the payment has something to land on.
                    // Written straight onto the ledger and dated before every
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
                        Some(moment(calendar_in(7, 1), hour)),
                    ).unwrap();
                    debt::pay(
                        &mut conn, SHOP, OWNER, customer, Money::centimes(centimes),
                        method, None, moment(day, hour),
                    ).unwrap();
                    match method {
                        PaymentMethod::Cash => delta.cash_customer_payments = centimes,
                        PaymentMethod::Card => delta.card_customer_payments = centimes,
                    }
                }
                Event::PaySupplier(method, centimes) => {
                    pay_supplier(&mut conn, SHOP, 1, centimes, method, moment(day, hour));
                    if method == PaymentMethod::Cash {
                        delta.supplier_cash = centimes;
                    }
                }
                Event::Spend(centimes) => {
                    spend(&mut conn, centimes, day);
                    delta.expenses = centimes;
                }
                Event::Elsewhere(centimes) => {
                    // The other shop sells, pays a supplier and spends on the
                    // same day. None of it is this shop's cash, so the delta
                    // stays empty for both ranges.
                    a_document(
                        &mut conn, 2, DocumentKind::Ticket, PaymentMode::Cash, centimes, 0,
                        moment(day, hour), None,
                    );
                    pay_supplier(&mut conn, 2, 2, centimes, PaymentMethod::Cash, moment(day, hour));
                }
            }
            if when.in_the_day() {
                for_the_day.add(delta);
            }
            if when.in_the_month() {
                for_the_month.add(delta);
            }
        }

        let day = cash::position(&mut conn, SHOP, Period::Day(calendar(THE_DAY))).unwrap();
        agrees(&day, for_the_day, "the day")?;

        // The same file read over the whole of September. The rows on 31
        // August and 1 October are what a month query gets wrong if it works
        // in whole months rather than in the days they cover.
        let month = cash::position(
            &mut conn,
            SHOP,
            Period::Month(Month::new(YEAR, MONTH).unwrap()),
        ).unwrap();
        prop_assert_eq!(month.from, calendar(1));
        prop_assert_eq!(month.to, calendar(30));
        agrees(&month, for_the_month, "the month")?;
    }
}

fn calendar(day: u32) -> NaiveDate {
    calendar_in(MONTH, day)
}

fn calendar_in(month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(YEAR, month, day).unwrap()
}

fn moment(day: NaiveDate, hour: u32) -> NaiveDateTime {
    day.and_hms_opt(hour, 30, 0).unwrap()
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

/// A payment to a supplier, written by hand: the supplier payment service
/// owns that, and what this file needs is the row on `supplier_ledger` the
/// cash position reads.
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
