// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! What the shop ordered, what arrived, what it cost to get here and what it
//! left on the supplier's account (features.md §1, Purchase).
//!
//! The two rules everything else hangs off: the landed unit cost is fixed
//! when the order is saved, and stock and debt move when the goods arrive and
//! never when the paper is written.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::money::{Bps, Money};
use dzpos_core::services::purchases::{
    self, MovementKind, NewLine, NewPurchase, Paid, PurchaseStatus, ReceiveLine,
};
use dzpos_core::services::supplier_debt::{self, PaymentMethod, SupplierDebtKind};
use dzpos_core::services::{audit, products, stock};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

mod common;

use common::{a_supplier, open_temp};

/// A product with no stock and a cost of its own, so a test can say what the
/// first receipt did to that cost.
fn a_product(conn: &mut SqliteConnection, name: &str, cost: i64) -> i32 {
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
            selling: Money::centimes(cost * 2),
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

fn an_order(supplier_id: i32, lines: Vec<NewLine>) -> NewPurchase {
    NewPurchase {
        supplier_id,
        supplier_document_number: None,
        purchase_date: "2026-09-10".to_string(),
        due_date: None,
        transport: Money::ZERO,
        extra_costs: Money::ZERO,
        note: None,
        lines,
        paid_now: None,
        receive_now: false,
    }
}

fn line(product_id: i32, qty_milli: i64, unit_cost: i64) -> NewLine {
    NewLine {
        product_id,
        qty_ordered_milli: qty_milli,
        unit_cost: Money::centimes(unit_cost),
    }
}

/// Every line of the order, taken whole.
fn all_of(view: &purchases::PurchaseView) -> Vec<ReceiveLine> {
    view.lines
        .iter()
        .map(|l| ReceiveLine {
            purchase_line_id: l.id,
            qty_milli: l.qty_ordered_milli - l.qty_received_milli,
        })
        .collect()
}

fn on_hand(conn: &mut SqliteConnection, product_id: i32) -> i64 {
    products::get(conn, SHOP, product_id)
        .unwrap()
        .qty_on_hand_milli
}

#[test]
fn a_purchase_saved_without_receiving_moves_no_stock_and_owes_nothing() {
    // Debt rises on receipt and never on save (plan lens, 2026-09-10): an
    // order nothing has arrived against is a piece of paper, and a shop does
    // not owe for goods it has not been handed.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 20_000);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 10_000, 20_000)]),
    )
    .unwrap();
    assert_eq!(saved.purchase.status, PurchaseStatus::Ordered);
    assert_eq!(on_hand(&mut conn, farine), 0);
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::ZERO
    );
    assert!(saved.receipts.is_empty());
    // The cost on the product is the one it was created with: nothing landed.
    assert_eq!(
        products::get(&mut conn, SHOP, farine).unwrap().cost,
        Money::centimes(20_000)
    );
}

#[test]
fn a_purchase_received_at_once_moves_the_stock_and_what_the_shop_owes() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 20_000);
    let sucre = a_product(&mut conn, "Sucre 1kg", 9_000);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            receive_now: true,
            ..an_order(
                supplier,
                vec![line(farine, 10_000, 20_000), line(sucre, 20_000, 9_000)],
            )
        },
    )
    .unwrap();
    assert_eq!(saved.purchase.status, PurchaseStatus::Received);
    assert_eq!(on_hand(&mut conn, farine), 10_000);
    assert_eq!(on_hand(&mut conn, sucre), 20_000);
    // No extra costs, so the landed cost is the unit cost and the debt is the
    // value of the two lines.
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(10 * 20_000 + 20 * 9_000)
    );
    assert_eq!(saved.receipts.len(), 1);
    assert_eq!(saved.receipts[0].lines.len(), 2);
    assert_eq!(saved.receipts[0].receipt.series, "reception:2026");
    assert_eq!(saved.receipts[0].receipt.number, 1);
    // The movements name the purchase kind and carry the landed cost.
    let moves = stock::list_for_product(&mut conn, SHOP, farine).unwrap();
    let last = moves.last().unwrap();
    assert_eq!(last.kind, MovementKind::Purchase);
    assert_eq!(last.qty_milli, 10_000);
    assert_eq!(last.unit_cost, Money::centimes(20_000));
    assert_eq!(last.document_id, None);
}

