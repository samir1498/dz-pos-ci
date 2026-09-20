// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The dashboard (features.md §1), against a real temp SQLite file. What is
//! asserted is what the service read back out of the ledgers, because that is
//! the whole of what the screen is: nothing on it is stored anywhere.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sql_types::{Integer, Text};
use diesel::sqlite::SqliteConnection;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::money::{Bps, Money, PaymentMode};
use dzpos_core::services::dashboard;
use dzpos_core::services::documents::Document;
use dzpos_core::services::sales::{self, NewSale, NewSaleLine, SaleKind};
use dzpos_core::services::{avoir, cancellation, cash, clock, products};

mod common;

use common::open_temp_selling_factures as open_temp;

const SHOP: i32 = 1;
const OWNER: i32 = 1;

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, d).unwrap()
}

fn at(d: u32, hour: u32) -> NaiveDateTime {
    day(d).and_hms_opt(hour, 0, 0).unwrap()
}

/// A product at `rate_bps` (most tests pass 0, so `total_ttc` is the lines'
/// own HT and every figure below is readable by hand; the TVA tests pass
/// the standard 1900 to tell `sales_ttc` and `sales_ht` apart).
fn product(conn: &mut SqliteConnection, name: &str, selling: i64, cost: i64, rate_bps: u32) -> i32 {
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
            qty_on_hand_milli: 100_000,
            low_stock_at_milli: 0,
            rate_bps: Some(Bps::new(rate_bps).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id
}

fn sell(
    conn: &mut SqliteConnection,
    product_id: i32,
    units: i64,
    kind: SaleKind,
    customer_id: Option<i32>,
    hour: u32,
) -> Document {
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
                qty_milli: units * 1_000,
                unit_price: None,
                line_discount: Money::ZERO,
            }],
            global_discount: Money::ZERO,
            payment_mode: mode,
            tendered: match mode {
                PaymentMode::Cash => Some(Money::centimes(10_000_000)),
                _ => None,
            },
            customer_id,
            override_credit: false,
            kind,
            issued_at: Some(at(15, hour)),
        },
    )
    .unwrap()
    .document
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

/// One order in whatever state the test needs. `purchases::save` is not what
/// is under test here; the count is.
fn a_purchase(conn: &mut SqliteConnection, supplier_id: i32, status: &str) {
    diesel::sql_query(
        "INSERT INTO purchases (shop_id, supplier_id, purchase_date, status, user_id) \
         VALUES (?, ?, '2026-09-15', ?, ?)",
    )
    .bind::<Integer, _>(SHOP)
    .bind::<Integer, _>(supplier_id)
    .bind::<Text, _>(status)
    .bind::<Integer, _>(OWNER)
    .execute(conn)
    .unwrap();
}

#[test]
fn a_day_nothing_happened_on_answers_zeros_and_not_nothing() {
    let (_dir, mut conn) = open_temp();
    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();

    assert_eq!(read.today.sales_ttc, Money::ZERO);
    assert_eq!(read.today.sales_count, 0);
    assert_eq!(read.today.sales_ht, Money::ZERO);
    assert_eq!(read.today.cost_of_goods, Money::ZERO);
    assert_eq!(read.today.margin, Money::ZERO);
    assert_eq!(read.today.expenses, Money::ZERO);
    assert_eq!(read.this_month.sales_ttc, Money::ZERO);
    assert_eq!(read.customer_debt.total, Money::ZERO);
    assert_eq!(read.customer_debt.parties, 0);
    assert_eq!(read.supplier_debt.parties, 0);
    assert_eq!(read.open_purchases, 0);
    assert!(read.low_stock.is_empty());
    assert!(read.top_by_quantity.is_empty());
    assert_eq!(read.month.as_text(), "2026-09");
}

#[test]
fn the_margin_is_what_the_lines_asked_for_less_what_the_goods_cost() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 10_000, 6_000, 0);
    sell(&mut conn, p, 3, SaleKind::Ticket, None, 9);

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    assert_eq!(read.today.sales_ttc, Money::centimes(30_000));
    assert_eq!(read.today.sales_count, 1);
    assert_eq!(read.today.sales_ht, Money::centimes(30_000));
    assert_eq!(read.today.cost_of_goods, Money::centimes(18_000));
    assert_eq!(read.today.margin, Money::centimes(12_000));
}

