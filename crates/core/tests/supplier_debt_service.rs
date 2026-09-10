// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! What the shop owes its suppliers (features.md §1). The mirror of
//! `debt_service` on the supply side: the ledger is append-only, the balance
//! is its sum, a payment settles the open orders oldest first through
//! `supplier_allocations`, and a correction is a movement rather than an edit.
//!
//! A purchase writes its own ledger rows in T3. Until then the tests seed the
//! order and the debit a receipt would write, so what a payment settles is a
//! real order carrying real value.

use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::money::Money;
use dzpos_core::services::supplier_debt::{self, PaymentMethod, SupplierDebtKind};
use dzpos_core::services::{audit, clock, suppliers};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

mod common;

use common::{a_purchase_ledger_row, a_purchase_row, a_supplier, open_temp};

fn second_shop(conn: &mut SqliteConnection) {
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(conn)
        .unwrap();
}

/// A supplier with two orders on the ledger, the older one worth 100 000
/// centimes and the newer one 60 000.
fn two_orders(conn: &mut SqliteConnection) -> (i32, i32, i32) {
    let supplier = a_supplier(conn, "Sarl Amrani");
    let older = a_purchase_row(conn, supplier, "2026-09-01");
    a_purchase_ledger_row(conn, supplier, older, 100_000);
    let newer = a_purchase_row(conn, supplier, "2026-09-05");
    a_purchase_ledger_row(conn, supplier, newer, 60_000);
    (supplier, older, newer)
}

#[test]
fn a_supplier_with_no_movement_owes_nothing() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::ZERO
    );
    let statement = supplier_debt::statement(&mut conn, SHOP, supplier).unwrap();
    assert_eq!(statement.balance, Money::ZERO);
    assert!(statement.lines.is_empty());
}

#[test]
fn another_shops_supplier_has_no_ledger_to_read() {
    let (_dir, mut conn) = open_temp();
    second_shop(&mut conn);
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let err = supplier_debt::balance(&mut conn, 2, supplier).unwrap_err();
    assert_eq!(err.code(), "not_found", "{err}");
    assert!(supplier_debt::ledger(&mut conn, 2, supplier).is_err());
}

#[test]
fn the_statement_carries_the_running_balance_and_reads_newest_first() {
    let (_dir, mut conn) = open_temp();
    let (supplier, _older, _newer) = two_orders(&mut conn);
    let statement = supplier_debt::statement(&mut conn, SHOP, supplier).unwrap();
    assert_eq!(statement.balance, Money::centimes(160_000));
    assert_eq!(statement.lines.len(), 2);
    // Newest first, each row carrying what was owed once it had landed. The
    // column is the core's, so a screen showing it adds nothing up.
    assert_eq!(statement.lines[0].balance_after, Money::centimes(160_000));
    assert_eq!(statement.lines[1].balance_after, Money::centimes(100_000));
}

#[test]
fn a_payment_settles_the_oldest_order_first_and_stops_where_the_money_does() {
    let (_dir, mut conn) = open_temp();
    let (supplier, older, newer) = two_orders(&mut conn);
    let paid = supplier_debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(120_000),
        PaymentMethod::Cash,
        Some("versement".to_string()),
        clock::now(),
    )
    .unwrap();

    assert_eq!(paid.entry.kind, SupplierDebtKind::Payment);
    assert_eq!(paid.entry.credit, Money::centimes(120_000));
    assert_eq!(paid.entry.payment_mode, Some(PaymentMethod::Cash));
    assert_eq!(paid.balance_after, Money::centimes(40_000));
    // The whole of the older order, then what is left goes on the newer one.
    assert_eq!(paid.allocations.len(), 2);
    assert_eq!(paid.allocations[0].purchase_id, older);
    assert_eq!(paid.allocations[0].amount, Money::centimes(100_000));
    assert_eq!(paid.allocations[1].purchase_id, newer);
    assert_eq!(paid.allocations[1].amount, Money::centimes(20_000));

    // What is still owed on each order, read back the way the next payment
    // will read it.
    let open = supplier_debt::open_purchases(&mut conn, SHOP, supplier).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].purchase_id, newer);
    assert_eq!(open[0].remaining, Money::centimes(40_000));
}

#[test]
fn a_second_payment_takes_up_where_the_first_one_left_off() {
    let (_dir, mut conn) = open_temp();
    let (supplier, _older, newer) = two_orders(&mut conn);
    supplier_debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(120_000),
        PaymentMethod::Cash,
        None,
        clock::now(),
    )
    .unwrap();
    let second = supplier_debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(40_000),
        PaymentMethod::Card,
        None,
        clock::now(),
    )
    .unwrap();
    assert_eq!(second.allocations.len(), 1);
    assert_eq!(second.allocations[0].purchase_id, newer);
    assert_eq!(second.allocations[0].amount, Money::centimes(40_000));
    assert_eq!(second.balance_after, Money::ZERO);
    assert!(supplier_debt::open_purchases(&mut conn, SHOP, supplier)
        .unwrap()
        .is_empty());
}