#[test]
fn the_extra_costs_are_spread_over_the_lines_by_value_and_the_last_takes_the_remainder() {
    // Two lines worth 200 000 and 100 000 centimes, 30 001 centimes of
    // transport and extra costs between them: two thirds and one third, and
    // the odd centime the division leaves goes to the last line.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let sucre = a_product(&mut conn, "Sucre 1kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            transport: Money::centimes(30_000),
            extra_costs: Money::centimes(1),
            ..an_order(
                supplier,
                // 10 units at 20 000 and 10 units at 10 000.
                vec![line(farine, 10_000, 20_000), line(sucre, 10_000, 10_000)],
            )
        },
    )
    .unwrap();
    // 30 001 × 200 000 / 300 000 = 20 000 (floored); the rest, 10 001, is the
    // last line's.
    assert_eq!(
        saved.lines[0].landed_unit_cost,
        // 20 000 + floor(20 000 / 10)
        Money::centimes(20_000 + 2_000)
    );
    assert_eq!(
        saved.lines[1].landed_unit_cost,
        // 10 000 + floor(10 001 / 10)
        Money::centimes(10_000 + 1_000)
    );
}

#[test]
fn a_landed_line_total_never_rises_above_what_the_order_cost() {
    // The per-unit division rounds down, so the landed line totals can land
    // under the lines' value plus the extra costs, never above it. One line
    // of 100 units and 999 centimes to spread is the worst case of the shape:
    // 9 centimes a unit lands, 99 are lost, one less than the quantity.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            extra_costs: Money::centimes(999),
            ..an_order(supplier, vec![line(farine, 100_000, 100)])
        },
    )
    .unwrap();
    assert_eq!(saved.lines[0].landed_unit_cost, Money::centimes(109));
    let landed = saved.lines[0]
        .landed_unit_cost
        .checked_mul_milli(100_000)
        .unwrap();
    assert_eq!(landed, Money::centimes(10_900));
    assert_eq!(
        Money::centimes(10_000 + 999).checked_sub(landed).unwrap(),
        Money::centimes(99)
    );
}

#[test]
fn extra_costs_with_no_value_to_sit_on_are_refused() {
    // Every line free of charge and money to spread over them: the share of a
    // line worth nothing is nothing, and there is no honest place to put the
    // amount. Refused on the field rather than spread by quantity, which
    // would divide kilos and pieces by each other.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let err = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            transport: Money::centimes(5_000),
            ..an_order(supplier, vec![line(farine, 10_000, 0)])
        },
    )
    .unwrap_err();
    match err {
        dzpos_core::error::CoreError::Validation { ref field, .. } => {
            assert_eq!(field, "transport_centimes");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn two_receipts_fill_the_order_and_the_second_one_closes_it() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 10_000, 20_000)]),
    )
    .unwrap();
    let first = purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 4_000,
        }],
        None,
    )
    .unwrap();
    assert_eq!(first.purchase.status, PurchaseStatus::PartiallyReceived);
    assert_eq!(on_hand(&mut conn, farine), 4_000);
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(4 * 20_000)
    );
    let second = purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 6_000,
        }],
        None,
    )
    .unwrap();
    assert_eq!(second.purchase.status, PurchaseStatus::Received);
    assert_eq!(on_hand(&mut conn, farine), 10_000);
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(10 * 20_000)
    );
    // Two bons de réception, each with its own number out of the year's
    // series.
    assert_eq!(second.receipts.len(), 2);
    let numbers: Vec<i64> = second.receipts.iter().map(|r| r.receipt.number).collect();
    assert_eq!(numbers, vec![2, 1]);
}

#[test]
fn a_receipt_of_more_than_is_still_outstanding_is_refused() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 10_000, 20_000)]),
    )
    .unwrap();
    let err = purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 10_001,
        }],
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), "validation", "{err}");
    // And nothing of it landed: no stock, no debt, no number burnt.
    assert_eq!(on_hand(&mut conn, farine), 0);
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::ZERO
    );
    let after = purchases::get(&mut conn, SHOP, saved.purchase.id).unwrap();
    assert!(after.receipts.is_empty());
}