#[test]
fn sales_ttc_is_ht_plus_the_tva_rounded_once_per_rate_and_cash_in_reads_it() {
    // Every other product in this file carries rate_bps 0, where `sales_ttc`
    // and `sales_ht` (`lines_ht` less the remise, features.md's
    // `subtotal_ht`) read the same figure and the two are never told apart.
    // This one is taxed at the standard 19 % (features.md, TVA rates) and
    // sold with a remise, so they cannot agree by accident.
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment taxé", 3_367, 2_000, 1_900);

    sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![NewSaleLine {
                product_id: p,
                qty_milli: 3_000,
                unit_price: None,
                line_discount: Money::ZERO,
            }],
            global_discount: Money::centimes(34),
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(10_000_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(at(15, 9)),
        },
    )
    .unwrap();

    // Independent arithmetic, not the code's own helper: features.md's
    // totals table, `tva = round(subtotal_ht_at_rate × rate)`, half away
    // from zero, to the centime.
    let lines_ht = 3_367 * 3;
    let subtotal_ht = lines_ht - 34;
    assert_eq!(subtotal_ht, 10_067);
    let raw = subtotal_ht * 1_900; // subtotal_ht centimes × rate_bps
    let half_away_from_zero = (raw + (10_000 / 2)) / 10_000;
    assert_eq!(half_away_from_zero, 1_913, "1 912,73 rounds up to 1 913");
    let total_ttc = subtotal_ht + half_away_from_zero;
    assert_eq!(total_ttc, 11_980);
    // Under the 300,00 DA stamp floor (money::stamp::STAMP_FLOOR), so the
    // cash a customer hands over is exactly the TTC figure and nothing this
    // test computed is muddied by the droit de timbre.
    assert!(total_ttc <= 30_000);

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    assert_eq!(read.today.sales_ht, Money::centimes(subtotal_ht));
    assert_eq!(read.today.sales_ttc, Money::centimes(total_ttc));
    assert!(
        read.today.sales_ttc > read.today.sales_ht,
        "a taxed sale's TTC must read above its HT"
    );
    assert_eq!(
        read.cash_today.cash_in.sales,
        Money::centimes(total_ttc),
        "a cash sale's cash_in must read the TTC figure"
    );
}

#[test]
fn a_cancelled_sale_leaves_both_sides_of_the_margin_at_once() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 10_000, 6_000, 0);
    let ticket = sell(&mut conn, p, 3, SaleKind::Ticket, None, 9);
    cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        ticket.id,
        "erreur de saisie".to_string(),
        Some(at(15, 10)),
    )
    .unwrap();

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    assert_eq!(read.today.sales_count, 0, "an annulled ticket sold nothing");
    assert_eq!(read.today.sales_ttc, Money::ZERO);
    assert_eq!(read.today.sales_ht, Money::ZERO);
    assert_eq!(
        read.today.cost_of_goods,
        Money::ZERO,
        "the goods went back on the shelf, so they cost the day nothing"
    );
    assert_eq!(read.today.margin, Money::ZERO);
}

#[test]
fn a_cancelled_facture_and_the_credit_note_it_issued_leave_together() {
    // The trap this is here for: the cancellation of a facture on credit
    // writes a numbered avoir. Dropping the facture for being annulled and
    // then subtracting that avoir would reverse the sale twice, and the
    // month would show a margin below zero for a sale that simply never
    // happened.
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 10_000, 6_000, 0);
    let c = common::an_identified_customer(&mut conn, "Entreprise Benali");
    let facture = sell(&mut conn, p, 3, SaleKind::Facture, Some(c), 9);
    cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "erreur de saisie".to_string(),
        Some(at(15, 10)),
    )
    .unwrap();

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    assert_eq!(read.today.sales_ht, Money::ZERO);
    assert_eq!(read.today.cost_of_goods, Money::ZERO);
    assert_eq!(read.today.margin, Money::ZERO);
    assert_eq!(read.today.sales_count, 0);
}

