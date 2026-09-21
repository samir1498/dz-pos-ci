// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Till shifts (features.md §1, the cash position): a drawer opened with a
//! float, counted at close, and the difference between the two.
//!
//! Every expected figure below is written out by hand from the rule in
//! `context/plans/20260921-till-shifts-a-float-and-a-count.md`:
//!
//!     opening_cash + that user's cash sales + that user's cash debt payments
//!
//! None of them is read back off the code under test. A fixture whose
//! expectation is computed the way the code computes it agrees with the code
//! whatever the code does.
//!
//! The fixture day is 2026-09-14, a week before this file was written, for a
//! reason the boundary tests spell out: `documents.created_at` takes SQLite's
//! `CURRENT_TIMESTAMP` and holds the moment the row was inserted, so a window
//! comparison written against that column instead of `issued_at` puts every
//! sale here outside every shift and the expected figure collapses to the
//! float. On a fixture dated today the two columns are an hour apart and the
//! swap survives.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sql_types::Timestamp;
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::money::{Bps, Money, PaymentMode, Regime, Totals, TvaLine};
use dzpos_core::services::audit;
use dzpos_core::services::cancellation;
use dzpos_core::services::cash;
use dzpos_core::services::debt::{self, DebtKind, NewDebtEntry, PaymentMethod};
use dzpos_core::services::documents::{
    self, DocumentKind, NewDocument, NewDocumentLine, SellerBlock,
};
use dzpos_core::services::expenses::{self, NewExpense};
use dzpos_core::services::shifts::{self, NewShift, TillCount};

mod common;
use common::{a_customer, open_temp};

const SHOP: i32 = 1;
/// The shift's own cashier, the user the first migration seeds.
const AMINA: i32 = 1;
/// The second person at the same till. The point of most of this file: her
/// takings are hers, and a shop-wide sum would hand them to Amina.
const KARIM: i32 = 2;

fn day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 14).unwrap()
}

/// A moment on the fixture day, on the shop's clock.
fn at(hour: u32, minute: u32, second: u32) -> NaiveDateTime {
    day().and_hms_opt(hour, minute, second).unwrap()
}

/// Midnight opening the fixture day and midnight opening the next: the window
/// the "which sales belonged to no shift" question is asked over.
fn the_whole_day() -> (NaiveDateTime, NaiveDateTime) {
    (
        day().and_hms_opt(0, 0, 0).unwrap(),
        day().succ_opt().unwrap().and_hms_opt(0, 0, 0).unwrap(),
    )
}

/// A second person at the till. The seeded file carries one user, and a
/// fixture with one user cannot tell a per-cashier figure from a shop-wide
/// one.
fn a_second_cashier(conn: &mut SqliteConnection) {
    diesel::sql_query(format!(
        "INSERT INTO users (id, shop_id, name, role) \
         VALUES ({KARIM}, {SHOP}, 'Karim', 'cashier')"
    ))
    .execute(conn)
    .unwrap();
}

/// A document written straight through `services::documents`, with its totals
/// stated rather than computed: what is under test is whose cash a shift
/// counts, not how a basket adds up.
///
/// `user_id` is a parameter and not a constant, which is the whole reason this
/// helper is not `cash_service.rs`'s: a fixture that could not name a second
/// ringer could not tell the two figures apart.
fn a_sale(
    conn: &mut SqliteConnection,
    user_id: i32,
    mode: PaymentMode,
    total_ttc: i64,
    issued_at: NaiveDateTime,
) -> i32 {
    a_sale_with_stamp(conn, user_id, mode, total_ttc, 0, issued_at)
}