#[test]
fn money_that_no_order_can_take_still_settles_the_balance() {
    // An opening balance carries no order at all, so a payment against it
    // settles the balance and no piece of paper.
    let (_dir, mut conn) = open_temp();
    let supplier = suppliers::create(
        &mut conn,
        SHOP,
        OWNER,
        common::a_supplier_fiche("Sarl Amrani"),
        Some(Money::centimes(80_000)),
    )
    .unwrap()
    .id;
    let paid = supplier_debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(80_000),
        PaymentMethod::Cash,
        None,
        clock::now(),
    )
    .unwrap();
    assert!(paid.allocations.is_empty());
    assert_eq!(paid.balance_after, Money::ZERO);
}

#[test]
fn a_payment_above_what_is_owed_is_refused_and_says_what_is_owed() {
    let (_dir, mut conn) = open_temp();
    let (supplier, _older, _newer) = two_orders(&mut conn);
    let err = supplier_debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(160_001),
        PaymentMethod::Cash,
        None,
        clock::now(),
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::PaymentAboveDebt {
                outstanding_centimes: 160_000
            }
        ),
        "{err}"
    );
    assert_eq!(err.code(), "validation");
    // Nothing was written: not the movement, and not an allocation.
    assert_eq!(
        supplier_debt::ledger(&mut conn, SHOP, supplier)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn a_payment_to_a_supplier_owed_nothing_says_nothing_is_owed() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let err = supplier_debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(1_000),
        PaymentMethod::Cash,
        None,
        clock::now(),
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::PaymentAboveDebt {
                outstanding_centimes: 0
            }
        ),
        "{err}"
    );
}

#[test]
fn a_payment_of_nothing_pays_nothing() {
    let (_dir, mut conn) = open_temp();
    let (supplier, _older, _newer) = two_orders(&mut conn);
    for amount in [Money::ZERO, Money::centimes(-100)] {
        let err = supplier_debt::pay(
            &mut conn,
            SHOP,
            OWNER,
            supplier,
            amount,
            PaymentMethod::Cash,
            None,
            clock::now(),
        )
        .unwrap_err();
        assert!(
            matches!(&err, CoreError::Validation { field, .. } if field == "amount_centimes"),
            "{err}"
        );
    }
}

#[test]
fn a_closed_fiche_still_takes_a_payment() {
    // A shop closes a fiche to stop buying from somebody, not to stop paying
    // them what it already owes.
    let (_dir, mut conn) = open_temp();
    let (supplier, _older, _newer) = two_orders(&mut conn);
    suppliers::close(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Some("le fournisseur a fermé".to_string()),
    )
    .unwrap();
    let paid = supplier_debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(160_000),
        PaymentMethod::Cash,
        None,
        clock::now(),
    )
    .unwrap();
    assert_eq!(paid.balance_after, Money::ZERO);
}

#[test]
fn a_payment_is_logged_with_both_balances_and_what_it_settled() {
    let (_dir, mut conn) = open_temp();
    let (supplier, older, _newer) = two_orders(&mut conn);
    supplier_debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(100_000),
        PaymentMethod::Card,
        None,
        clock::now(),
    )
    .unwrap();

    let entry = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .find(|e| e.action == "supplier_debt.pay")
        .expect("money paid to a supplier is logged");
    assert_eq!(entry.entity, "supplier_debt");
    assert_eq!(entry.entity_id, Some(supplier));
    let before: serde_json::Value =
        serde_json::from_str(&entry.before.unwrap_or_default()).unwrap();
    assert_eq!(before["balance_centimes"], 160_000);
    let after: serde_json::Value = serde_json::from_str(&entry.after.unwrap_or_default()).unwrap();
    assert_eq!(after["balance_centimes"], 60_000);
    assert_eq!(after["amount_centimes"], 100_000);
    assert_eq!(after["payment_mode"], "card");
    assert_eq!(after["allocations"][0]["purchase_id"], older);
    assert_eq!(after["allocations"][0]["amount_centimes"], 100_000);
}

#[test]
fn a_correction_downwards_settles_the_oldest_order_first() {
    let (_dir, mut conn) = open_temp();
    let (supplier, older, _newer) = two_orders(&mut conn);
    let written = supplier_debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(-30_000),
        Some("rabais accordé".to_string()),
    )
    .unwrap();
    assert_eq!(written.entry.kind, SupplierDebtKind::Adjustment);
    assert_eq!(written.entry.credit, Money::centimes(30_000));
    assert_eq!(written.entry.debit, Money::ZERO);
    assert_eq!(written.entry.payment_mode, None);
    assert_eq!(written.allocations.len(), 1);
    assert_eq!(written.allocations[0].purchase_id, older);
    // The amount as well as the order: a correction that named the right
    // paper and took the wrong figure off it would pass on the id alone.
    assert_eq!(written.allocations[0].amount, Money::centimes(30_000));
    assert_eq!(written.statement.balance, Money::centimes(130_000));
}

