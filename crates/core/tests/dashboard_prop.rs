// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The dashboard adds up, over months nobody wrote by hand.
//!
//! The case builds a random history out of the things a shop does, keeps its
//! own tally of what each of them is worth as it writes them, and compares
//! that tally with what `dashboard::read` reads back out of the file. The
//! tally is kept the way a comptable would keep it: what a line asked for,
//! and what the goods on it cost on the day they left. The service reads it
//! back off the stock ledger, so the fiche's cost price moving between a sale
//! and the credit note that reverses it is exactly what the two would
//! disagree about.
//!
//! Three properties, and none of them is true by construction:
//! - the revenue less the cost of the goods is the margin, to the centime,
//!   against a tally built from the prices and the costs of the day;
//! - the day the case asks for, plus the other days of its month, is the
//!   month;
//! - the low stock list is the products a plain scan of the fiches would
//!   name.
//!
//! Against a real temp SQLite file, one per case, because the filters under
//! test are SQL.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::sqlite::SqliteConnection;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::money::{Bps, Money, PaymentMode};
use dzpos_core::services::dashboard::{self, Figures};
use dzpos_core::services::documents::DocumentKind;
use dzpos_core::services::expenses::{self, NewExpense};
use dzpos_core::services::sales::{self, NewSale, NewSaleLine, SaleKind};
use dzpos_core::services::{avoir, documents, products};
use proptest::prelude::*;

mod common;

const SHOP: i32 = 1;
const OWNER: i32 = 1;
const YEAR: i32 = 2026;
const MONTH: u32 = 9;
/// The day the dashboard is asked for. Rows land on it, on the days on
/// either side of it inside the month, and on the last day of the month
/// before and the first of the month after, so both sets of bounds are under
/// test on every case.
const THE_DAY: u32 = 15;

/// The three days of the month a row can land on, in order.
const IN_THE_MONTH: [u32; 3] = [THE_DAY - 1, THE_DAY, THE_DAY + 1];

/// Where a row lands.
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

    const fn in_the_day(self) -> bool {
        matches!(self, When::OnTheDay)
    }

    /// Which of the month's three days this is, if it is one of them.
    const fn index(self) -> Option<usize> {
        match self {
            When::DayBefore => Some(0),
            When::OnTheDay => Some(1),
            When::DayAfter => Some(2),
            When::MonthBefore | When::MonthAfter => None,
        }
    }
}

/// One thing the shop does, and the day it does it on.
#[derive(Debug, Clone, Copy)]
enum Event {
    /// A cash ticket: which product, how many units, and the remise given off
    /// the whole ticket.
    Ticket {
        product: usize,
        units: i64,
        discount: i64,
    },
    /// A credit facture, a delivery that moves the fiche's cost, and then an
    /// avoir crediting the facture in full, all on the same day. Both papers
    /// are in the figures and what they leave is nothing, which is only true
    /// if the credit note put the goods back at the cost they left on: the
    /// delivery in the middle is what tells the two apart.
    FactureThenAvoir {
        product: usize,
        units: i64,
        cost_between: i64,
    },
    /// A cash ticket rung up and annulled the same day. It leaves the figures
    /// with its own status, and so do the goods it put back.
    TicketThenCancel { product: usize, units: i64 },
    /// A delivery: the fiche's cost price moves. Nothing is sold, and every
    /// sale before it keeps the cost it left on.
    Restock { product: usize, cost: i64 },
    /// Money out that is not stock.
    Spend { amount: i64 },
}

/// The tally the case keeps for one stretch of days.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Expected {
    lines_ht: i64,
    discounts: i64,
    cost_of_goods: i64,
    sales_ttc: i64,
    sales_count: i64,
    expenses: i64,
}

impl Expected {
    const fn sales_ht(&self) -> i64 {
        self.lines_ht - self.discounts
    }

    const fn margin(&self) -> i64 {
        self.sales_ht() - self.cost_of_goods
    }
}

fn calendar(day: u32) -> NaiveDate {
    calendar_in(MONTH, day)
}

fn calendar_in(month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(YEAR, month, day).unwrap()
}

/// The moment a paper of that day is issued at. The hour walks so two papers
/// of one day never share a second, and it stays inside the day.
fn moment(day: NaiveDate, nth: u32) -> NaiveDateTime {
    day.and_hms_opt(8 + nth % 12, nth % 60, 0).unwrap()
}

/// The two products every case sells, priced so a whole month of them still
/// fits an i64 with room to spare and taxed at nothing, so `total_ttc` is the
/// lines' own HT and the tally has no rounding to model.
fn a_product(conn: &mut SqliteConnection, name: &str, selling: i64, cost: i64) -> i32 {
    products::create(
        conn,
        SHOP,
        OWNER,
        NewProduct {
            name: name.to_string(),
            barcode: None,
            category_id: None,
            unit: Unit::Piece,
            cost: Money::centimes(cost),
            selling: Money::centimes(selling),
            wholesale: None,
            qty_on_hand_milli: 1_000_000,
            low_stock_at_milli: 20_000,
            rate_bps: Some(Bps::new(0).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id
}

/// Moves the fiche's cost the way a delivery does.
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
            low_stock_at_milli: 20_000,
            rate_bps: Some(Bps::new(0).unwrap()),
            active: true,
        },
    )
    .unwrap();
}