/// The same, on a facture carrying a droit de timbre. The customer hands the
/// stamp over with the rest, so the drawer holds `net_to_pay` and not
/// `total_ttc`.
fn a_sale_with_stamp(
    conn: &mut SqliteConnection,
    user_id: i32,
    mode: PaymentMode,
    total_ttc: i64,
    stamp: i64,
    issued_at: NaiveDateTime,
) -> i32 {
    let ttc = Money::centimes(total_ttc);
    let stamp = Money::centimes(stamp);
    documents::issue(
        conn,
        SHOP,
        NewDocument {
            kind: DocumentKind::Ticket,
            issued_at,
            user_id,
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

/// A customer who owes money, so somebody can hand cash over against it.
fn a_debtor(conn: &mut SqliteConnection, owes: i64, when: NaiveDateTime) -> i32 {
    let customer = a_customer(conn, "Entreprise Benali");
    debt::append_at(
        conn,
        SHOP,
        NewDebtEntry {
            customer_id: customer,
            document_id: None,
            kind: DebtKind::Sale,
            debit: Money::centimes(owes),
            credit: Money::ZERO,
            user_id: AMINA,
            note: None,
        },
        Some(when),
    )
    .unwrap();
    customer
}

/// Money to a supplier, written in SQL: what this file needs is the
/// `supplier_ledger` row, not the service that owns writing one.
fn pay_a_supplier(conn: &mut SqliteConnection, centimes: i64, when: NaiveDateTime) {
    diesel::sql_query(format!(
        "INSERT INTO suppliers (id, shop_id, name) VALUES (1, {SHOP}, 'Fournisseur')"
    ))
    .execute(conn)
    .unwrap();
    let stamped = when.format("%Y-%m-%d %H:%M:%S");
    diesel::sql_query(format!(
        "INSERT INTO supplier_ledger \
         (shop_id, supplier_id, kind, debit_centimes, credit_centimes, user_id, created_at, payment_mode) \
         VALUES ({SHOP}, 1, 'payment', 0, {centimes}, {AMINA}, '{stamped}', 'cash')"
    ))
    .execute(conn)
    .unwrap();
}

fn spend(conn: &mut SqliteConnection, centimes: i64) {
    let category = expenses::categories(conn, SHOP).unwrap()[0].id;
    expenses::create(
        conn,
        SHOP,
        AMINA,
        NewExpense {
            category_id: category,
            amount: Money::centimes(centimes),
            expense_date: day(),
            note: None,
        },
    )
    .unwrap();
}

#[derive(QueryableByName)]
struct Stamp {
    #[diesel(sql_type = Timestamp)]
    created_at: NaiveDateTime,
}

/// What SQLite's own `CURRENT_TIMESTAMP` wrote on the row, which is the moment
/// of the INSERT in UTC and never the moment the sale happened.
fn created_at_of(conn: &mut SqliteConnection, document_id: i32) -> NaiveDateTime {
    diesel::sql_query(format!(
        "SELECT created_at FROM documents WHERE id = {document_id}"
    ))
    .get_result::<Stamp>(conn)
    .unwrap()
    .created_at
}

fn opened_at_nine(opening_cash: i64) -> NewShift {
    NewShift {
        opened_at: at(9, 0, 0),
        opening_cash: Money::centimes(opening_cash),
    }
}

#[test]
fn the_expected_figure_is_this_cashiers_own_cash_and_nothing_else() {
    let (_dir, mut conn) = open_temp();
    a_second_cashier(&mut conn);
    let customer = a_debtor(&mut conn, 500_000, at(8, 0, 0));

    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(500_000)).unwrap();

    // Amina's own cash, inside her window. These four figures and the float
    // are the only ones the expected figure is allowed to be made of.
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 120_000, at(10, 0, 0));
    a_sale_with_stamp(
        &mut conn,
        AMINA,
        PaymentMode::Cash,
        300_000,
        2_000,
        at(11, 0, 0),
    );
    debt::pay(
        &mut conn,
        SHOP,
        AMINA,
        customer,
        Money::centimes(75_000),
        PaymentMethod::Cash,
        None,
        at(12, 0, 0),
    )
    .unwrap();

    // Four things inside the same window that may not reach it. A card sale
    // never touches a drawer; an expense and a supplier payment are the
    // shop's money going out under `commit_money`, which a cashier does not
    // hold; and Karim's cash is Karim's. Karim's sale is the point of the
    // fixture: without it this test passes against a shop-wide sum.
    a_sale(&mut conn, AMINA, PaymentMode::Card, 90_000, at(13, 0, 0));
    spend(&mut conn, 40_000);
    pay_a_supplier(&mut conn, 60_000, at(15, 0, 0));
    a_sale(&mut conn, KARIM, PaymentMode::Cash, 250_000, at(16, 0, 0));
    debt::pay(
        &mut conn,
        SHOP,
        KARIM,
        customer,
        Money::centimes(33_000),
        PaymentMethod::Cash,
        None,
        at(16, 30, 0),
    )
    .unwrap();

    // 500 000 float + 120 000 + 302 000 (300 000 and the 2 000 stamp the
    // customer handed over with it) + 75 000 against the debt = 997 000.
    // Written out here; never asked of the code.
    let expected = Money::centimes(997_000);

    let live = shifts::report(&mut conn, SHOP, shift.id).unwrap();
    assert_eq!(live.expected, expected);
    assert_eq!(live.takings.sales, Money::centimes(422_000));
    assert_eq!(live.takings.stamp, Money::centimes(2_000));
    assert_eq!(live.takings.customer_payments, Money::centimes(75_000));
    assert_eq!(live.difference, None, "nothing has been counted yet");

    // What a shop-wide sum would have answered, so the number this test
    // refuses is on the page beside the one it asks for.
    assert_ne!(expected, Money::centimes(1_280_000));

    let counted = Money::centimes(977_000);
    let closed = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted,
            note: Some("20 000 remis au patron".to_string()),
            at: Some(at(19, 0, 0)),
        },
    )
    .unwrap();
    let close = closed.close.clone().unwrap();
    assert_eq!(close.expected, expected);
    assert_eq!(close.counted, counted);
    // Short, so negative. The sign is the whole answer the screen shows.
    assert_eq!(close.difference().unwrap(), Money::centimes(-20_000));
    assert_eq!(closed.note.as_deref(), Some("20 000 remis au patron"));

    // And the shop's own figure did not move: a shift asks a narrower
    // question of the same module and is never an input to this one.
    let position = cash::position(
        &mut conn,
        SHOP,
        dzpos_core::services::clock::Period::Day(day()),
    )
    .unwrap();
    assert_eq!(position.cash_in.sales, Money::centimes(672_000));
    assert_eq!(position.cash_in.customer_payments, Money::centimes(108_000));
}