#[test]
fn an_avoir_on_a_standing_facture_lowers_the_revenue_and_the_cost_together() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 10_000, 6_000, 0);
    let c = common::an_identified_customer(&mut conn, "Entreprise Benali");
    let facture = sell(&mut conn, p, 3, SaleKind::Facture, Some(c), 9);
    avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        Some(vec![avoir::AvoirLine {
            document_line_id: facture.lines[0].id,
            qty_milli: 1_000,
        }]),
        None,
        Some(at(15, 11)),
    )
    .unwrap();

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    // Two units kept: 200,00 of revenue and 120,00 of cost.
    assert_eq!(read.today.sales_ht, Money::centimes(20_000));
    assert_eq!(read.today.cost_of_goods, Money::centimes(12_000));
    assert_eq!(read.today.margin, Money::centimes(8_000));
    assert_eq!(
        read.today.sales_ttc,
        Money::centimes(30_000),
        "the facture still asked for its own amount over the counter"
    );
}

#[test]
fn a_remise_off_the_whole_document_comes_off_the_margin() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 10_000, 6_000, 0);
    sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![NewSaleLine {
                product_id: p,
                qty_milli: 3_000,
                unit_price: None,
                line_discount: Money::ZERO,
            }],
            global_discount: Money::centimes(5_000),
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(10_000_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(at(15, 9)),
        },
    )
    .unwrap();

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    assert_eq!(read.today.lines_ht, Money::centimes(30_000));
    assert_eq!(read.today.discounts, Money::centimes(5_000));
    assert_eq!(
        read.today.sales_ht,
        Money::centimes(25_000),
        "a remise the shop gave is money it did not collect"
    );
    assert_eq!(read.today.margin, Money::centimes(7_000));
}

#[test]
fn the_day_is_a_slice_of_its_month_and_neither_reaches_the_other_month() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 10_000, 6_000, 0);
    sell(&mut conn, p, 1, SaleKind::Ticket, None, 9);
    // The same shop, the day before and the day after, and the two days that
    // sit just outside the month.
    for (d, hour) in [(14u32, 9u32), (16, 9)] {
        let sale = NewSale {
            lines: vec![NewSaleLine {
                product_id: p,
                qty_milli: 1_000,
                unit_price: None,
                line_discount: Money::ZERO,
            }],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(10_000_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(day(d).and_hms_opt(hour, 0, 0).unwrap()),
        };
        sales::issue(&mut conn, SHOP, OWNER, sale).unwrap();
    }
    for outside in [
        NaiveDate::from_ymd_opt(2026, 8, 31).unwrap(),
        NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
    ] {
        let sale = NewSale {
            lines: vec![NewSaleLine {
                product_id: p,
                qty_milli: 1_000,
                unit_price: None,
                line_discount: Money::ZERO,
            }],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(10_000_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(outside.and_hms_opt(23, 59, 59).unwrap()),
        };
        sales::issue(&mut conn, SHOP, OWNER, sale).unwrap();
    }

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    assert_eq!(read.today.sales_count, 1);
    assert_eq!(
        read.this_month.sales_count, 3,
        "the two papers outside September are not September's"
    );
    assert_eq!(read.this_month.sales_ht, Money::centimes(30_000));
}

#[test]
fn the_cash_position_is_the_one_the_cash_rule_answers() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 10_000, 6_000, 0);
    sell(&mut conn, p, 3, SaleKind::Ticket, None, 9);

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    assert_eq!(
        read.cash_today,
        cash::position(&mut conn, SHOP, clock::Period::Day(day(15))).unwrap()
    );
    assert_eq!(
        read.cash_this_month,
        cash::position(
            &mut conn,
            SHOP,
            clock::Period::Month(clock::Month::of(day(15)))
        )
        .unwrap()
    );
}