#[test]
fn a_correction_upwards_settles_nothing() {
    let (_dir, mut conn) = open_temp();
    let (supplier, _older, _newer) = two_orders(&mut conn);
    let written = supplier_debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(30_000),
        None,
    )
    .unwrap();
    assert_eq!(written.entry.debit, Money::centimes(30_000));
    assert!(
        written.allocations.is_empty(),
        "debt no order carries settles no order"
    );
    assert_eq!(written.statement.balance, Money::centimes(190_000));
}

#[test]
fn a_correction_of_nothing_corrects_nothing() {
    let (_dir, mut conn) = open_temp();
    let (supplier, _older, _newer) = two_orders(&mut conn);
    let err =
        supplier_debt::adjust(&mut conn, SHOP, OWNER, supplier, Money::ZERO, None).unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "amount"),
        "{err}"
    );
}

#[test]
fn a_correction_is_logged_with_both_balances() {
    let (_dir, mut conn) = open_temp();
    let (supplier, _older, _newer) = two_orders(&mut conn);
    supplier_debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(-30_000),
        Some("rabais accordé".to_string()),
    )
    .unwrap();
    let entry = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .find(|e| e.action == "supplier_debt.adjust")
        .expect("a correction is logged");
    assert_eq!(entry.entity, "supplier_debt");
    let after: serde_json::Value = serde_json::from_str(&entry.after.unwrap_or_default()).unwrap();
    assert_eq!(after["balance_centimes"], 130_000);
    assert_eq!(after["amount_centimes"], -30_000);
    assert_eq!(after["note"], "rabais accordé");
}

#[test]
fn the_statement_over_a_range_opens_where_the_movements_before_it_left_off() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let today = clock::now();
    // Two movements on the shop's clock, one before the range asked for
    // below and one inside it.
    supplier_debt::append_at(
        &mut conn,
        SHOP,
        dzpos_core::services::supplier_debt::NewSupplierEntry {
            supplier_id: supplier,
            purchase_id: None,
            kind: SupplierDebtKind::Opening,
            debit: Money::centimes(100_000),
            credit: Money::ZERO,
            user_id: OWNER,
            note: None,
        },
        Some(today - chrono::Duration::days(10)),
    )
    .unwrap();
    let inside = supplier_debt::append_at(
        &mut conn,
        SHOP,
        dzpos_core::services::supplier_debt::NewSupplierEntry {
            supplier_id: supplier,
            purchase_id: None,
            kind: SupplierDebtKind::Adjustment,
            debit: Money::centimes(20_000),
            credit: Money::ZERO,
            user_id: OWNER,
            note: None,
        },
        Some(today),
    )
    .unwrap();

    let day = today.date();
    let statement = supplier_debt::statement_between(&mut conn, SHOP, supplier, day, day).unwrap();
    assert_eq!(statement.from, day);
    assert_eq!(statement.opening, Money::centimes(100_000));
    assert_eq!(statement.entries.len(), 1);
    assert_eq!(statement.entries[0].entry.id, inside.id);
    assert_eq!(statement.entries[0].balance_after, Money::centimes(120_000));
    assert_eq!(statement.closing, Money::centimes(120_000));
}

#[test]
fn a_range_that_ends_before_it_starts_is_refused() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let from = NaiveDate::from_ymd_opt(2026, 9, 10).unwrap();
    let to = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
    let err = supplier_debt::statement_between(&mut conn, SHOP, supplier, from, to).unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "to"),
        "{err}"
    );
}

#[test]
fn a_movement_raises_the_debt_or_lowers_it_and_never_both_or_neither() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    for (debit, credit) in [
        (Money::centimes(100), Money::centimes(100)),
        (Money::ZERO, Money::ZERO),
        (Money::centimes(-100), Money::ZERO),
    ] {
        let err = supplier_debt::append(
            &mut conn,
            SHOP,
            dzpos_core::services::supplier_debt::NewSupplierEntry {
                supplier_id: supplier,
                purchase_id: None,
                kind: SupplierDebtKind::Adjustment,
                debit,
                credit,
                user_id: OWNER,
                note: None,
            },
        )
        .unwrap_err();
        assert_eq!(err.code(), "validation", "{err}");
    }
}