#[test]
fn the_product_cost_follows_the_landed_cost_of_the_last_receipt() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 20_000);
    purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            transport: Money::centimes(10_000),
            receive_now: true,
            ..an_order(supplier, vec![line(farine, 10_000, 20_000)])
        },
    )
    .unwrap();
    // 10 000 centimes of transport over ten units is 1 000 a unit.
    assert_eq!(
        products::get(&mut conn, SHOP, farine).unwrap().cost,
        Money::centimes(21_000)
    );
}

#[test]
fn a_receipt_after_the_purchase_was_fully_paid_lands_the_credit_on_it() {
    // The awkward case the plan names: the shop pays the whole order the day
    // it is placed and the goods come a week later. The payment has no order
    // to settle when it is written, so it sits on the balance as credit; the
    // receipt's `purchase` row is what it lands on, and the order is not left
    // asking to be paid a second time.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            paid_now: Some(Paid {
                amount: Money::centimes(200_000),
                mode: PaymentMethod::Cash,
            }),
            ..an_order(supplier, vec![line(farine, 10_000, 20_000)])
        },
    )
    .unwrap();
    // Money out, nothing owed yet: the supplier is holding an advance.
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(-200_000)
    );
    assert!(supplier_debt::open_purchases(&mut conn, SHOP, supplier)
        .unwrap()
        .is_empty());
    purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        all_of(&saved),
        None,
    )
    .unwrap();
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::ZERO
    );
    // The order is settled, so nothing asks for it again.
    assert!(supplier_debt::open_purchases(&mut conn, SHOP, supplier)
        .unwrap()
        .is_empty());
    assert_eq!(
        supplier_debt::allocations(&mut conn, SHOP, saved.purchase.id)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn paid_now_with_the_goods_settles_the_order_it_was_saved_with() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            receive_now: true,
            paid_now: Some(Paid {
                amount: Money::centimes(50_000),
                mode: PaymentMethod::Card,
            }),
            ..an_order(supplier, vec![line(farine, 10_000, 20_000)])
        },
    )
    .unwrap();
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(150_000)
    );
    let open = supplier_debt::open_purchases(&mut conn, SHOP, supplier).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].purchase_id, saved.purchase.id);
    assert_eq!(open[0].remaining, Money::centimes(150_000));
    // The payment carries the mode the file insists a payment carries.
    let ledger = supplier_debt::ledger(&mut conn, SHOP, supplier).unwrap();
    let payment = ledger
        .iter()
        .find(|e| e.kind == SupplierDebtKind::Payment)
        .unwrap();
    assert_eq!(payment.payment_mode, Some(PaymentMethod::Card));
}

#[test]
fn paying_more_than_the_order_is_worth_is_refused_when_it_is_saved() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let err = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            paid_now: Some(Paid {
                amount: Money::centimes(200_001),
                mode: PaymentMethod::Cash,
            }),
            ..an_order(supplier, vec![line(farine, 10_000, 20_000)])
        },
    )
    .unwrap_err();
    match err {
        dzpos_core::error::CoreError::Validation { ref field, .. } => {
            assert_eq!(field, "paid_now_centimes");
        }
        other => panic!("{other:?}"),
    }
    // Nothing was written on the way past.
    assert!(purchases::list(&mut conn, SHOP, None, None)
        .unwrap()
        .is_empty());
}

#[test]
fn a_purchase_is_cancelled_only_while_nothing_has_arrived() {
    // The second awkward case the plan names. A cancellation says the order
    // never happened, and goods on the shelf say it did; what closes an order
    // the rest of which will never come is `close_short`.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 10_000, 20_000)]),
    )
    .unwrap();
    purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 4_000,
        }],
        None,
    )
    .unwrap();
    let err = purchases::cancel(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        "le fournisseur ne livre plus".to_string(),
    )
    .unwrap_err();
    match err {
        dzpos_core::error::CoreError::Validation { ref field, .. } => assert_eq!(field, "status"),
        other => panic!("{other:?}"),
    }
    // Closed short instead: what arrived stays, what never came is written off.
    let short = purchases::close_short(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        "le fournisseur ne livre plus".to_string(),
    )
    .unwrap();
    assert_eq!(short.purchase.status, PurchaseStatus::ClosedShort);
    assert_eq!(on_hand(&mut conn, farine), 4_000);
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(4 * 20_000)
    );
}