fn sell(
    conn: &mut SqliteConnection,
    product_id: i32,
    units: i64,
    discount: i64,
    kind: SaleKind,
    customer_id: Option<i32>,
    at: NaiveDateTime,
) -> dzpos_core::services::documents::Document {
    let mode = match kind {
        SaleKind::Facture => PaymentMode::Credit,
        _ => PaymentMode::Cash,
    };
    sales::issue(
        conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![NewSaleLine {
                product_id,
                qty_milli: units.saturating_mul(1_000),
                unit_price: None,
                line_discount: Money::ZERO,
            }],
            global_discount: Money::centimes(discount),
            payment_mode: mode,
            tendered: match mode {
                PaymentMode::Cash => Some(Money::centimes(100_000_000)),
                _ => None,
            },
            customer_id,
            override_credit: false,
            kind,
            issued_at: Some(at),
        },
    )
    .unwrap()
    .document
}

fn when() -> impl Strategy<Value = When> {
    prop_oneof![
        Just(When::MonthBefore),
        Just(When::DayBefore),
        Just(When::OnTheDay),
        Just(When::DayAfter),
        Just(When::MonthAfter),
    ]
}

fn event() -> impl Strategy<Value = Event> {
    prop_oneof![
        (0usize..2, 1i64..6, 0i64..300).prop_map(|(product, units, discount)| Event::Ticket {
            product,
            units,
            discount
        }),
        (0usize..2, 1i64..6, 100i64..9_000).prop_map(|(product, units, cost_between)| {
            Event::FactureThenAvoir {
                product,
                units,
                cost_between,
            }
        }),
        (0usize..2, 1i64..6)
            .prop_map(|(product, units)| Event::TicketThenCancel { product, units }),
        (0usize..2, 100i64..9_000).prop_map(|(product, cost)| Event::Restock { product, cost }),
        (10i64..5_000).prop_map(|amount| Event::Spend { amount }),
    ]
}

fn history() -> impl Strategy<Value = Vec<(When, Event)>> {
    prop::collection::vec((when(), event()), 0..14)
}