#[test]
fn credit_the_shop_holds_reaches_the_next_order_and_the_fiche_then_closes() {
    // The customer side settles a document out of credit the customer already
    // holds at the moment the document is issued (M2 ruling, features.md §3).
    // Without the same on this side, credit left by a correction never reaches
    // an order written after it: the order would go on asking for its whole
    // value while the shop owed less than that, and a payment of what is owed
    // would leave it open forever.
    let (_dir, mut conn) = open_temp();
    let supplier = suppliers::create(
        &mut conn,
        SHOP,
        OWNER,
        common::a_supplier_fiche("Sarl Amrani"),
        Some(Money::centimes(80_000)),
    )
    .unwrap()
    .id;

    let older = a_purchase_row(&mut conn, supplier, "2026-09-01");
    a_purchase_ledger_row(&mut conn, supplier, older, 50_000);
    // Nothing is held yet, so this places nothing: the call is what T3's
    // receipt path makes after every `purchase` row, held or not.
    assert!(supplier_debt::place_credit_on(&mut conn, SHOP, older)
        .unwrap()
        .is_empty());

    // A correction past what was owed: 50 000 of it lands on the older order
    // and the rest leaves the supplier owing the shop.
    supplier_debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(-140_000),
        None,
    )
    .unwrap();
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(-10_000)
    );

    let newer = a_purchase_row(&mut conn, supplier, "2026-09-05");
    a_purchase_ledger_row(&mut conn, supplier, newer, 20_000);
    let placed = supplier_debt::place_credit_on(&mut conn, SHOP, newer).unwrap();
    // What was held and no more: the 80 000 of the correction that answered
    // the opening balance is not credit, it is a debt that was written off.
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].purchase_id, newer);
    assert_eq!(placed[0].amount, Money::centimes(10_000));

    let open = supplier_debt::open_purchases(&mut conn, SHOP, supplier).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].purchase_id, newer);
    assert_eq!(open[0].remaining, Money::centimes(10_000));
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(10_000)
    );

    supplier_debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(10_000),
        PaymentMethod::Cash,
        None,
        clock::now(),
    )
    .unwrap();
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::ZERO
    );
    assert!(supplier_debt::open_purchases(&mut conn, SHOP, supplier)
        .unwrap()
        .is_empty());
    // Nothing is owed and no order is asking, so the fiche closes the way any
    // settled one does.
    let closed = suppliers::close(&mut conn, SHOP, OWNER, supplier, None).unwrap();
    assert!(!closed.active);
}

#[test]
fn credit_is_placed_on_one_order_only_up_to_what_that_order_asks_for() {
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    // The shop paid in advance: 100 000 sitting with the supplier and no
    // order to put it on yet.
    supplier_debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(-100_000),
        None,
    )
    .unwrap();
    let order = a_purchase_row(&mut conn, supplier, "2026-09-05");
    a_purchase_ledger_row(&mut conn, supplier, order, 30_000);
    let placed = supplier_debt::place_credit_on(&mut conn, SHOP, order).unwrap();
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].amount, Money::centimes(30_000));
    assert!(supplier_debt::open_purchases(&mut conn, SHOP, supplier)
        .unwrap()
        .is_empty());
    // The rest is still held: the order took what it asked for and no more.
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(-70_000)
    );
    // And it is not placed twice.
    assert!(supplier_debt::place_credit_on(&mut conn, SHOP, order)
        .unwrap()
        .is_empty());
}

#[test]
fn a_closed_fiche_still_takes_a_correction() {
    // A shop closes a fiche to stop buying from somebody, not to stop putting
    // right what it owes them. The same rule the payment above holds.
    let (_dir, mut conn) = open_temp();
    let (supplier, _older, _newer) = two_orders(&mut conn);
    suppliers::close(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Some("le fournisseur a fermé".to_string()),
    )
    .unwrap();
    let written = supplier_debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        supplier,
        Money::centimes(-30_000),
        Some("rabais accordé".to_string()),
    )
    .unwrap();
    assert_eq!(written.statement.balance, Money::centimes(130_000));
}

#[test]
fn a_closed_fiche_is_what_a_purchase_is_refused_on() {
    // What a closed fiche does refuse is more goods. T3's receipt path asks
    // this before it writes a `purchase` row, the way a sale asks the same of
    // a customer's fiche.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    supplier_debt::ensure_active(&mut conn, SHOP, supplier).unwrap();
    suppliers::close(&mut conn, SHOP, OWNER, supplier, None).unwrap();
    let err = supplier_debt::ensure_active(&mut conn, SHOP, supplier).unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "supplier_id"),
        "{err}"
    );
    // And another shop's supplier is not found rather than active.
    second_shop(&mut conn);
    let err = supplier_debt::ensure_active(&mut conn, 2, supplier).unwrap_err();
    assert_eq!(err.code(), "not_found", "{err}");
}