#[test]
fn a_purchase_nothing_arrived_against_is_cancelled_and_takes_no_more_receipts() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 10_000, 20_000)]),
    )
    .unwrap();
    let after = purchases::cancel(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        "commande passée deux fois".to_string(),
    )
    .unwrap();
    assert_eq!(after.purchase.status, PurchaseStatus::Cancelled);
    assert!(purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        all_of(&saved),
        None,
    )
    .is_err());
    // A close short needs something to have arrived, so it is refused too.
    assert!(
        purchases::close_short(&mut conn, SHOP, OWNER, saved.purchase.id, "x".to_string()).is_err()
    );
}

#[test]
fn a_return_takes_the_stock_back_out_and_lowers_what_the_shop_owes() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            receive_now: true,
            ..an_order(supplier, vec![line(farine, 10_000, 20_000)])
        },
    )
    .unwrap();
    let after = purchases::return_to_supplier(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 3_000,
        }],
        Some("trois sacs éventrés".to_string()),
    )
    .unwrap();
    assert_eq!(after.lines[0].qty_returned_milli, 3_000);
    assert_eq!(on_hand(&mut conn, farine), 7_000);
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(7 * 20_000)
    );
    let ledger = supplier_debt::ledger(&mut conn, SHOP, supplier).unwrap();
    let credit = ledger
        .iter()
        .find(|e| e.kind == SupplierDebtKind::Return)
        .unwrap();
    assert_eq!(credit.credit, Money::centimes(3 * 20_000));
    assert_eq!(credit.purchase_id, Some(saved.purchase.id));
    // The movement leaves the shelf: a return to a supplier is stock going
    // out, and the kind is the same word the customer side uses inbound.
    let moves = stock::list_for_product(&mut conn, SHOP, farine).unwrap();
    assert_eq!(moves.last().unwrap().qty_milli, -3_000);
    assert_eq!(moves.last().unwrap().kind, MovementKind::Return);
}

#[test]
fn a_return_of_an_order_already_paid_leaves_credit_with_the_supplier() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            receive_now: true,
            paid_now: Some(Paid {
                amount: Money::centimes(200_000),
                mode: PaymentMethod::Cash,
            }),
            ..an_order(supplier, vec![line(farine, 10_000, 20_000)])
        },
    )
    .unwrap();
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::ZERO
    );
    purchases::return_to_supplier(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 2_000,
        }],
        None,
    )
    .unwrap();
    // The debt was already settled, so the credit note takes the balance
    // below zero: money the supplier is holding for the shop.
    let balance = supplier_debt::balance(&mut conn, SHOP, supplier).unwrap();
    assert_eq!(balance, Money::centimes(-40_000));
    assert_eq!(
        supplier_debt::credit_held(balance).unwrap(),
        Money::centimes(40_000)
    );
}

#[test]
fn a_return_of_more_than_arrived_is_refused() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 10_000, 20_000)]),
    )
    .unwrap();
    purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 4_000,
        }],
        None,
    )
    .unwrap();
    let err = purchases::return_to_supplier(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 4_001,
        }],
        None,
    )
    .unwrap_err();
    assert_eq!(err.code(), "validation", "{err}");
    assert_eq!(on_hand(&mut conn, farine), 4_000);
}

#[test]
fn a_closed_supplier_takes_no_order_and_no_receipt() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 10_000, 20_000)]),
    )
    .unwrap();
    dzpos_core::services::suppliers::close(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Some("plus de livraisons".to_string()),
    )
    .unwrap();
    let err = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 1_000, 20_000)]),
    )
    .unwrap_err();
    match err {
        dzpos_core::error::CoreError::Validation { ref field, .. } => {
            assert_eq!(field, "supplier_id");
        }
        other => panic!("{other:?}"),
    }
    assert!(purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        all_of(&saved),
        None
    )
    .is_err());
}

