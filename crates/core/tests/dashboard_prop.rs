// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The dashboard adds up, over months nobody wrote by hand.
//!
//! The case builds a random history out of the things a shop does and keeps
//! its own tally as it writes them. The tally is the load-bearing check: it
//! is compared with `dashboard::read` column for column, for the day the case
//! asks about and for each of the three days of its month, so a filter that
//! let the wrong paper through, signed it the wrong way or dated it by the
//! wrong column shows up as a column that disagrees.
//!
//! Where the tally's figures come from, and why
//! - The revenue side is the paper's own `total_ht` and `discount`, read off
//!   the document the service handed back. That is not the dashboard's
//!   arithmetic: the dashboard sums `document_lines`, so folding the header's
//!   totals is what catches lines and header drifting apart, and it leaves
//!   the avoir's own prorating to the file that pins it (`avoir_prop`).
//! - The cost side the case computes itself, from the cost the fiche carried
//!   at the moment of the sale. The dashboard never reads the fiche, so this
//!   is the independent half: an avoir written after a delivery, or after a
//!   purchase moved the cost price, is where the two would part.
//!
//! A cancelled paper takes its whole chain with it. The tally holds one
//! posting per paper, tagged with the chain it belongs to, and a cancellation
//! drops the chain's postings rather than posting a reversal, which is what
//! the service does by dropping the paper and every avoir written against it.
//!
//! Against a real temp SQLite file, one per case, because the filters under
//! test are SQL.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::sqlite::SqliteConnection;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::money::{Bps, Money, PaymentMode};
use dzpos_core::services::dashboard::{self, Figures};
use dzpos_core::services::documents::Document;
use dzpos_core::services::expenses::{self, NewExpense};
use dzpos_core::services::purchases::{self, NewLine, NewPurchase, ReceiveLine};
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

    /// Which of the month's three days this is, if it is one of them.
    const fn index(self) -> Option<usize> {
        match self {
            When::DayBefore => Some(0),
            When::OnTheDay => Some(1),
            When::DayAfter => Some(2),
            When::MonthBefore | When::MonthAfter => None,
        }
    }

    /// The day a paper written against this one is credited on: the same day,
    /// a later day of the month, or a day of the month after. The three are
    /// what tell a figure dated by the paper apart from one dated by the row
    /// the file happened to write.
    fn later(self, steps: u8) -> Self {
        match (self, steps) {
            (_, 0) => self,
            (When::MonthBefore, _) => When::DayBefore,
            (When::DayBefore, 1) => When::OnTheDay,
            (When::DayBefore, _) | (When::OnTheDay, 1) => When::DayAfter,
            (When::OnTheDay, _) | (When::DayAfter, _) | (When::MonthAfter, _) => When::MonthAfter,
        }
    }
}

/// One thing the shop does.
#[derive(Debug, Clone, Copy)]
enum Event {
    /// A sale over the counter: which product, thousandths of the unit, the
    /// remise off the whole paper, and how it was paid. Cash and card are
    /// tickets; credit is a facture, because that is what a customer signs
    /// for.
    Sell {
        product: usize,
        qty_milli: i64,
        discount: i64,
        mode: PaymentMode,
    },
    /// A facture, then a credit note for part or all of it, on the same day
    /// or a later one. The remise makes the credit note prorate it.
    SellThenAvoir {
        product: usize,
        qty_milli: i64,
        discount: i64,
        back_milli: i64,
        /// A delivery between the two: the fiche's cost moves, and the credit
        /// note has to ignore it.
        cost_between: i64,
        later: u8,
    },
    /// A facture, a credit note for part of it, and then the whole paper
    /// annulled. Everything written on that chain leaves the figures.
    SellAvoirThenCancel {
        product: usize,
        qty_milli: i64,
        back_milli: i64,
        later: u8,
    },
    /// A cash ticket rung up and annulled.
    SellThenCancel {
        product: usize,
        qty_milli: i64,
        later: u8,
    },
    /// An order placed and taken in, whole or in part. It moves stock and the
    /// supplier's ledger and it moves the fiche's cost to the landed one; it
    /// is on no sale, so no figure of the margin may feel it.
    Buy {
        product: usize,
        qty_milli: i64,
        unit_cost: i64,
        /// Whether the delivery is the whole order or half of it, which is
        /// what leaves an order open.
        whole: bool,
        /// Goods handed straight back to the supplier.
        send_back: bool,
    },
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
    fn add(&mut self, other: &Expected) {
        self.lines_ht += other.lines_ht;
        self.discounts += other.discounts;
        self.cost_of_goods += other.cost_of_goods;
        self.sales_ttc += other.sales_ttc;
        self.sales_count += other.sales_count;
        self.expenses += other.expenses;
    }
}