#[test]
fn a_difference_with_no_reason_is_refused_before_anything_is_written() {
    let (_dir, mut conn) = open_temp();
    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(100_000)).unwrap();
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 50_000, at(10, 0, 0));

    // 100 000 float + 50 000 = 150 000, by hand.
    let refused = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(140_000),
            note: None,
            at: Some(at(19, 0, 0)),
        },
    );
    assert!(
        matches!(&refused, Err(CoreError::Validation { field, .. }) if field == "note"),
        "a drawer 10 000 short closed with no reason came back as {refused:?}"
    );
    // A blank note is no note: the column would be cleared, and the file's
    // CHECK would refuse the row as a diesel error an API cannot hang on an
    // input.
    let blank = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(140_000),
            note: Some("   ".to_string()),
            at: Some(at(19, 0, 0)),
        },
    );
    assert!(matches!(blank, Err(CoreError::Validation { ref field, .. }) if field == "note"));

    // Nothing was written: the till is still open and still countable.
    assert!(shifts::open_for(&mut conn, SHOP, AMINA)
        .unwrap()
        .is_some_and(|s| s.id == shift.id));
    let closed = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(140_000),
            note: Some("un billet manque".to_string()),
            at: Some(at(19, 0, 0)),
        },
    )
    .unwrap();
    assert_eq!(
        closed.close.unwrap().difference().unwrap(),
        Money::centimes(-10_000)
    );
    assert_eq!(shifts::open_for(&mut conn, SHOP, AMINA).unwrap(), None);
}

#[test]
fn an_equal_count_closes_with_no_reason_at_all() {
    // The other side of the rule above. A refusal alone is satisfied by an
    // unconditional "a close needs a note", which would break every clean
    // evening. A drawer that opened empty and sold nothing is the same case at
    // zero, which is where an implementation that treats zero as missing goes
    // wrong.
    let (_dir, mut conn) = open_temp();
    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(0)).unwrap();
    let closed = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::ZERO,
            note: None,
            at: Some(at(19, 0, 0)),
        },
    )
    .unwrap();
    let close = closed.close.unwrap();
    assert_eq!(close.expected, Money::ZERO);
    assert_eq!(close.counted, Money::ZERO);
    assert_eq!(close.difference().unwrap(), Money::ZERO);
    assert_eq!(closed.note, None);
}