#[test]
fn an_order_takes_a_product_of_its_own_shop_only_and_names_it_once() {
    let (_dir, mut conn) = open_temp();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut conn)
        .unwrap();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    // Another shop's supplier is not this shop's to buy from.
    assert!(purchases::save(
        &mut conn,
        2,
        OWNER,
        an_order(supplier, vec![line(farine, 1_000, 20_000)])
    )
    .is_err());
    // A product this shop does not have.
    let err = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(9_999, 1_000, 20_000)]),
    )
    .unwrap_err();
    assert_eq!(err.code(), "not_found", "{err}");
    // The same product twice would leave "the cost of the last receipt"
    // asking which of two lines it meant.
    let err = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(
            supplier,
            vec![line(farine, 1_000, 20_000), line(farine, 2_000, 19_000)],
        ),
    )
    .unwrap_err();
    match err {
        dzpos_core::error::CoreError::Validation { ref field, .. } => {
            assert_eq!(field, "product_id");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_order_with_no_line_and_a_line_of_nothing_are_both_refused() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    assert!(purchases::save(&mut conn, SHOP, OWNER, an_order(supplier, vec![])).is_err());
    assert!(purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 0, 20_000)])
    )
    .is_err());
    assert!(purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 1_000, -1)])
    )
    .is_err());
    assert!(purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            transport: Money::centimes(-1),
            ..an_order(supplier, vec![line(farine, 1_000, 20_000)])
        }
    )
    .is_err());
    assert!(purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            purchase_date: "10/09/2026".to_string(),
            ..an_order(supplier, vec![line(farine, 1_000, 20_000)])
        }
    )
    .is_err());
}

#[test]
fn the_list_reads_by_status_and_by_supplier() {
    let (_dir, mut conn) = open_temp();
    let amrani = a_supplier(&mut conn, "Sarl Amrani");
    let bouzid = a_supplier(&mut conn, "Sarl Bouzid");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let ordered = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(amrani, vec![line(farine, 10_000, 20_000)]),
    )
    .unwrap();
    let received = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            receive_now: true,
            supplier_id: bouzid,
            ..an_order(bouzid, vec![line(farine, 10_000, 20_000)])
        },
    )
    .unwrap();
    assert_eq!(
        purchases::list(&mut conn, SHOP, None, None).unwrap().len(),
        2
    );
    let only_ordered =
        purchases::list(&mut conn, SHOP, Some(PurchaseStatus::Ordered), None).unwrap();
    assert_eq!(only_ordered.len(), 1);
    assert_eq!(only_ordered[0].id, ordered.purchase.id);
    let of_bouzid = purchases::list(&mut conn, SHOP, None, Some(bouzid)).unwrap();
    assert_eq!(of_bouzid.len(), 1);
    assert_eq!(of_bouzid[0].id, received.purchase.id);
    assert!(purchases::list(&mut conn, 2, None, None)
        .unwrap()
        .is_empty());
}

#[test]
fn every_change_to_an_order_writes_the_audit_entry_of_its_own_name() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 10_000, 20_000)]),
    )
    .unwrap();
    purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 4_000,
        }],
        None,
    )
    .unwrap();
    purchases::return_to_supplier(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 1_000,
        }],
        None,
    )
    .unwrap();
    purchases::close_short(&mut conn, SHOP, OWNER, saved.purchase.id, "fin".to_string()).unwrap();
    let actions: Vec<String> = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .filter(|e| e.entity == "purchase")
        .map(|e| e.action)
        .collect();
    assert!(actions.contains(&audit::ACTION_CREATE_PURCHASE.to_string()));
    assert!(actions.contains(&audit::ACTION_RECEIVE_PURCHASE.to_string()));
    assert!(actions.contains(&audit::ACTION_RETURN_PURCHASE.to_string()));
    assert!(actions.contains(&audit::ACTION_CLOSE_SHORT_PURCHASE.to_string()));
    // The reason a close short was given is in the log, because writing off
    // goods that never came is a decision.
    let short = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .find(|e| e.action == audit::ACTION_CLOSE_SHORT_PURCHASE)
        .unwrap();
    assert!(short.after.unwrap_or_default().contains("fin"));
}

