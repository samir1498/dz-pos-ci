// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The three sums a purchase has to keep true, over orders and deliveries
//! nobody wrote by hand.
//!
//! One: the landed line totals never come out above what the order cost, and
//! what the per-unit division loses is bounded by the quantities themselves.
//! Two: what the shop owes the supplier is the value that arrived at landed
//! cost, less what it paid and less what it sent back. Three: the stock of
//! each product is what arrived less what went back.
//!
//! Against a real temp SQLite file, one per case: the invariants are about
//! what the writes leave in the file, and a generator feeding a mock would
//! prove the mock.

use diesel::sqlite::SqliteConnection;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::models::purchase::PurchaseLine as PurchaseLineDto;
use dzpos_core::money::{Bps, Money};
use dzpos_core::services::purchases::{
    self, NewLine, NewPurchase, Paid, PurchaseStatus, PurchaseView, ReceiveLine,
};
use dzpos_core::services::supplier_debt::{self, PaymentMethod};
use dzpos_core::services::{products, suppliers};
use proptest::prelude::*;

const SHOP: i32 = 1;
const OWNER: i32 = 1;

/// What a shop does to an order after it has been written.
#[derive(Debug, Clone, Copy)]
enum Step {
    /// A delivery taking this many thousandths off every line that still has
    /// something outstanding, capped at what is outstanding.
    Receive(i64),
    /// A return of this many thousandths of every line that has something to
    /// send back, capped at what arrived and has not gone back yet.
    Return(i64),
    /// Money handed to the supplier. Refused when it is more than the whole
    /// balance, which is one of the things worth generating.
    Pay(i64),
}

fn steps() -> impl Strategy<Value = Vec<Step>> {
    let step = prop_oneof![
        (1i64..=9_000).prop_map(Step::Receive),
        (1i64..=4_000).prop_map(Step::Return),
        (1i64..=200_000).prop_map(Step::Pay),
    ];
    prop::collection::vec(step, 1..=8)
}

/// One to four lines: a quantity in thousandths and a unit cost in centimes.
/// The unit cost is never nothing, so the extra costs always have value to
/// sit on (a purchase whose lines are worth nothing is refused, by design).
fn an_order() -> impl Strategy<Value = (Vec<(i64, i64)>, i64, i64)> {
    (
        prop::collection::vec((500i64..=25_000, 1i64..=50_000), 1..=4),
        0i64..=90_000,
        0i64..=9_000,
    )
}

