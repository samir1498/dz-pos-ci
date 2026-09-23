// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The `payment` row an order pays for as it is written (features.md §1,
//! Purchase): the one path on which money leaves the drawer while an order is
//! being saved rather than through `supplier_debt::pay`.
//!
//! Its own file because `purchases_service.rs` is at the length
//! `scripts/file-sizes.json` pins it to and may not grow, and because this is
//! one subject: what that row holds, and the bound this path does not have.

use diesel::sqlite::SqliteConnection;
use dzpos_kernel::services::clock;
use dzpos_retail::models::product::{NewProduct, Unit};
use dzpos_retail::money::{Bps, Money};
use dzpos_retail::services::products;
use dzpos_retail::services::purchases::{self, NewLine, NewPurchase, Paid};
use dzpos_retail::services::supplier_debt::{self, PaymentMethod, SupplierDebtKind};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

mod common;

use common::{a_purchase_ledger_row, a_purchase_row, a_supplier, open_temp};

fn a_product(conn: &mut SqliteConnection, name: &str) -> i32 {
    products::create(
        conn,
        SHOP,
        OWNER,
        NewProduct {
            name: name.to_string(),
            barcode: None,
            category_id: None,
            unit: Unit::Piece,
            cost: Money::ZERO,
            selling: Money::centimes(40_000),
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

/// Ten units at 20 000 centimes each and nothing spread over them: an order
/// worth 200 000 centimes, written by hand here rather than summed from the
/// lines by the code the test is about.
fn an_order_worth_200_000(supplier_id: i32, product_id: i32, paid: Option<Paid>) -> NewPurchase {
    NewPurchase {
        supplier_id,
        supplier_document_number: None,
        purchase_date: "2026-09-10".to_string(),
        due_date: None,
        transport: Money::ZERO,
        extra_costs: Money::ZERO,
        note: None,
        lines: vec![NewLine {
            product_id,
            qty_ordered_milli: 10_000,
            unit_cost: Money::centimes(20_000),
        }],
        paid_now: paid,
        // The goods come later, so the order carries no ledger row of its own
        // while the money is handed over. That is the case `pay` cannot take.
        receive_now: false,
    }
}

#[test]
fn money_handed_over_with_an_order_is_not_measured_against_the_balance() {
    // `supplier_debt::pay` refuses money above what the shop owes. This path
    // must not take that bound: the supplier is owed 50 000 for an older
    // order, the shop hands over 200 000 for one whose goods have not come
    // yet, and 200 000 is four times the balance at the moment it is written.
    // What no open order can take stays on the balance as credit, which the
    // next receipt places.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let older = a_purchase_row(&mut conn, supplier, "2026-09-01");
    a_purchase_ledger_row(&mut conn, supplier, older, 50_000);
    let farine = a_product(&mut conn, "Farine 5kg");

    let saved = purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order_worth_200_000(
            supplier,
            farine,
            Some(Paid {
                amount: Money::centimes(200_000),
                mode: PaymentMethod::Cash,
            }),
        ),
    )
    .unwrap();

    // 50 000 owed less 200 000 handed over: the supplier holds 150 000 of the
    // shop's money.
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, supplier).unwrap(),
        Money::centimes(-150_000)
    );
    assert_eq!(
        supplier_debt::credit_held(supplier_debt::balance(&mut conn, SHOP, supplier).unwrap())
            .unwrap(),
        Money::centimes(150_000)
    );
    // Oldest first, and the order it arrived with is not the one it settled:
    // the older order took all 50 000 it was still asking for, and the order
    // just saved took nothing, because nothing has arrived against it.
    let placed = supplier_debt::allocations(&mut conn, SHOP, older).unwrap();
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].amount, Money::centimes(50_000));
    assert!(
        supplier_debt::allocations(&mut conn, SHOP, saved.purchase.id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn the_payment_row_an_order_writes_carries_the_mode_no_order_and_the_shop_clock() {
    // The row itself, column by column, because this path writes it beside a
    // purchase rather than through `pay` and a drift here would be a payment
    // the cash position reads on the wrong day, or one no screen can tell was
    // cash. Its `purchase_id` is empty on purpose: a payment settles orders
    // through its allocations, which can be several, so the column that names
    // one order would have to pick.
    let (_dir, mut conn) = open_temp();
    let supplier = a_supplier(&mut conn, "Sarl Amrani");
    let farine = a_product(&mut conn, "Farine 5kg");

    let before = clock::now();
    purchases::save(
        &mut conn,
        SHOP,
        OWNER,
        an_order_worth_200_000(
            supplier,
            farine,
            Some(Paid {
                amount: Money::centimes(120_000),
                mode: PaymentMethod::Card,
            }),
        ),
    )
    .unwrap();
    let after = clock::now();

    let ledger = supplier_debt::ledger(&mut conn, SHOP, supplier).unwrap();
    assert_eq!(ledger.len(), 1);
    let payment = &ledger[0];
    assert_eq!(payment.kind, SupplierDebtKind::Payment);
    assert_eq!(payment.credit, Money::centimes(120_000));
    assert_eq!(payment.debit, Money::ZERO);
    assert_eq!(payment.payment_mode, Some(PaymentMethod::Card));
    assert_eq!(payment.purchase_id, None);
    assert_eq!(payment.note, None);
    assert_eq!(payment.user_id, OWNER);
    // Stamped by the shop's clock and never left to the column's own
    // CURRENT_TIMESTAMP, which is UTC: the cash position reads a day's cash
    // off this column on the shop's calendar, and one hour a day the two
    // disagree about which day the money left the drawer.
    assert!(
        payment.created_at >= before && payment.created_at <= after,
        "{} is not between {before} and {after}",
        payment.created_at
    );
    let utc = chrono::Utc::now().naive_utc();
    let ahead = (payment.created_at - utc).num_seconds();
    assert!(
        (3500..=3600).contains(&ahead),
        "{} is not an Algerian hour ahead of {utc}",
        payment.created_at
    );
}