#[test]
fn a_sale_at_the_moment_the_drawer_opened_is_counted_and_one_at_the_moment_it_closed_is_not() {
    // The window is `opened_at <= issued_at < closed_at`. A fixture whose rows
    // all sit well inside one shift passes with the comparison inverted, so
    // the three moments here are the open, the last second before the close,
    // and the close itself.
    let (_dir, mut conn) = open_temp();
    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(0)).unwrap();
    let on_the_dot = a_sale(&mut conn, AMINA, PaymentMode::Cash, 10_000, at(9, 0, 0));
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 5_000, at(18, 59, 59));
    let at_the_close = a_sale(&mut conn, AMINA, PaymentMode::Cash, 20_000, at(19, 0, 0));
    // And one before the drawer was opened, which is the same boundary from
    // the other side.
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 7_000, at(8, 59, 59));

    // The column this reads is `issued_at`. `created_at` holds the moment the
    // INSERT ran, which is neither of these and is not even on the fixture's
    // day: a comparison written against it would put all four sales outside
    // the shift and answer the float.
    assert_ne!(created_at_of(&mut conn, on_the_dot), at(9, 0, 0));
    assert_ne!(created_at_of(&mut conn, at_the_close), at(19, 0, 0));

    // 10 000 + 5 000, by hand. Not the 20 000 rung at the close and not the
    // 7 000 rung a second before the open.
    let closed = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(15_000),
            note: None,
            at: Some(at(19, 0, 0)),
        },
    )
    .unwrap();
    let close = closed.close.unwrap();
    assert_eq!(close.expected, Money::centimes(15_000));
    assert_eq!(close.difference().unwrap(), Money::ZERO);
}

#[test]
fn a_sale_rung_with_no_shift_open_belongs_to_no_shift() {
    let (_dir, mut conn) = open_temp();
    a_second_cashier(&mut conn);
    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(0)).unwrap();
    let inside = a_sale(&mut conn, AMINA, PaymentMode::Cash, 10_000, at(10, 0, 0));
    let before = a_sale(&mut conn, AMINA, PaymentMode::Cash, 3_000, at(7, 0, 0));
    // The two boundaries, put through the tag and not only through the sum.
    // They are separate comparisons in separate files — the repo query's
    // `issued_at` bounds and this module's own `covers` — and each needs a row
    // sitting exactly on it or one of them can be inverted with the suite
    // still green.
    let on_the_dot = a_sale(&mut conn, AMINA, PaymentMode::Cash, 2_000, at(9, 0, 0));
    let at_the_close = a_sale(&mut conn, AMINA, PaymentMode::Cash, 6_000, at(19, 0, 0));
    // 10 000 + 2 000, by hand: the sale at the open is in, the one at the
    // close is out.
    shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(12_000),
            note: None,
            at: Some(at(19, 0, 0)),
        },
    )
    .unwrap();
    // The phone's queue replaying after the close (ruling 3): `issued_at` is
    // the moment the sale reached the server, so a sale rung offline at 18:00
    // and synced at 19:30 lands after the drawer was counted.
    let replayed = a_sale(&mut conn, AMINA, PaymentMode::Cash, 4_000, at(19, 30, 0));
    let on_the_card = a_sale(&mut conn, AMINA, PaymentMode::Card, 9_000, at(20, 0, 0));
    // Karim never opened a drawer at all, so every sale of his is outside one.
    // It is not Amina's answer, which is the same filter the expected figure
    // needs.
    a_sale(&mut conn, KARIM, PaymentMode::Cash, 60_000, at(10, 0, 0));

    let (from, until) = the_whole_day();
    let hers = shifts::sales_outside_a_shift(&mut conn, SHOP, AMINA, from, until).unwrap();
    let ids: Vec<i32> = hers.sales.iter().map(|s| s.document_id).collect();
    assert_eq!(ids, vec![before, at_the_close, replayed, on_the_card]);
    // 3 000 + 6 000 + 4 000, by hand. The card sale is on the list and not in
    // the figure: a screen names the paper, a drawer holds only the cash.
    assert_eq!(hers.cash, Money::centimes(13_000));
    assert!(!ids.contains(&inside));
    assert!(
        !ids.contains(&on_the_dot),
        "a sale rung at the second the drawer opened belongs to that shift"
    );

    let his = shifts::sales_outside_a_shift(&mut conn, SHOP, KARIM, from, until).unwrap();
    assert_eq!(his.cash, Money::centimes(60_000));

    // The same question one sale at a time, which is the shape the till's own
    // hook asks it in, and with both boundaries on it: `opened_at` is inside
    // the window and `closed_at` is not.
    assert!(!shifts::tag_if_outside_a_shift(&mut conn, SHOP, AMINA, inside, at(10, 0, 0)).unwrap());
    assert!(
        !shifts::tag_if_outside_a_shift(&mut conn, SHOP, AMINA, on_the_dot, at(9, 0, 0)).unwrap(),
        "a sale at the second the drawer opened is inside that shift"
    );
    assert!(
        shifts::tag_if_outside_a_shift(&mut conn, SHOP, AMINA, at_the_close, at(19, 0, 0)).unwrap(),
        "a sale at the second the drawer closed is outside that shift"
    );
    assert!(
        shifts::tag_if_outside_a_shift(&mut conn, SHOP, AMINA, replayed, at(19, 30, 0)).unwrap()
    );
    let mut tagged: Vec<i32> = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .filter(|e| e.action == audit::ACTION_SALE_OUTSIDE_SHIFT)
        .filter_map(|e| e.entity_id)
        .collect();
    tagged.sort_unstable();
    let mut want = vec![at_the_close, replayed];
    want.sort_unstable();
    assert_eq!(
        tagged, want,
        "the two sales inside the shift wrote no row between them"
    );
}