/// The figure the service answered, as the tally holds it.
fn as_expected(figures: &Figures) -> Expected {
    Expected {
        lines_ht: figures.lines_ht.as_centimes(),
        discounts: figures.discounts.as_centimes(),
        cost_of_goods: figures.cost_of_goods.as_centimes(),
        sales_ttc: figures.sales_ttc.as_centimes(),
        sales_count: figures.sales_count,
        expenses: figures.expenses.as_centimes(),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn the_margin_the_day_and_the_low_stock_list_are_what_the_rows_say(
        history in history()
    ) {
        let (_dir, mut conn) = common::open_temp_selling_factures();
        let category = expenses::categories(&mut conn, SHOP).unwrap()[0].id;
        let customer = common::an_identified_customer(&mut conn, "Entreprise Benali");
        let names = ["Ciment", "Sable"];
        let selling = [10_000i64, 4_000i64];
        let ids = [
            a_product(&mut conn, names[0], selling[0], 5_000),
            a_product(&mut conn, names[1], selling[1], 1_500),
        ];
        // What each product costs right now, the way the fiche holds it. A
        // sale is tallied at this, and a delivery moves it.
        let mut cost_now = [5_000i64, 1_500i64];

        // One tally for the day the dashboard is asked for, one for each day
        // of its month. Built by the same walk, so they cannot drift apart.
        let mut for_the_day = Expected::default();
        let mut per_day: [Expected; 3] = [Expected::default(); 3];
        let mut nth = 0u32;

        for (when, event) in history {
            nth = nth.wrapping_add(1);
            let day = when.day();
            let at = moment(day, nth);
            let in_month = when.index();

            // The tally the row is folded into, if any.
            let mut fold = |f: &mut dyn FnMut(&mut Expected)| {
                if when.in_the_day() {
                    f(&mut for_the_day);
                }
                if let Some(index) = in_month {
                    f(&mut per_day[index]);
                }
            };

            match event {
                Event::Ticket { product, units, discount } => {
                    let ht = selling[product].saturating_mul(units);
                    // A remise never takes a paper below nothing.
                    let discount = discount.min(ht);
                    let cost = cost_now[product].saturating_mul(units);
                    sell(&mut conn, ids[product], units, discount, SaleKind::Ticket, None, at);
                    fold(&mut |e: &mut Expected| {
                        e.lines_ht += ht;
                        e.discounts += discount;
                        e.cost_of_goods += cost;
                        e.sales_ttc += ht - discount;
                        e.sales_count += 1;
                    });
                }
                Event::FactureThenAvoir { product, units, cost_between } => {
                    let ht = selling[product].saturating_mul(units);
                    let facture = sell(
                        &mut conn,
                        ids[product],
                        units,
                        0,
                        SaleKind::Facture,
                        Some(customer),
                        at,
                    );
                    // The delivery between the two papers. The credit note
                    // that follows has to ignore it.
                    move_the_cost(&mut conn, ids[product], names[product], selling[product], cost_between);
                    cost_now[product] = cost_between;
                    avoir::issue(&mut conn, SHOP, OWNER, facture.id, None, None, Some(at))
                        .unwrap();
                    // The facture is in the figures and so is the credit note
                    // that undoes it: both sides of the margin come to
                    // nothing, and the ttc column keeps what was rung up.
                    fold(&mut |e: &mut Expected| {
                        e.sales_ttc += ht;
                        e.sales_count += 1;
                    });
                }
                Event::TicketThenCancel { product, units } => {
                    let ticket = sell(
                        &mut conn,
                        ids[product],
                        units,
                        0,
                        SaleKind::Ticket,
                        None,
                        at,
                    );
                    documents::cancel(
                        &mut conn,
                        SHOP,
                        OWNER,
                        ticket.id,
                        "erreur de saisie".to_string(),
                        Some(at),
                    )
                    .unwrap();
                    // Nothing is folded: an annulled paper sold nothing and
                    // cost nothing.
                }
                Event::Restock { product, cost } => {
                    move_the_cost(&mut conn, ids[product], names[product], selling[product], cost);
                    cost_now[product] = cost;
                }
                Event::Spend { amount } => {
                    expenses::create(
                        &mut conn,
                        SHOP,
                        OWNER,
                        NewExpense {
                            category_id: category,
                            amount: Money::centimes(amount),
                            expense_date: day,
                            note: None,
                        },
                    )
                    .unwrap();
                    fold(&mut |e: &mut Expected| e.expenses += amount);
                }
            }
        }

        let read = dashboard::read(&mut conn, SHOP, calendar(THE_DAY)).unwrap();

        // The day is what the rows of that day say, column for column, and
        // the margin is the revenue less the cost to the centime.
        prop_assert_eq!(as_expected(&read.today), for_the_day);
        prop_assert_eq!(read.today.sales_ht.as_centimes(), for_the_day.sales_ht());
        prop_assert_eq!(read.today.margin.as_centimes(), for_the_day.margin());
        prop_assert_eq!(
            read.today.margin.as_centimes() + read.today.cost_of_goods.as_centimes(),
            read.today.sales_ht.as_centimes()
        );

        // The day the case asked for, plus the other days of its month, is
        // the month. Read day by day out of the service rather than out of
        // the tally, so a month bound that leaked would show here.
        let mut summed = Expected::default();
        for day in IN_THE_MONTH {
            let one = dashboard::read(&mut conn, SHOP, calendar(day)).unwrap();
            let e = as_expected(&one.today);
            summed.lines_ht += e.lines_ht;
            summed.discounts += e.discounts;
            summed.cost_of_goods += e.cost_of_goods;
            summed.sales_ttc += e.sales_ttc;
            summed.sales_count += e.sales_count;
            summed.expenses += e.expenses;
        }
        prop_assert_eq!(as_expected(&read.this_month), summed);
        prop_assert_eq!(
            read.this_month.margin.as_centimes() + read.this_month.cost_of_goods.as_centimes(),
            read.this_month.sales_ht.as_centimes()
        );

        // The low stock list is what a plain scan of the fiches names.
        let mut scanned: Vec<(i32, i64, i64)> = products::list(&mut conn, SHOP)
            .unwrap()
            .into_iter()
            .filter(|p| p.active && p.qty_on_hand_milli < p.low_stock_at_milli)
            .map(|p| (p.id, p.qty_on_hand_milli, p.low_stock_at_milli))
            .collect();
        scanned.sort_by_key(|row| row.0);
        let mut listed: Vec<(i32, i64, i64)> = read
            .low_stock
            .iter()
            .map(|l| (l.product_id, l.qty_on_hand_milli, l.low_stock_at_milli))
            .collect();
        listed.sort_by_key(|row| row.0);
        prop_assert_eq!(listed, scanned);

        // The month's top lists are the same rows read another way: what one
        // product brought in less what it cost is its margin, and the two
        // lists rank the same set.
        for product in read.top_by_quantity.iter().chain(read.top_by_margin.iter()) {
            prop_assert_eq!(
                product.margin.as_centimes(),
                product.sales_ht.as_centimes() - product.cost_of_goods.as_centimes()
            );
        }
        prop_assert!(read.top_by_quantity.len() <= 2);
        prop_assert_eq!(read.month.as_text(), format!("{YEAR:04}-{MONTH:02}"));
        prop_assert_eq!(read.day, calendar(THE_DAY));
        // Nothing on this file is a supply-side paper, so the only kind the
        // figures ever counted is a sale.
        prop_assert!(read.open_purchases == 0);
        prop_assert_eq!(
            documents::list(&mut conn, SHOP, Some(DocumentKind::Proforma)).unwrap().len(),
            0
        );
    }
}