#[test]
fn the_low_stock_list_names_the_products_in_use_that_have_fallen_under() {
    let (_dir, mut conn) = open_temp();
    let short = products::create(
        &mut conn,
        SHOP,
        OWNER,
        NewProduct {
            name: "Ciment".to_string(),
            barcode: None,
            category_id: None,
            unit: Unit::Piece,
            cost: Money::centimes(6_000),
            selling: Money::centimes(10_000),
            wholesale: None,
            qty_on_hand_milli: 2_000,
            low_stock_at_milli: 10_000,
            rate_bps: Some(Bps::new(0).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id;
    let retired = products::create(
        &mut conn,
        SHOP,
        OWNER,
        NewProduct {
            name: "Sable".to_string(),
            barcode: None,
            category_id: None,
            unit: Unit::Piece,
            cost: Money::centimes(1_000),
            selling: Money::centimes(2_000),
            wholesale: None,
            qty_on_hand_milli: 0,
            low_stock_at_milli: 10_000,
            rate_bps: Some(Bps::new(0).unwrap()),
            active: false,
        },
    )
    .unwrap()
    .id;
    // Stocked above its threshold, so it is nobody's problem.
    product(&mut conn, "Gravier", 3_000, 1_000, 0);

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    let named: Vec<i32> = read.low_stock.iter().map(|l| l.product_id).collect();
    assert_eq!(named, vec![short], "a retired product is not reordered");
    assert!(!named.contains(&retired));
    assert_eq!(read.low_stock[0].qty_on_hand_milli, 2_000);
    assert_eq!(read.low_stock[0].low_stock_at_milli, 10_000);
}

#[test]
fn the_top_lists_rank_by_units_and_by_margin_and_they_are_not_the_same_list() {
    let (_dir, mut conn) = open_temp();
    // Many units, almost nothing on each.
    let cheap = product(&mut conn, "Sable", 1_000, 900, 0);
    // Few units, a great deal on each.
    let rich = product(&mut conn, "Ciment", 50_000, 10_000, 0);
    sell(&mut conn, cheap, 20, SaleKind::Ticket, None, 9);
    sell(&mut conn, rich, 2, SaleKind::Ticket, None, 10);

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    assert_eq!(
        read.top_by_quantity
            .iter()
            .map(|p| p.product_id)
            .collect::<Vec<i32>>(),
        vec![cheap, rich]
    );
    assert_eq!(
        read.top_by_margin
            .iter()
            .map(|p| p.product_id)
            .collect::<Vec<i32>>(),
        vec![rich, cheap]
    );
    assert_eq!(read.top_by_quantity[0].qty_milli, 20_000);
    assert_eq!(read.top_by_margin[0].margin, Money::centimes(80_000));
    assert_eq!(read.top_by_margin[0].name, "Ciment");
}

#[test]
fn the_debts_are_the_parties_in_the_red_and_a_party_in_credit_is_not_netted_off() {
    let (_dir, mut conn) = open_temp();
    let owing = common::an_identified_customer(&mut conn, "Benali");
    let in_credit = common::a_customer(&mut conn, "Cherif");
    // One customer owes; the other is holding credit the shop owes back.
    common::a_payment_row(&mut conn, in_credit, 5_000);
    let p = product(&mut conn, "Ciment", 10_000, 6_000, 0);
    sell(&mut conn, p, 3, SaleKind::Facture, Some(owing), 9);

    let supplier = common::a_supplier(&mut conn, "Cimenterie");
    let purchase = common::a_purchase_row(&mut conn, supplier, "2026-09-10");
    common::a_purchase_ledger_row(&mut conn, supplier, purchase, 40_000);
    a_purchase(&mut conn, supplier, "ordered");
    a_purchase(&mut conn, supplier, "partially_received");
    a_purchase(&mut conn, supplier, "cancelled");

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    assert_eq!(read.customer_debt.total, Money::centimes(30_000));
    assert_eq!(
        read.customer_debt.parties, 1,
        "the customer in credit owes nothing and is not counted"
    );
    assert_eq!(read.supplier_debt.total, Money::centimes(40_000));
    assert_eq!(read.supplier_debt.parties, 1);
    assert_eq!(
        read.open_purchases, 2,
        "an order and a part delivery are open; a cancelled one and the received one are not"
    );
}

/// The one arm of the remise arithmetic the other tests never reach: a
/// credit note carries its own share of the remise the facture gave, and that
/// share has to come back off the period's remise or the reversal would give
/// back more than the facture charged.
///
/// Two units at 10,00 with 1,00 off the whole facture, the cost moved from
/// 6,00 to 7,00, then one unit credited. What is left is one unit sold: 10,00
/// of lines, 0,50 of remise, 9,50 of revenue, 6,00 of cost at the price it
/// left on, and 3,50 of margin.
#[test]
fn a_partial_avoir_gives_back_its_share_of_the_remise_and_no_more() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 1_000, 600, 0);
    let c = common::an_identified_customer(&mut conn, "Entreprise Benali");
    let facture = sales::issue(
        &mut conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![NewSaleLine {
                product_id: p,
                qty_milli: 2_000,
                unit_price: None,
                line_discount: Money::ZERO,
            }],
            global_discount: Money::centimes(100),
            payment_mode: PaymentMode::Credit,
            tendered: None,
            customer_id: Some(c),
            override_credit: false,
            kind: SaleKind::Facture,
            issued_at: Some(at(15, 9)),
        },
    )
    .unwrap()
    .document;
    move_the_cost(&mut conn, p, "Ciment", 1_000, 700);
    avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        Some(vec![avoir::AvoirLine {
            document_line_id: facture.lines[0].id,
            qty_milli: 1_000,
        }]),
        None,
        Some(at(15, 11)),
    )
    .unwrap();

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    assert_eq!(read.today.lines_ht, Money::centimes(1_000));
    assert_eq!(read.today.discounts, Money::centimes(50));
    assert_eq!(read.today.sales_ht, Money::centimes(950));
    assert_eq!(
        read.today.cost_of_goods,
        Money::centimes(600),
        "the unit that came back was priced at the delivery after it left"
    );
    assert_eq!(read.today.margin, Money::centimes(350));
}