#[test]
fn an_order_received_in_parts_asks_for_credit_only_once() {
    // `place_credit_on` is handed the value of the row that has just landed
    // and not the order's whole value: an order received in parts carries one
    // `purchase` row per delivery, and the order's total would count the
    // first delivery as if it had only just been owed. The second delivery
    // would then go looking for credit the ledger had already spent on the
    // first, and refuse the whole receipt.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 10_000, 20_000)]),
    )
    .unwrap();
    purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 9_000,
        }],
        None,
    )
    .unwrap();
    // A dinar off the order, so the first delivery is no longer owed in full.
    supplier_debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(100),
        PaymentMethod::Cash,
        None,
        dzpos_core::services::clock::now(),
    )
    .unwrap();
    let after = purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 1_000,
        }],
        None,
    )
    .unwrap();
    assert_eq!(after.purchase.status, PurchaseStatus::Received);
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(10 * 20_000 - 100)
    );
}

#[test]
fn money_handed_over_with_the_order_is_logged_the_way_any_other_payment_is() {
    // The one path on which cash leaves the drawer while an order is being
    // saved. It writes the same audit entry `supplier_debt::pay` writes, so a
    // comptable reading the log finds every payment to a supplier in one
    // place, and the order it arrived with is named beside it.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            receive_now: true,
            paid_now: Some(Paid {
                amount: Money::centimes(50_000),
                mode: PaymentMethod::Card,
            }),
            ..an_order(supplier, vec![line(farine, 10_000, 20_000)])
        },
    )
    .unwrap();
    let logged = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .find(|e| e.action == audit::ACTION_PAY_SUPPLIER)
        .unwrap();
    assert_eq!(logged.entity, "supplier_debt");
    assert_eq!(logged.entity_id, Some(supplier));
    let after = logged.after.unwrap_or_default();
    assert!(after.contains("\"amount_centimes\":50000"), "{after}");
    assert!(after.contains("\"payment_mode\":\"card\""), "{after}");
    assert!(after.contains("\"balance_centimes\":150000"), "{after}");
    assert!(
        after.contains(&format!("\"purchase_id\":{}", saved.purchase.id)),
        "{after}"
    );
    // And what it settled, so the log reads against the order.
    assert!(after.contains("\"allocations\":[{"), "{after}");
}

#[test]
fn an_order_of_free_samples_is_saved_and_one_with_costs_on_it_is_not() {
    // Two lines worth nothing between them. With no extra costs there is
    // nothing to spread and the order is ordinary: a supplier's free samples
    // are goods the shop takes in and counts. With extra costs there is no
    // honest line to put them on, and the refusal names the field.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let sucre = a_product(&mut conn, "Sucre 1kg", 0);
    let free = vec![line(farine, 10_000, 0), line(sucre, 20_000, 0)];
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            receive_now: true,
            ..an_order(supplier, free.clone())
        },
    )
    .unwrap();
    assert_eq!(saved.purchase.status, PurchaseStatus::Received);
    assert_eq!(saved.lines[0].landed_unit_cost, Money::ZERO);
    assert_eq!(saved.lines[1].landed_unit_cost, Money::ZERO);
    // The goods are on the shelf and the shop owes nothing for them.
    assert_eq!(on_hand(&mut conn, farine), 10_000);
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::ZERO
    );

    let err = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            transport: Money::centimes(5_000),
            ..an_order(supplier, free)
        },
    )
    .unwrap_err();
    match err {
        dzpos_core::error::CoreError::Validation { ref field, .. } => {
            assert_eq!(field, "transport_centimes");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_line_received_in_parts_debits_its_landed_total_and_never_a_centime_more() {
    // A landed cost of 12,85 over two units: the whole line is worth 25,70.
    // Rounding each delivery half-up on its own gives 6,43 and 19,28, which is
    // 25,71: one centime the order never owed, and a shop that had paid the
    // whole 25,70 up front would be left with an order asking for ever.
    //
    // The rule instead: a delivery is worth what the line is worth once it has
    // arrived, less what it was worth before, and the last one takes the
    // remainder.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 2_000, 1_285)]),
    )
    .unwrap();
    let whole = Money::centimes(1_285).checked_mul_milli(2_000).unwrap();
    assert_eq!(whole, Money::centimes(2_570));

    purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 500,
        }],
        None,
    )
    .unwrap();
    // Half of a unit, rounded down: 6,42 and not 6,43.
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(642)
    );

    purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 1_500,
        }],
        None,
    )
    .unwrap();
    // The delivery that finishes the line takes the remainder, so the order
    // has been debited exactly what it is worth.
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        whole
    );
}