#[test]
fn the_figure_stored_at_the_close_is_a_snapshot_a_later_cancellation_cannot_move() {
    let (_dir, mut conn) = open_temp();
    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(0)).unwrap();
    let ticket = a_sale(&mut conn, AMINA, PaymentMode::Cash, 30_000, at(10, 0, 0));
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 20_000, at(11, 0, 0));
    // 30 000 + 20 000, by hand.
    shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(50_000),
            note: None,
            at: Some(at(19, 0, 0)),
        },
    )
    .unwrap();

    // Three days later somebody annuls the ticket. A derived expected figure
    // would move a count Amina signed on the Monday; the stored one does not.
    cancellation::cancel(
        &mut conn,
        SHOP,
        AMINA,
        ticket,
        "erreur de caisse".to_string(),
        Some(at(19, 30, 0)),
    )
    .unwrap();
    let after = shifts::report(&mut conn, SHOP, shift.id).unwrap();
    assert_eq!(after.expected, Money::centimes(50_000));
    assert_eq!(after.difference, Some(Money::ZERO));
    // The live read of the same window has moved, which is what says the two
    // are different questions rather than two names for one.
    assert_eq!(after.takings.sales, Money::centimes(20_000));
}

#[test]
fn a_float_or_a_count_below_nothing_is_refused_and_a_sum_past_the_range_is_an_error() {
    let (_dir, mut conn) = open_temp();
    let below = shifts::open(
        &mut conn,
        SHOP,
        AMINA,
        NewShift {
            opened_at: at(9, 0, 0),
            opening_cash: Money::centimes(-1),
        },
    );
    assert!(
        matches!(&below, Err(CoreError::Validation { field, .. }) if field == "opening_cash_centimes"),
        "a drawer opened with less than nothing came back as {below:?}"
    );

    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(i64::MAX)).unwrap();
    let counted_below = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(-1),
            note: Some("erreur".to_string()),
            at: Some(at(19, 0, 0)),
        },
    );
    assert!(
        matches!(&counted_below, Err(CoreError::Validation { field, .. }) if field == "counted_centimes"),
        "a drawer counted below nothing came back as {counted_below:?}"
    );

    // A float at the top of the range and one centime of takings. Checked
    // arithmetic answers an error; a bare `+` would have wrapped to a figure
    // the shop is owed money against.
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 100, at(10, 0, 0));
    assert!(shifts::report(&mut conn, SHOP, shift.id).is_err());
    assert!(shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(1_000),
            note: Some("erreur".to_string()),
            at: Some(at(19, 0, 0)),
        },
    )
    .is_err());
    // And the drawer is still open, so nothing was signed against a figure
    // nobody could compute.
    assert!(shifts::open_for(&mut conn, SHOP, AMINA).unwrap().is_some());
}

#[test]
fn a_second_drawer_and_a_second_count_are_both_refused_and_another_person_may_open_one() {
    let (_dir, mut conn) = open_temp();
    a_second_cashier(&mut conn);
    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(100_000)).unwrap();
    let again = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(100_000));
    assert!(
        matches!(&again, Err(CoreError::Conflict { field, .. }) if field == "opened_by"),
        "a second open drawer for one person came back as {again:?}"
    );
    // Overlapping on purpose: each person's cash is physically their own.
    assert!(shifts::open(&mut conn, SHOP, KARIM, opened_at_nine(50_000)).is_ok());

    let count = TillCount {
        counted: Money::centimes(100_000),
        note: None,
        at: Some(at(19, 0, 0)),
    };
    shifts::close(&mut conn, SHOP, shift.id, AMINA, count.clone()).unwrap();
    let twice = shifts::close(&mut conn, SHOP, shift.id, AMINA, count);
    assert!(
        matches!(&twice, Err(CoreError::Conflict { field, .. }) if field == "closed_at"),
        "a till counted twice came back as {twice:?}"
    );
}