/// A facture credited in part and then annulled takes its own credit note
/// with it, and leaves every other facture's alone. Reading only the
/// cancellation's own avoir out would leave the earlier one lowering a sale
/// the figures no longer carry.
#[test]
fn cancelling_a_facture_takes_the_avoirs_written_against_it_and_no_others() {
    let (_dir, mut conn) = open_temp();
    let p = product(&mut conn, "Ciment", 10_000, 6_000, 0);
    let c = common::an_identified_customer(&mut conn, "Entreprise Benali");

    // The one that goes: two units, one credited, then annulled.
    let doomed = sell(&mut conn, p, 2, SaleKind::Facture, Some(c), 9);
    avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        doomed.id,
        Some(vec![avoir::AvoirLine {
            document_line_id: doomed.lines[0].id,
            qty_milli: 1_000,
        }]),
        None,
        Some(at(15, 10)),
    )
    .unwrap();

    // The one that stays: two units, one credited, still standing.
    let standing = sell(&mut conn, p, 2, SaleKind::Facture, Some(c), 11);
    avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        standing.id,
        Some(vec![avoir::AvoirLine {
            document_line_id: standing.lines[0].id,
            qty_milli: 1_000,
        }]),
        None,
        Some(at(15, 12)),
    )
    .unwrap();

    cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        doomed.id,
        "erreur de saisie".to_string(),
        Some(at(15, 13)),
    )
    .unwrap();

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    // One facture left, one unit of it kept.
    assert_eq!(read.today.sales_count, 1);
    assert_eq!(read.today.sales_ht, Money::centimes(10_000));
    assert_eq!(read.today.cost_of_goods, Money::centimes(6_000));
    assert_eq!(read.today.margin, Money::centimes(4_000));
}

/// A product sold and credited back inside the month moved no units and
/// brought in nothing. It stays in the month's totals, where the two cancel,
/// and it is off the top lists: a row of zeros among the ten best reads as a
/// product that did something.
#[test]
fn a_product_whose_month_came_to_nothing_is_off_the_top_lists() {
    let (_dir, mut conn) = open_temp();
    let sold = product(&mut conn, "Ciment", 10_000, 6_000, 0);
    let returned = product(&mut conn, "Sable", 4_000, 1_000, 0);
    let c = common::an_identified_customer(&mut conn, "Entreprise Benali");
    sell(&mut conn, sold, 3, SaleKind::Ticket, None, 9);
    let credited = sell(&mut conn, returned, 2, SaleKind::Facture, Some(c), 10);
    avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        credited.id,
        None,
        None,
        Some(at(15, 11)),
    )
    .unwrap();

    let read = dashboard::read(&mut conn, SHOP, day(15)).unwrap();
    assert_eq!(
        read.top_by_quantity
            .iter()
            .map(|p| p.product_id)
            .collect::<Vec<i32>>(),
        vec![sold]
    );
    assert_eq!(
        read.top_by_margin
            .iter()
            .map(|p| p.product_id)
            .collect::<Vec<i32>>(),
        vec![sold]
    );
    // Still counted where it belongs: the facture was rung up and the credit
    // note took both sides of its margin back off.
    assert_eq!(read.today.sales_count, 2);
    assert_eq!(read.today.sales_ht, Money::centimes(30_000));
    assert_eq!(read.today.margin, Money::centimes(12_000));
}

// ---- the thirty day series (features.md §1, the dashboard's chart) ----