#[test]
fn an_order_paid_in_full_before_the_goods_is_settled_by_the_last_delivery() {
    // The centime above, seen from the account: the shop pays the whole order
    // the day it is written, and the deliveries have to add up to exactly
    // that or the order never stops asking to be paid.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            paid_now: Some(Paid {
                amount: Money::centimes(2_570),
                mode: PaymentMethod::Cash,
            }),
            ..an_order(supplier, vec![line(farine, 2_000, 1_285)])
        },
    )
    .unwrap();
    for part in [500, 1_500] {
        purchases::receive(
            &mut conn,
            SHOP,
            OWNER,
            saved.purchase.id,
            vec![ReceiveLine {
                purchase_line_id: saved.lines[0].id,
                qty_milli: part,
            }],
            None,
        )
        .unwrap();
    }
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::ZERO
    );
    assert!(supplier_debt::open_purchases(&mut conn, SHOP, supplier)
        .unwrap()
        .is_empty());
}

#[test]
fn a_delivery_worth_nothing_finishes_the_line_without_a_row_of_nothing() {
    // A thousandth of a unit at a centime is worth nothing at all, and the
    // ledger has no row for a movement of nothing (M2's rule: it would sit in
    // every statement for ever). The delivery still finishes the line and
    // still moves the stock, and the service still asks for the credit the
    // shop is holding to be placed, which is why that call sits outside the
    // branch that writes the row.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order(supplier, vec![line(farine, 1_001, 1)]),
    )
    .unwrap();
    // One unit in: worth one centime, and the order owes it.
    purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 1_000,
        }],
        None,
    )
    .unwrap();
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(1)
    );
    // Paid, and then the last thousandth arrives worth nothing.
    supplier_debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(1),
        PaymentMethod::Cash,
        None,
        dzpos_core::services::clock::now(),
    )
    .unwrap();
    let after = purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 1,
        }],
        None,
    )
    .unwrap();
    assert_eq!(after.purchase.status, PurchaseStatus::Received);
    assert_eq!(on_hand(&mut conn, farine), 1_001);
    // Nothing was added, so the balance is still nothing and no row of zero
    // was written.
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::ZERO
    );
    let ledger = supplier_debt::ledger(&mut conn, SHOP, supplier).unwrap();
    assert!(ledger
        .iter()
        .all(|e| e.debit != Money::ZERO || e.credit != Money::ZERO));
}

#[test]
fn returning_everything_that_arrived_takes_the_whole_debit_back_off() {
    // The mirror of the delivery rule: a return is worth what has gone back
    // once it has, less what had gone back before. A line received whole and
    // returned whole leaves nothing on the account, whatever the rounding did
    // on the way.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            receive_now: true,
            ..an_order(supplier, vec![line(farine, 2_000, 1_285)])
        },
    )
    .unwrap();
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(2_570)
    );
    for part in [500, 1_500] {
        purchases::return_to_supplier(
            &mut conn,
            SHOP,
            OWNER,
            saved.purchase.id,
            vec![ReceiveLine {
                purchase_line_id: saved.lines[0].id,
                qty_milli: part,
            }],
            None,
        )
        .unwrap();
    }
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::ZERO
    );
    assert_eq!(on_hand(&mut conn, farine), 0);
}

#[test]
fn an_order_whose_every_line_has_arrived_takes_no_further_delivery() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg", 0);
    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        NewPurchase {
            receive_now: true,
            ..an_order(supplier, vec![line(farine, 10_000, 20_000)])
        },
    )
    .unwrap();
    assert_eq!(saved.purchase.status, PurchaseStatus::Received);
    let err = purchases::receive(
        &mut conn,
        SHOP,
        OWNER,
        saved.purchase.id,
        vec![ReceiveLine {
            purchase_line_id: saved.lines[0].id,
            qty_milli: 1_000,
        }],
        None,
    )
    .unwrap_err();
    match err {
        dzpos_core::error::CoreError::Validation { ref field, .. } => assert_eq!(field, "status"),
        other => panic!("{other:?}"),
    }
    // And no number of the year's series was burnt on the way past.
    let after = purchases::get(&mut conn, SHOP, saved.purchase.id).unwrap();
    assert_eq!(after.receipts.len(), 1);
    assert_eq!(after.receipts[0].receipt.number, 1);
}