fn a_product(conn: &mut SqliteConnection, n: usize) -> i32 {
    products::create(
        conn,
        SHOP,
        OWNER,
        NewProduct {
            name: format!("Article {n}"),
            barcode: None,
            category_id: None,
            unit: Unit::Piece,
            cost: Money::centimes(1_000),
            selling: Money::centimes(2_000),
            wholesale: None,
            qty_on_hand_milli: 0,
            low_stock_at_milli: 0,
            rate_bps: Some(Bps::new(1900).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id
}

fn a_supplier(conn: &mut SqliteConnection) -> i32 {
    suppliers::create(
        conn,
        SHOP,
        OWNER,
        suppliers::NewSupplier {
            name: "Sarl Amrani".to_string(),
            phone: None,
            address: None,
            rc: None,
            nif: None,
            nis: None,
            ai: None,
            notes: None,
            active: true,
        },
        None,
    )
    .unwrap()
    .id
}

fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let conn = dzpos_core::db::open(&path).unwrap();
    (dir, conn)
}

/// What one delivery or one return is worth, stated here the way the rule
/// reads rather than the way the service computes it: a movement is worth
/// what the line is worth once it has moved, less what it was worth before.
///
/// Never `checked_mul_milli` on the movement's own quantity. That is the
/// half-up rounding the service used to do per paper, and two halves of a
/// line each rounded up come to a centime more than the line is worth; a test
/// that repeated it would agree with the bug.
fn value_of(view: &PurchaseView, moved: &[ReceiveLine], before: Which) -> Money {
    let mut total = Money::ZERO;
    for line in moved {
        let Some(ordered) = view.lines.iter().find(|l| l.id == line.purchase_line_id) else {
            continue;
        };
        let start = match before {
            Which::Received => ordered.qty_received_milli,
            Which::Returned => ordered.qty_returned_milli,
        };
        let step = running_value(ordered, start + line.qty_milli)
            .checked_sub(running_value(ordered, start))
            .unwrap();
        total = total.checked_add(step).unwrap();
    }
    total
}

/// Which running total a movement is measured against.
#[derive(Debug, Clone, Copy)]
enum Which {
    Received,
    Returned,
}

/// What the first `qty_milli` of a line are worth: rounded down, except at the
/// whole ordered quantity, where it is the line's landed total.
fn running_value(line: &PurchaseLineDto, qty_milli: i64) -> Money {
    if qty_milli >= line.qty_ordered_milli {
        return line
            .landed_unit_cost
            .checked_mul_milli(line.qty_ordered_milli)
            .unwrap();
    }
    let raw = i128::from(line.landed_unit_cost.as_centimes()) * i128::from(qty_milli) / 1_000;
    Money::centimes(i64::try_from(raw).unwrap())
}

/// Every `purchase` debit and every `return` credit the ledger carries for one
/// order, read back out of the file. The invariant below is stated against
/// these rows and not against any second arithmetic.
fn rows_of(conn: &mut SqliteConnection, supplier_id: i32, purchase_id: i32) -> (Money, Money) {
    let mut debits = Money::ZERO;
    let mut credits = Money::ZERO;
    for entry in supplier_debt::ledger(conn, SHOP, supplier_id).unwrap() {
        if entry.purchase_id != Some(purchase_id) {
            continue;
        }
        debits = debits.checked_add(entry.debit).unwrap();
        credits = credits.checked_add(entry.credit).unwrap();
    }
    (debits, credits)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

    /// The extra costs are spread by value and then divided per unit, and the
    /// division rounds down. So the landed line totals never come out above
    /// the lines' value plus the extra costs, and what they fall short by is
    /// bounded: each line loses less than one unit's worth of its own share,
    /// which is at most `floor(qty in units) + 1` centimes.
    #[test]
    fn the_landed_totals_never_rise_above_what_the_order_cost((lines, transport, extra) in an_order()) {
        let (_dir, mut conn) = open_temp();
        let supplier = a_supplier(&mut conn);
        let new_lines: Vec<NewLine> = lines
            .iter()
            .enumerate()
            .map(|(n, (qty, cost))| NewLine {
                product_id: a_product(&mut conn, n),
                qty_ordered_milli: *qty,
                unit_cost: Money::centimes(*cost),
            })
            .collect();
        let saved = purchases::save(
            &mut conn,
            SHOP,
            OWNER,
            NewPurchase {
                supplier_id: supplier,
                supplier_document_number: None,
                purchase_date: "2026-09-10".to_string(),
                due_date: None,
                transport: Money::centimes(transport),
                extra_costs: Money::centimes(extra),
                note: None,
                lines: new_lines,
                paid_now: None,
                receive_now: false,
            },
        )
        .unwrap();

        let mut plain = Money::ZERO;
        let mut landed = Money::ZERO;
        let mut slack = 0i64;
        for line in &saved.lines {
            plain = plain
                .checked_add(line.unit_cost.checked_mul_milli(line.qty_ordered_milli).unwrap())
                .unwrap();
            landed = landed
                .checked_add(
                    line.landed_unit_cost
                        .checked_mul_milli(line.qty_ordered_milli)
                        .unwrap(),
                )
                .unwrap();
            // Less than one unit's worth per line, and the half-up rounding
            // of the line total can cost one centime more.
            slack += line.qty_ordered_milli / 1_000 + 1;
        }
        let ceiling = plain
            .checked_add(Money::centimes(transport + extra))
            .unwrap();
        prop_assert!(landed <= ceiling, "landed {landed:?} above {ceiling:?}");
        let short = ceiling.checked_sub(landed).unwrap().as_centimes();
        prop_assert!(short <= slack, "short by {short} against a bound of {slack}");
    }

    /// Whatever order deliveries, returns and payments arrive in: what the
    /// shop owes is the value on its shelf at landed cost less what it has
    /// paid, and each product's stock is what arrived less what went back.
    #[test]
    fn the_debt_and_the_stock_follow_the_goods((lines, transport, extra) in an_order(), steps in steps()) {
        let (_dir, mut conn) = open_temp();
        let supplier = a_supplier(&mut conn);
        let new_lines: Vec<NewLine> = lines
            .iter()
            .enumerate()
            .map(|(n, (qty, cost))| NewLine {
                product_id: a_product(&mut conn, n),
                qty_ordered_milli: *qty,
                unit_cost: Money::centimes(*cost),
            })
            .collect();
        let saved = purchases::save(
            &mut conn,
            SHOP,
            OWNER,
            NewPurchase {
                supplier_id: supplier,
                supplier_document_number: None,
                purchase_date: "2026-09-10".to_string(),
                due_date: None,
                transport: Money::centimes(transport),
                extra_costs: Money::centimes(extra),
                note: None,
                lines: new_lines,
                paid_now: None,
                receive_now: false,
            },
        )
        .unwrap();
        let purchase_id = saved.purchase.id;
        // The three sums the ledger's balance has to come out as, kept here
        // paper by paper: what arrived, what went back, what was handed over.
        let mut arrived = Money::ZERO;
        let mut sent_back = Money::ZERO;
        let mut paid = Money::ZERO;

        for step in steps {
            let view = purchases::get(&mut conn, SHOP, purchase_id).unwrap();
            match step {
                Step::Receive(by) => {
                    let taking: Vec<ReceiveLine> = view
                        .lines
                        .iter()
                        .filter_map(|l| {
                            let left = l.qty_ordered_milli - l.qty_received_milli;
                            (left > 0).then_some(ReceiveLine {
                                purchase_line_id: l.id,
                                qty_milli: by.min(left),
                            })
                        })
                        .collect();
                    if taking.is_empty() || view.purchase.status == PurchaseStatus::Received {
                        continue;
                    }
                    arrived = arrived
                        .checked_add(value_of(&view, &taking, Which::Received))
                        .unwrap();
                    purchases::receive(&mut conn, SHOP, OWNER, purchase_id, taking, None).unwrap();
                }
                Step::Return(by) => {
                    let back: Vec<ReceiveLine> = view
                        .lines
                        .iter()
                        .filter_map(|l| {
                            let left = l.qty_received_milli - l.qty_returned_milli;
                            (left > 0).then_some(ReceiveLine {
                                purchase_line_id: l.id,
                                qty_milli: by.min(left),
                            })
                        })
                        .collect();
                    if back.is_empty() {
                        continue;
                    }
                    sent_back = sent_back
                        .checked_add(value_of(&view, &back, Which::Returned))
                        .unwrap();
                    purchases::return_to_supplier(&mut conn, SHOP, OWNER, purchase_id, back, None)
                        .unwrap();
                }
                Step::Pay(amount) => {
                    let amount = Money::centimes(amount);
                    // What the shop owed the instant before this payment was
                    // tried, by the same three-sum formula the loop checks
                    // against after every step: the only refusal `pay` is
                    // allowed here is a payment above that.
                    let owed_before = arrived
                        .checked_sub(sent_back)
                        .unwrap()
                        .checked_sub(paid)
                        .unwrap();
                    let at = dzpos_core::services::clock::now();
                    match supplier_debt::pay(
                        &mut conn,
                        SHOP,
                        OWNER,
                        supplier,
                        amount,
                        PaymentMethod::Cash,
                        None,
                        at,
                    ) {
                        Ok(_) => paid = paid.checked_add(amount).unwrap(),
                        // More than the shop owes is refused, and nothing of
                        // it landed; any other refusal here would be a bug
                        // this property is exactly placed to catch.
                        Err(_) => {
                            prop_assert!(
                                amount > owed_before,
                                "a payment of {amount:?} against a debt of {owed_before:?} was refused"
                            );
                            continue;
                        }
                    }
                }
            }

            let view = purchases::get(&mut conn, SHOP, purchase_id).unwrap();
            // What the shop owes: what arrived at landed cost, less what went
            // back and less every dinar handed over. Nothing else touches this
            // supplier's ledger in this test.
            let owed = arrived
                .checked_sub(sent_back)
                .unwrap()
                .checked_sub(paid)
                .unwrap();
            prop_assert_eq!(
                supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
                owed
            );
            // Stated against the rows the file carries and against nothing
            // this test computed: what the order has been debited never rises
            // above what its lines are worth, and reaches it exactly once
            // every line has arrived. That is the invariant the half-up
            // rounding per delivery broke, and the one a full prepayment
            // depends on.
            let (debits, credits) = rows_of(&mut conn, supplier, purchase_id);
            let mut worth = Money::ZERO;
            let mut whole = true;
            let mut all_back = true;
            for line in &view.lines {
                worth = worth
                    .checked_add(
                        line.landed_unit_cost
                            .checked_mul_milli(line.qty_ordered_milli)
                            .unwrap(),
                    )
                    .unwrap();
                whole &= line.qty_received_milli == line.qty_ordered_milli;
                all_back &= line.qty_returned_milli == line.qty_received_milli;
            }
            prop_assert!(debits <= worth, "debited {debits:?} of {worth:?}");
            if whole {
                prop_assert_eq!(debits, worth);
            }
            // And back the other way: a return never credits more than the
            // order was debited, and everything back is everything credited.
            prop_assert!(credits <= debits, "credited {credits:?} of {debits:?}");
            if all_back {
                prop_assert_eq!(credits, debits);
            }

            // And the shelf itself: what arrived less what went back.
            for line in &view.lines {
                let on_hand = products::get(&mut conn, SHOP, line.product_id)
                    .unwrap()
                    .qty_on_hand_milli;
                prop_assert_eq!(
                    on_hand,
                    line.qty_received_milli - line.qty_returned_milli
                );
            }
        }
    }

    /// Money handed over when the order is saved is bounded by the order and
    /// lands on it: whatever the goods do afterwards, the shop never ends up
    /// owing more than the value on its shelf less what it paid.
    #[test]
    fn money_handed_over_when_the_order_is_written_lands_on_it(amount in 1i64..=200_000) {
        let (_dir, mut conn) = open_temp();
        let supplier = a_supplier(&mut conn);
        let product = a_product(&mut conn, 0);
        let order = NewPurchase {
            supplier_id: supplier,
            supplier_document_number: None,
            purchase_date: "2026-09-10".to_string(),
            due_date: None,
            transport: Money::ZERO,
            extra_costs: Money::ZERO,
            note: None,
            lines: vec![NewLine {
                product_id: product,
                qty_ordered_milli: 10_000,
                unit_cost: Money::centimes(20_000),
            }],
            paid_now: Some(Paid {
                amount: Money::centimes(amount),
                mode: PaymentMethod::Cash,
            }),
            receive_now: false,
        };
        // The order is worth 200 000 centimes, so anything above it is
        // refused and nothing at all is written.
        let saved = match purchases::save(&mut conn, SHOP, OWNER, order) {
            Ok(saved) => saved,
            Err(_) => {
                prop_assert!(amount > 200_000);
                return Ok(());
            }
        };
        let all: Vec<ReceiveLine> = saved
            .lines
            .iter()
            .map(|l| ReceiveLine {
                purchase_line_id: l.id,
                qty_milli: l.qty_ordered_milli,
            })
            .collect();
        let value = value_of(&saved, &all, Which::Received);
        purchases::receive(&mut conn, SHOP, OWNER, saved.purchase.id, all, None).unwrap();
        prop_assert_eq!(
            supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
            value.checked_sub(Money::centimes(amount)).unwrap()
        );
        // And the money is on this order's paper, not sitting on the balance.
        let placed: i64 = supplier_debt::allocations(&mut conn, SHOP, saved.purchase.id)
            .unwrap()
            .iter()
            .map(|a| a.amount.as_centimes())
            .sum();
        prop_assert_eq!(placed, amount);
    }
}