#[test]
fn a_till_counted_before_it_was_opened_is_refused() {
    let (_dir, mut conn) = open_temp();
    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(100_000)).unwrap();
    let backwards = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(100_000),
            note: None,
            at: Some(at(8, 0, 0)),
        },
    );
    assert!(
        matches!(&backwards, Err(CoreError::Validation { field, .. }) if field == "closed_at"),
        "a till counted an hour before it opened came back as {backwards:?}"
    );
}

#[test]
fn the_log_holds_the_float_the_count_and_the_opener_when_somebody_else_closed() {
    let (_dir, mut conn) = open_temp();
    a_second_cashier(&mut conn);
    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(100_000)).unwrap();
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 20_000, at(10, 0, 0));
    // 100 000 + 20 000 = 120 000, by hand; counted 5 000 over.
    shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        KARIM,
        TillCount {
            counted: Money::centimes(125_000),
            note: Some("Amina partie, caisse comptée par Karim".to_string()),
            at: Some(at(19, 0, 0)),
        },
    )
    .unwrap();

    let rows = audit::list(&mut conn, SHOP).unwrap();
    let opened = rows
        .iter()
        .find(|e| e.action == audit::ACTION_OPEN_TILL)
        .unwrap();
    assert_eq!(opened.entity, "shift");
    assert_eq!(opened.entity_id, Some(shift.id));
    assert_eq!(opened.user_id, AMINA);
    assert!(opened
        .after
        .as_deref()
        .is_some_and(|a| a.contains("\"opening_cash_centimes\":100000")));

    let closed = rows
        .iter()
        .find(|e| e.action == audit::ACTION_CLOSE_TILL)
        .unwrap();
    // The row is the closer's, and it names the opener because the two differ.
    assert_eq!(closed.user_id, KARIM);
    let after = closed.after.clone().unwrap();
    assert!(
        after.contains("\"expected_at_close_centimes\":120000"),
        "{after}"
    );
    assert!(after.contains("\"counted_centimes\":125000"), "{after}");
    assert!(after.contains("\"difference_centimes\":5000"), "{after}");
    assert!(after.contains(&format!("\"opened_by\":{AMINA}")), "{after}");
}

#[test]
fn a_clean_close_by_the_opener_names_nobody_else() {
    let (_dir, mut conn) = open_temp();
    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(100_000)).unwrap();
    shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(100_000),
            note: None,
            at: Some(at(19, 0, 0)),
        },
    )
    .unwrap();
    let after = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .find(|e| e.action == audit::ACTION_CLOSE_TILL)
        .and_then(|e| e.after)
        .unwrap();
    // Null rather than the closer's own id: the row's `user_id` already says
    // who counted, and the same number twice reads as two people.
    assert!(after.contains("\"opened_by\":null"), "{after}");
    assert!(after.contains("\"note\":null"), "{after}");
}

#[test]
fn takings_for_answers_one_person_over_one_window_and_never_the_shop() {
    // The unit underneath the fixture, asked with its bounds in the open, so
    // the half-open window is pinned where it is written rather than only
    // where a shift happens to use it.
    let (_dir, mut conn) = open_temp();
    a_second_cashier(&mut conn);
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 10_000, at(9, 0, 0));
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 20_000, at(12, 0, 0));
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 40_000, at(14, 0, 0));
    a_sale(&mut conn, KARIM, PaymentMode::Cash, 80_000, at(12, 0, 0));

    let hers = cash::takings_for(&mut conn, SHOP, AMINA, at(9, 0, 0), at(14, 0, 0)).unwrap();
    // 10 000 at the lower bound, which is in; 20 000; and not the 40 000 at
    // the upper bound, which is out. 30 000, by hand.
    assert_eq!(hers.sales, Money::centimes(30_000));
    assert_eq!(hers.total().unwrap(), Money::centimes(30_000));

    let his = cash::takings_for(&mut conn, SHOP, KARIM, at(9, 0, 0), at(14, 0, 0)).unwrap();
    assert_eq!(his.sales, Money::centimes(80_000));

    // An empty window answers zeros rather than nothing.
    let none = cash::takings_for(&mut conn, SHOP, AMINA, at(15, 0, 0), at(16, 0, 0)).unwrap();
    assert_eq!(none.sales, Money::ZERO);
    assert_eq!(none.stamp, Money::ZERO);
    assert_eq!(none.customer_payments, Money::ZERO);
}