/// One paper's contribution, held rather than folded, so a cancellation can
/// take a whole chain of them back out.
#[derive(Debug, Clone, Copy)]
struct Posting {
    when: When,
    /// The paper this was written against, or the paper itself: everything
    /// on one chain leaves together.
    chain: usize,
    delta: Expected,
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

/// `centimes × qty_milli / 1000`, rounded half away from zero, which is how
/// the core prices a quantity (dz-money). Written out here so the tally's
/// cost side is the case's own arithmetic and not the code under test.
fn mul_milli(centimes: i64, qty_milli: i64) -> i64 {
    let raw = i128::from(centimes) * i128::from(qty_milli);
    let magnitude = raw.abs();
    let rounded = (magnitude + 500) / 1_000;
    let signed = if raw < 0 { -rounded } else { rounded };
    i64::try_from(signed).unwrap()
}

/// The two products every case sells, taxed at nothing so `total_ttc` is the
/// paper's own HT and the tally has no tax to model. The stock is deep enough
/// that no case runs it down to a level a low stock list would move on.
fn a_product(conn: &mut SqliteConnection, name: &str, selling: i64, cost: i64) -> i32 {
    products::create(
        conn,
        SHOP,
        OWNER,
        NewProduct {
            name: name.to_string(),
            barcode: None,
            category_id: None,
            unit: Unit::Kg,
            cost: Money::centimes(cost),
            selling: Money::centimes(selling),
            wholesale: None,
            qty_on_hand_milli: 10_000_000,
            low_stock_at_milli: 0,
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
            unit: Unit::Kg,
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

fn sell(
    conn: &mut SqliteConnection,
    product_id: i32,
    qty_milli: i64,
    discount: i64,
    mode: PaymentMode,
    customer_id: i32,
    at: NaiveDateTime,
) -> Document {
    let kind = match mode {
        PaymentMode::Credit => SaleKind::Facture,
        _ => SaleKind::Ticket,
    };
    sales::issue(
        conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![NewSaleLine {
                product_id,
                qty_milli,
                unit_price: None,
                line_discount: Money::ZERO,
            }],
            global_discount: Money::centimes(discount),
            payment_mode: mode,
            tendered: match mode {
                PaymentMode::Cash => Some(Money::centimes(100_000_000)),
                _ => None,
            },
            // A facture names its buyer; a ticket is handed to whoever is at
            // the counter, and naming one on it changes no figure here.
            customer_id: match kind {
                SaleKind::Facture => Some(customer_id),
                _ => None,
            },
            override_credit: true,
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

fn mode() -> impl Strategy<Value = PaymentMode> {
    prop_oneof![
        Just(PaymentMode::Cash),
        Just(PaymentMode::Card),
        Just(PaymentMode::Credit),
    ]
}

/// Thousandths of a unit, most of them not a whole one: a product sold by
/// weight is where a line total and a cost round on their own.
fn qty() -> impl Strategy<Value = i64> {
    prop_oneof![
        Just(1_000i64),
        Just(2_000i64),
        Just(1_500i64),
        Just(2_500i64),
        Just(333i64),
        Just(4_125i64),
    ]
}

fn event() -> impl Strategy<Value = Event> {
    prop_oneof![
        (0usize..2, qty(), 0i64..300, mode()).prop_map(|(product, qty_milli, discount, mode)| {
            Event::Sell {
                product,
                qty_milli,
                discount,
                mode,
            }
        }),
        (0usize..2, qty(), 0i64..300, 100i64..9_000, 0u8..3).prop_map(
            |(product, qty_milli, discount, cost_between, later)| Event::SellThenAvoir {
                product,
                qty_milli,
                discount,
                // Part of the line, and the whole of it when the halving
                // lands back on the quantity.
                back_milli: (qty_milli / 2).max(1),
                cost_between,
                later,
            }
        ),
        (0usize..2, qty(), 0u8..3).prop_map(|(product, qty_milli, later)| {
            Event::SellAvoirThenCancel {
                product,
                qty_milli,
                back_milli: (qty_milli / 3).max(1),
                later,
            }
        }),
        (0usize..2, qty(), 0u8..3).prop_map(|(product, qty_milli, later)| Event::SellThenCancel {
            product,
            qty_milli,
            later
        }),
        (
            0usize..2,
            qty(),
            100i64..9_000,
            any::<bool>(),
            any::<bool>()
        )
            .prop_map(
                |(product, qty_milli, unit_cost, whole, send_back)| Event::Buy {
                    product,
                    // An order is written in whole units of stock so the halved
                    // delivery below is a quantity the line can carry.
                    qty_milli: qty_milli.max(2) * 2,
                    unit_cost,
                    whole,
                    send_back,
                }
            ),
        (10i64..5_000).prop_map(|amount| Event::Spend { amount }),
    ]
}

fn history() -> impl Strategy<Value = Vec<(When, Event)>> {
    prop::collection::vec((when(), event()), 0..12)
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
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn the_dashboard_folds_the_rows_that_belong_in_it_and_no_others(
        history in history()
    ) {
        let (_dir, mut conn) = common::open_temp_selling_factures();
        let category = expenses::categories(&mut conn, SHOP).unwrap()[0].id;
        let customer = common::an_identified_customer(&mut conn, "Entreprise Benali");
        let supplier = common::a_supplier(&mut conn, "Cimenterie de Meftah");
        let names = ["Ciment", "Sable"];
        let selling = [10_000i64, 4_000i64];
        let ids = [
            a_product(&mut conn, names[0], selling[0], 5_000),
            a_product(&mut conn, names[1], selling[1], 1_500),
        ];
        // What each product costs right now, the way the fiche holds it. A
        // sale is tallied at this; a delivery and an edit both move it.
        let mut cost_now = [5_000i64, 1_500i64];

        let mut postings: Vec<Posting> = Vec::new();
        let mut cancelled: Vec<usize> = Vec::new();
        let mut open_orders = 0i64;
        let mut nth = 0u32;

        for (when, event) in history {
            nth = nth.wrapping_add(1);
            let at = moment(when.day(), nth);
            // Every paper of one event is one chain: a facture, the credit
            // notes written against it, and the credit note a cancellation
            // issues all stand or fall together. Numbered by the event and
            // never by how many postings have been made, or an event that
            // posts nothing would hand its number to the next one.
            let chain = usize::try_from(nth).unwrap();

            match event {
                Event::Sell { product, qty_milli, discount, mode } => {
                    let sold = sell(&mut conn, ids[product], qty_milli, discount, mode, customer, at);
                    postings.push(Posting {
                        when,
                        chain,
                        delta: Expected {
                            lines_ht: sold.totals.total_ht.as_centimes(),
                            discounts: sold.totals.discount.as_centimes(),
                            cost_of_goods: mul_milli(cost_now[product], qty_milli),
                            sales_ttc: sold.totals.total_ttc.as_centimes(),
                            sales_count: 1,
                            expenses: 0,
                        },
                    });
                }
                Event::SellThenAvoir {
                    product, qty_milli, discount, back_milli, cost_between, later,
                } => {
                    let sold_at_cost = cost_now[product];
                    let facture = sell(
                        &mut conn, ids[product], qty_milli, discount,
                        PaymentMode::Credit, customer, at,
                    );
                    postings.push(Posting {
                        when,
                        chain,
                        delta: Expected {
                            lines_ht: facture.totals.total_ht.as_centimes(),
                            discounts: facture.totals.discount.as_centimes(),
                            cost_of_goods: mul_milli(sold_at_cost, qty_milli),
                            sales_ttc: facture.totals.total_ttc.as_centimes(),
                            sales_count: 1,
                            expenses: 0,
                        },
                    });
                    // The delivery between the two papers. The credit note
                    // that follows has to ignore it.
                    move_the_cost(&mut conn, ids[product], names[product], selling[product], cost_between);
                    cost_now[product] = cost_between;

                    let credited = when.later(later);
                    let back = back_milli.min(qty_milli);
                    let note = avoir::issue(
                        &mut conn, SHOP, OWNER, facture.id,
                        Some(vec![avoir::AvoirLine {
                            document_line_id: facture.lines[0].id,
                            qty_milli: back,
                        }]),
                        None,
                        Some(moment(credited.day(), nth.wrapping_add(30))),
                    ).unwrap();
                    postings.push(Posting {
                        when: credited,
                        chain,
                        delta: Expected {
                            lines_ht: -note.totals.total_ht.as_centimes(),
                            discounts: -note.totals.discount.as_centimes(),
                            cost_of_goods: -mul_milli(sold_at_cost, back),
                            ..Expected::default()
                        },
                    });
                }
                Event::SellAvoirThenCancel { product, qty_milli, back_milli, later } => {
                    let facture = sell(
                        &mut conn, ids[product], qty_milli, 0,
                        PaymentMode::Credit, customer, at,
                    );
                    let back = back_milli.min(qty_milli);
                    avoir::issue(
                        &mut conn, SHOP, OWNER, facture.id,
                        Some(vec![avoir::AvoirLine {
                            document_line_id: facture.lines[0].id,
                            qty_milli: back,
                        }]),
                        None,
                        Some(moment(when.day(), nth.wrapping_add(30))),
                    ).unwrap();
                    documents::cancel(
                        &mut conn, SHOP, OWNER, facture.id,
                        "erreur de saisie".to_string(),
                        Some(moment(when.later(later).day(), nth.wrapping_add(45))),
                    ).unwrap();
                    // Nothing is posted: the facture, the credit note written
                    // against it and the one the cancellation issued all
                    // leave the figures together.
                    cancelled.push(chain);
                }
                Event::SellThenCancel { product, qty_milli, later } => {
                    let ticket = sell(
                        &mut conn, ids[product], qty_milli, 0,
                        PaymentMode::Cash, customer, at,
                    );
                    documents::cancel(
                        &mut conn, SHOP, OWNER, ticket.id,
                        "erreur de saisie".to_string(),
                        Some(moment(when.later(later).day(), nth.wrapping_add(45))),
                    ).unwrap();
                    cancelled.push(chain);
                }
                Event::Buy { product, qty_milli, unit_cost, whole, send_back } => {
                    let order = purchases::save(
                        &mut conn, SHOP, OWNER,
                        NewPurchase {
                            supplier_id: supplier,
                            supplier_document_number: None,
                            purchase_date: when.day().format("%Y-%m-%d").to_string(),
                            due_date: None,
                            transport: Money::ZERO,
                            extra_costs: Money::ZERO,
                            note: None,
                            lines: vec![NewLine {
                                product_id: ids[product],
                                qty_ordered_milli: qty_milli,
                                unit_cost: Money::centimes(unit_cost),
                            }],
                            // Nothing handed over with the order: what a
                            // payment does to the supplier's ledger is
                            // `supplier_debt`'s business, and none of it
                            // reaches a figure on this screen.
                            paid_now: None,
                            receive_now: false,
                        },
                    ).unwrap();
                    let line_id = order.lines[0].id;
                    let taken = if whole { qty_milli } else { qty_milli / 2 };
                    purchases::receive(
                        &mut conn, SHOP, OWNER, order.purchase.id,
                        vec![ReceiveLine { purchase_line_id: line_id, qty_milli: taken }],
                        None,
                    ).unwrap();
                    if send_back {
                        purchases::return_to_supplier(
                            &mut conn, SHOP, OWNER, order.purchase.id,
                            vec![ReceiveLine { purchase_line_id: line_id, qty_milli: taken }],
                            None,
                        ).unwrap();
                    }
                    if !whole {
                        open_orders += 1;
                    }
                    // A delivery moves the fiche's cost to the landed one.
                    // Read back rather than assumed: what the landed cost is
                    // belongs to `purchase_prop`, and what this case needs is
                    // the figure a later sale will be written at.
                    cost_now[product] =
                        products::get(&mut conn, SHOP, ids[product]).unwrap().cost.as_centimes();
                    // Nothing is posted. A purchase and a return name no
                    // document, so no figure of the margin may feel them.
                }
                Event::Spend { amount } => {
                    expenses::create(
                        &mut conn, SHOP, OWNER,
                        NewExpense {
                            category_id: category,
                            amount: Money::centimes(amount),
                            expense_date: when.day(),
                            note: None,
                        },
                    ).unwrap();
                    postings.push(Posting {
                        when,
                        chain,
                        delta: Expected { expenses: amount, ..Expected::default() },
                    });
                }
            }
        }

        // The tally, folded once every paper is written: a chain annulled
        // later takes the postings it made earlier back out.
        let mut for_the_day = Expected::default();
        let mut per_day: [Expected; 3] = [Expected::default(); 3];
        for posting in &postings {
            if cancelled.contains(&posting.chain) {
                continue;
            }
            if posting.when == When::OnTheDay {
                for_the_day.add(&posting.delta);
            }
            if let Some(index) = posting.when.index() {
                per_day[index].add(&posting.delta);
            }
        }
        let mut for_the_month = Expected::default();
        for one in &per_day {
            for_the_month.add(one);
        }

        let read = dashboard::read(&mut conn, SHOP, calendar(THE_DAY)).unwrap();

        // The load-bearing check, both ranges, column for column.
        prop_assert_eq!(as_expected(&read.today), for_the_day);
        prop_assert_eq!(as_expected(&read.this_month), for_the_month);

        // And the month read day by day out of the service, which is what a
        // month bound that leaked from either side would break.
        let mut summed = Expected::default();
        for day in [THE_DAY - 1, THE_DAY, THE_DAY + 1] {
            let one = dashboard::read(&mut conn, SHOP, calendar(day)).unwrap();
            summed.add(&as_expected(&one.today));
        }
        prop_assert_eq!(as_expected(&read.this_month), summed);

        // Orders still waiting on goods, which no sale figure feels.
        prop_assert_eq!(read.open_purchases, open_orders);

        // The top lists are the same rows read another way, and a product
        // whose month came to nothing is off them.
        for product in read.top_by_quantity.iter().chain(read.top_by_margin.iter()) {
            prop_assert_eq!(
                product.margin.as_centimes(),
                product.lines_ht.as_centimes() - product.cost_of_goods.as_centimes()
            );
            prop_assert!(product.qty_milli != 0 || product.lines_ht != Money::ZERO);
        }

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

        prop_assert_eq!(read.month.as_text(), format!("{YEAR:04}-{MONTH:02}"));
        prop_assert_eq!(read.day, calendar(THE_DAY));
    }
}