/// A sale on whatever day the case wants it on, which the `sell` above cannot
/// do: it rings everything up on the fifteenth.
fn sell_on(
    conn: &mut SqliteConnection,
    product_id: i32,
    units: i64,
    d: u32,
    hour: u32,
) -> Document {
    sales::issue(
        conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![NewSaleLine {
                product_id,
                qty_milli: units * 1_000,
                unit_price: None,
                line_discount: Money::ZERO,
            }],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(10_000_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: Some(at(d, hour)),
        },
    )
    .unwrap()
    .document
}

/// A month with something on most of its days: sales on nine of them, an
/// expense on two, a credit note and a cancellation, so the sum below is over
/// figures that go both ways.
fn a_month_of_trading(conn: &mut SqliteConnection) {
    let ciment = product(conn, "Ciment", 10_000, 6_000, 0);
    let sable = product(conn, "Sable", 4_000, 1_000, 0);
    let buyer = common::an_identified_customer(conn, "Entreprise Benali");
    for d in [1u32, 3, 8, 14, 15, 21, 22, 29, 30] {
        sell_on(conn, ciment, 2, d, 9);
        sell_on(conn, sable, 3, d, 14);
    }
    // A facture credited in part, on a later day than the one it was written
    // on: the two sides of the margin have to land on their own days.
    let credited = sales::issue(
        conn,
        SHOP,
        OWNER,
        NewSale {
            lines: vec![NewSaleLine {
                product_id: ciment,
                qty_milli: 4_000,
                unit_price: None,
                line_discount: Money::ZERO,
            }],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Credit,
            tendered: None,
            customer_id: Some(buyer),
            override_credit: true,
            kind: SaleKind::Facture,
            issued_at: Some(at(10, 10)),
        },
    )
    .unwrap()
    .document;
    avoir::issue(
        conn,
        SHOP,
        OWNER,
        credited.id,
        Some(vec![avoir::AvoirLine {
            document_line_id: credited.lines[0].id,
            qty_milli: 2_000,
        }]),
        None,
        Some(at(18, 11)),
    )
    .unwrap();
    // And a ticket that never happened, so a cancelled paper is inside the
    // window on both sides.
    let void = sell_on(conn, sable, 5, 6, 16);
    cancellation::cancel(
        conn,
        SHOP,
        OWNER,
        void.id,
        "erreur de saisie".to_string(),
        Some(at(7, 9)),
    )
    .unwrap();
    let category = dzpos_core::services::expenses::categories(conn, SHOP).unwrap()[0].id;
    for d in [2u32, 20] {
        dzpos_core::services::expenses::create(
            conn,
            SHOP,
            OWNER,
            dzpos_core::services::expenses::NewExpense {
                category_id: category,
                amount: Money::centimes(15_000),
                expense_date: day(d),
                note: None,
            },
        )
        .unwrap();
    }
}

/// The columns of a stretch of days, added up by the test itself.
fn fold(points: &[dashboard::SeriesPoint]) -> (i64, i64, i64, i64, i64, i64) {
    points.iter().fold((0, 0, 0, 0, 0, 0), |acc, p| {
        (
            acc.0 + p.figures.sales_ttc.as_centimes(),
            acc.1 + p.figures.sales_ht.as_centimes(),
            acc.2 + p.figures.cost_of_goods.as_centimes(),
            acc.3 + p.figures.margin.as_centimes(),
            acc.4 + p.figures.expenses.as_centimes(),
            acc.5 + p.cash_in.as_centimes(),
        )
    })
}

#[test]
fn the_thirty_days_of_the_series_add_up_to_the_month_the_dashboard_reads() {
    let (_dir, mut conn) = open_temp();
    a_month_of_trading(&mut conn);

    // September has thirty days, so a thirty day window ending on the last of
    // them is exactly the month the dashboard's second column reads.
    let series = dashboard::series(&mut conn, SHOP, day(30), 30).unwrap();
    let month = dashboard::read(&mut conn, SHOP, day(15))
        .unwrap()
        .this_month;

    assert_eq!(series.from, day(1));
    assert_eq!(series.to, day(30));
    assert_eq!(series.days.len(), 30);

    let (ttc, ht, cost, margin, expenses, _cash) = fold(&series.days);
    assert_eq!(ttc, month.sales_ttc.as_centimes());
    assert_eq!(ht, month.sales_ht.as_centimes());
    assert_eq!(cost, month.cost_of_goods.as_centimes());
    assert_eq!(margin, month.margin.as_centimes());
    assert_eq!(expenses, month.expenses.as_centimes());
    // A month that came to nothing would make the equality above vacuous.
    assert!(margin > 0);
}

#[test]
fn each_day_of_the_series_is_the_day_the_dashboard_reads_on_its_own() {
    let (_dir, mut conn) = open_temp();
    a_month_of_trading(&mut conn);

    let series = dashboard::series(&mut conn, SHOP, day(30), 30).unwrap();
    for point in &series.days {
        assert_eq!(point.from, point.to, "a day's bucket is one day wide");
        let one = dashboard::read(&mut conn, SHOP, point.from).unwrap();
        assert_eq!(point.figures, one.today, "{}", point.from);
        assert_eq!(
            point.cash_in,
            cash::position(&mut conn, SHOP, clock::Period::Day(point.from))
                .unwrap()
                .cash_in
                .total()
                .unwrap(),
            "{}",
            point.from
        );
    }
}

#[test]
fn the_weeks_are_the_days_taken_seven_at_a_time_back_from_the_last() {
    let (_dir, mut conn) = open_temp();
    a_month_of_trading(&mut conn);

    let series = dashboard::series(&mut conn, SHOP, day(30), 30).unwrap();
    // Thirty days is four whole weeks and the two days that do not fill one.
    // The short bucket is the oldest, because the chart is read from the
    // right: the week the shop is in has to be a whole one.
    assert_eq!(series.weeks.len(), 5);
    assert_eq!((series.weeks[0].from, series.weeks[0].to), (day(1), day(2)));
    assert_eq!((series.weeks[1].from, series.weeks[1].to), (day(3), day(9)));
    assert_eq!(
        (series.weeks[4].from, series.weeks[4].to),
        (day(24), day(30))
    );

    // And each week is its own days, added up.
    let mut start = 0usize;
    for week in &series.weeks {
        let span = usize::try_from((week.to - week.from).num_days() + 1).unwrap();
        let mine = &series.days[start..start + span];
        assert_eq!(
            fold(mine),
            fold(std::slice::from_ref(week)),
            "{}",
            week.from
        );
        start += span;
    }
    assert_eq!(start, series.days.len());
}

#[test]
fn a_window_of_no_days_and_one_longer_than_a_year_are_both_refused() {
    let (_dir, mut conn) = open_temp();
    assert!(dashboard::series(&mut conn, SHOP, day(30), 0).is_err());
    assert!(dashboard::series(&mut conn, SHOP, day(30), 367).is_err());
    // The bounds themselves answer.
    assert!(dashboard::series(&mut conn, SHOP, day(30), 1).is_ok());
    assert!(dashboard::series(&mut conn, SHOP, day(30), 366).is_ok());
}

#[test]
fn the_series_walks_through_a_year_end_and_a_leap_day_without_a_gap() {
    let (_dir, mut conn) = open_temp();
    // Every fixture above keeps the window inside September; a rollover
    // bug at a month or year end would pass them all. Ten days ending on
    // 5 January reach back into December; nine ending on 2 March 2028
    // cross the leap day.
    for (last, days, first, must_hold) in [
        (
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
            10,
            NaiveDate::from_ymd_opt(2025, 12, 27).unwrap(),
            [
                NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            ],
        ),
        (
            NaiveDate::from_ymd_opt(2028, 3, 2).unwrap(),
            9,
            NaiveDate::from_ymd_opt(2028, 2, 23).unwrap(),
            [
                NaiveDate::from_ymd_opt(2028, 2, 29).unwrap(),
                NaiveDate::from_ymd_opt(2028, 3, 1).unwrap(),
            ],
        ),
    ] {
        let series = dashboard::series(&mut conn, SHOP, last, days).unwrap();
        assert_eq!(series.from, first, "{last}");
        assert_eq!(series.to, last, "{last}");
        assert_eq!(series.days.len(), days as usize, "{last}");
        for pair in series.days.windows(2) {
            assert_eq!(
                pair[1].from,
                pair[0].from.succ_opt().unwrap(),
                "consecutive days around {}",
                pair[0].from
            );
        }
        for d in must_hold {
            assert!(
                series.days.iter().any(|p| p.from == d),
                "{d} is in the window"
            );
        }
    }
}
