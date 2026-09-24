// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Till shifts (features.md §1, the cash position): a drawer opened with
//! opening cash, counted at close, and the difference between the two.
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
//! opening cash. On a fixture dated today the two columns are an hour apart
//! and the swap survives.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sql_types::Timestamp;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::services::audit;
use dzpos_kernel::services::clock;
use dzpos_retail::audit_actions;
use dzpos_retail::error::CoreError;
use dzpos_retail::money::{Money, PaymentMode};
use dzpos_retail::services::cancellation;
use dzpos_retail::services::cash;
use dzpos_retail::services::customers;
use dzpos_retail::services::debt::{self, DebtKind, NewDebtEntry, PaymentMethod};
use dzpos_retail::services::expenses::{self, NewExpense};
use dzpos_retail::services::shifts::{self, NewShift, TillCount};

mod common;
use common::shifts::{
    a_floor_manager, a_sale, a_sale_in, a_sale_with_stamp, a_second_cashier, at, day, AMINA, KARIM,
    LEILA,
};
use common::{a_fiche, open_temp};

const SHOP: i32 = 1;
/// A second shop on the same file. Every query in the crate is scoped by
/// `shop_id` (rule 3), and a fixture living entirely in shop 1 passes with
/// that filter deleted.
const OTHER_SHOP: i32 = 2;

/// A shift opened and closed clean, at the two moments and the opening cash
/// named:
/// `list`'s own fixture wants several of these and nothing about the reason
/// each closed clean, so this is the shared shape rather than three open/close
/// pairs written out by hand.
fn opened_and_closed(
    conn: &mut SqliteConnection,
    shop: i32,
    user: i32,
    opened_at: NaiveDateTime,
    closed_at: NaiveDateTime,
    centimes: i64,
) -> i32 {
    let made = shifts::open(
        conn,
        shop,
        user,
        NewShift {
            opened_at: Some(opened_at),
            opening_cash: Money::centimes(centimes),
        },
    )
    .unwrap();
    shifts::close(
        conn,
        shop,
        made.id,
        user,
        TillCount {
            counted: Money::centimes(centimes),
            note: None,
            at: Some(closed_at),
        },
    )
    .unwrap();
    made.id
}

/// Midnight opening the fixture day and midnight opening the next: the window
/// the "which sales belonged to no shift" question is asked over.
fn the_whole_day() -> (NaiveDateTime, NaiveDateTime) {
    (
        day().and_hms_opt(0, 0, 0).unwrap(),
        day().succ_opt().unwrap().and_hms_opt(0, 0, 0).unwrap(),
    )
}

/// A second shop on the same file, so a query that dropped its `shop_id` has
/// somewhere to go wrong.
fn a_second_shop(conn: &mut SqliteConnection) {
    diesel::sql_query(format!(
        "INSERT INTO shops (id, name) VALUES ({OTHER_SHOP}, 'Autre magasin')"
    ))
    .execute(conn)
    .unwrap();
}

/// A customer who owes money, so somebody can hand cash over against it.
fn a_debtor(conn: &mut SqliteConnection, owes: i64, when: NaiveDateTime) -> i32 {
    a_debtor_in(conn, SHOP, owes, when)
}

/// The same, on whichever shop's books.
fn a_debtor_in(conn: &mut SqliteConnection, shop_id: i32, owes: i64, when: NaiveDateTime) -> i32 {
    let customer = customers::create(conn, shop_id, AMINA, a_fiche("Entreprise Benali"), None)
        .unwrap()
        .id;
    debt::append_at(
        conn,
        shop_id,
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

/// Cash handed over against what a customer owes. `debt::pay` stamps the row
/// with the moment the money was given and with the user who took it, which is
/// the pair `takings_for` reads.
fn a_debt_payment(
    conn: &mut SqliteConnection,
    user_id: i32,
    customer_id: i32,
    centimes: i64,
    when: NaiveDateTime,
) {
    a_debt_payment_in(conn, SHOP, user_id, customer_id, centimes, when);
}

/// The same, on whichever shop's books.
fn a_debt_payment_in(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    customer_id: i32,
    centimes: i64,
    when: NaiveDateTime,
) {
    debt::pay(
        conn,
        shop_id,
        user_id,
        customer_id,
        Money::centimes(centimes),
        PaymentMethod::Cash,
        None,
        when,
    )
    .unwrap();
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
        opened_at: Some(at(9, 0, 0)),
        opening_cash: Money::centimes(opening_cash),
    }
}

#[test]
fn the_expected_figure_is_this_cashiers_own_cash_and_nothing_else() {
    let (_dir, mut conn) = open_temp();
    a_second_cashier(&mut conn);
    let customer = a_debtor(&mut conn, 500_000, at(8, 0, 0));

    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(500_000)).unwrap();

    // Amina's own cash, inside her window. These four figures and the
    // opening cash are the only ones the expected figure is allowed to be
    // made of.
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 120_000, at(10, 0, 0));
    a_sale_with_stamp(
        &mut conn,
        SHOP,
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

    // Four things that may not reach it. A card sale never touches a drawer;
    // an expense and a supplier payment are the shop's money going out under
    // `commit_money`, which a cashier does not hold; and Karim's cash is
    // Karim's. Karim's sale is the point of the fixture: without it this test
    // passes against a shop-wide sum.
    //
    // Three of the four sit inside the window by their own stamp. The expense
    // does not, and cannot: `expenses::create` takes `expense_date`, a bare
    // date with no hour on it, and the row's own moment is the wall clock of
    // whenever this test ran. An expense is kept out structurally instead —
    // `takings_for` asks `repos::cash` for sales and debt payments and never
    // asks `repos::expenses` anything — so this row proves the shape of the
    // sum and not a bound on it. That is also the plan's reason for leaving
    // expenses out: `expense_date` has no shift-sized slice to ask for.
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

    // 500 000 opening cash + 120 000 + 302 000 (300 000 and the 2 000 stamp the
    // customer handed over with it) + 75 000 against the debt = 997 000.
    // Written out here; never asked of the code. A shop-wide sum over the
    // same window would have answered 1 280 000, which is the figure the
    // assertions below are refusing by reading 997 000 back.
    let expected = Money::centimes(997_000);

    let live = shifts::report(&mut conn, SHOP, shift.id).unwrap();
    assert_eq!(live.expected, expected);
    assert_eq!(live.takings.sales, Money::centimes(422_000));
    assert_eq!(live.takings.stamp, Money::centimes(2_000));
    assert_eq!(live.takings.customer_payments, Money::centimes(75_000));
    assert_eq!(live.difference, None, "nothing has been counted yet");

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
        dzpos_kernel::services::clock::Period::Day(day()),
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

    // 100 000 opening cash + 50 000 = 150 000, by hand.
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
    // Over is a difference too. The rule is "the two figures differ", not
    // "the drawer is light": a count above what was expected is money nobody
    // can account for, the same question from the other side. Without this
    // case the comparison narrows to "counted is less than expected" and
    // ships green.
    let over = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(160_000),
            note: None,
            at: Some(at(19, 0, 0)),
        },
    );
    assert!(
        matches!(&over, Err(CoreError::Validation { field, .. }) if field == "note"),
        "a drawer 10 000 over closed with no reason came back as {over:?}"
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
    // the shift and answer the opening cash.
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
        .filter(|e| e.action == audit_actions::ACTION_SALE_OUTSIDE_SHIFT)
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
            opened_at: Some(at(9, 0, 0)),
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

    // Opening cash at the top of the range and one centime of takings. Checked
    // arithmetic answers an error; a bare `+` would have wrapped to a figure
    // the shop is owed money against.
    //
    // The variant is named rather than left as a bare `is_err`: every refusal
    // above is also an error, so `is_err` alone passes when the sum never
    // happens at all and something else refuses first.
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 100, at(10, 0, 0));
    let live = shifts::report(&mut conn, SHOP, shift.id);
    assert!(
        matches!(&live, Err(CoreError::Money(_))),
        "opening cash at the top of the range plus takings came back as {live:?}"
    );
    let counted = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(1_000),
            note: Some("erreur".to_string()),
            at: Some(at(19, 0, 0)),
        },
    );
    assert!(
        matches!(&counted, Err(CoreError::Money(_))),
        "closing against a sum past the range came back as {counted:?}"
    );
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
    // Leila the floor manager closes, not a second cashier: ruling 10 makes
    // counting a drawer that is not your own manager and owner work, and
    // `shifts::close` refuses a cashier who reaches for one.
    let (_dir, mut conn) = open_temp();
    a_floor_manager(&mut conn);
    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(100_000)).unwrap();
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 20_000, at(10, 0, 0));
    // 100 000 + 20 000 = 120 000, by hand; counted 5 000 over.
    shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        LEILA,
        TillCount {
            counted: Money::centimes(125_000),
            note: Some("Amina partie, caisse comptée par Leila".to_string()),
            at: Some(at(19, 0, 0)),
        },
    )
    .unwrap();

    let rows = audit::list(&mut conn, SHOP).unwrap();
    let opened = rows
        .iter()
        .find(|e| e.action == audit_actions::ACTION_OPEN_TILL)
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
        .find(|e| e.action == audit_actions::ACTION_CLOSE_TILL)
        .unwrap();
    // The row is the closer's, and it names the opener because the two differ.
    assert_eq!(closed.user_id, LEILA);
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
        .find(|e| e.action == audit_actions::ACTION_CLOSE_TILL)
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
    let customer = a_debtor(&mut conn, 500_000, at(8, 0, 0));
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 10_000, at(9, 0, 0));
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 20_000, at(12, 0, 0));
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 40_000, at(14, 0, 0));
    a_sale(&mut conn, KARIM, PaymentMode::Cash, 80_000, at(12, 0, 0));
    // The debt half of the window is its own query on its own column, and it
    // needs its own rows on the two bounds: the sales half above goes on
    // passing with `customer_payments_of`'s `ge`/`lt` inverted.
    a_debt_payment(&mut conn, AMINA, customer, 1_000, at(9, 0, 0));
    a_debt_payment(&mut conn, AMINA, customer, 2_000, at(11, 0, 0));
    a_debt_payment(&mut conn, AMINA, customer, 4_000, at(14, 0, 0));
    a_debt_payment(&mut conn, KARIM, customer, 9_000, at(12, 0, 0));

    let hers = cash::takings_for(&mut conn, SHOP, AMINA, at(9, 0, 0), at(14, 0, 0)).unwrap();
    // 10 000 at the lower bound, which is in; 20 000; and not the 40 000 at
    // the upper bound, which is out. 30 000, by hand.
    assert_eq!(hers.sales, Money::centimes(30_000));
    // 1 000 at the lower bound and 2 000, and not the 4 000 at the upper one.
    // 3 000, by hand.
    assert_eq!(hers.customer_payments, Money::centimes(3_000));
    assert_eq!(hers.total().unwrap(), Money::centimes(33_000));

    let his = cash::takings_for(&mut conn, SHOP, KARIM, at(9, 0, 0), at(14, 0, 0)).unwrap();
    assert_eq!(his.sales, Money::centimes(80_000));
    assert_eq!(his.customer_payments, Money::centimes(9_000));

    // Another shop's rows, written against the same user id, which is the one
    // way to tell a `shop_id` filter from a `user_id` one: a fixture living
    // entirely in shop 1 passes with either of them deleted.
    a_second_shop(&mut conn);
    a_sale_in(
        &mut conn,
        OTHER_SHOP,
        AMINA,
        PaymentMode::Cash,
        700_000,
        at(12, 0, 0),
    );
    let elsewhere = a_debtor_in(&mut conn, OTHER_SHOP, 900_000, at(8, 0, 0));
    a_debt_payment_in(
        &mut conn,
        OTHER_SHOP,
        AMINA,
        elsewhere,
        600_000,
        at(12, 0, 0),
    );
    let still_hers = cash::takings_for(&mut conn, SHOP, AMINA, at(9, 0, 0), at(14, 0, 0)).unwrap();
    assert_eq!(still_hers, hers, "another shop's till is not this one's");

    // An empty window answers zeros rather than nothing.
    let none = cash::takings_for(&mut conn, SHOP, AMINA, at(15, 0, 0), at(16, 0, 0)).unwrap();
    assert_eq!(none.sales, Money::ZERO);
    assert_eq!(none.stamp, Money::ZERO);
    assert_eq!(none.customer_payments, Money::ZERO);
}

#[test]
fn a_sale_rung_while_the_drawer_is_still_open_is_not_tagged() {
    // The open arm of the window: a shift with no `closed_at` has no upper
    // bound, so every moment at or after `opened_at` belongs to it.
    //
    // Its own test because every other case in this file closes the shift
    // first, and a closed shift never takes that arm. Without it the arm can
    // be turned to "an open shift covers nothing" and the suite stays green —
    // and that is the arm the till's own hook asks on every sale rung during
    // a shift, so the log would fill with a tag on each of them.
    let (_dir, mut conn) = open_temp();
    shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(0)).unwrap();
    let during = a_sale(&mut conn, AMINA, PaymentMode::Cash, 5_000, at(10, 0, 0));
    let last_thing = a_sale(&mut conn, AMINA, PaymentMode::Cash, 7_000, at(23, 59, 59));
    let before = a_sale(&mut conn, AMINA, PaymentMode::Cash, 1_000, at(8, 0, 0));

    assert!(
        !shifts::tag_if_outside_a_shift(&mut conn, SHOP, AMINA, during, at(10, 0, 0)).unwrap(),
        "a sale rung while the drawer is open belongs to that shift"
    );
    assert!(
        !shifts::tag_if_outside_a_shift(&mut conn, SHOP, AMINA, last_thing, at(23, 59, 59))
            .unwrap(),
        "an open shift has no upper bound, however late the sale"
    );
    assert!(
        shifts::tag_if_outside_a_shift(&mut conn, SHOP, AMINA, before, at(8, 0, 0)).unwrap(),
        "and it still has a lower one"
    );

    let (from, until) = the_whole_day();
    let hers = shifts::sales_outside_a_shift(&mut conn, SHOP, AMINA, from, until).unwrap();
    let ids: Vec<i32> = hers.sales.iter().map(|s| s.document_id).collect();
    assert_eq!(ids, vec![before]);
    assert_eq!(hers.cash, Money::centimes(1_000));
    // One row, for the one sale that fell outside.
    let tagged = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .filter(|e| e.action == audit_actions::ACTION_SALE_OUTSIDE_SHIFT)
        .count();
    assert_eq!(tagged, 1);
}

#[test]
fn a_new_drawer_that_reaches_back_into_the_last_one_is_refused() {
    // The file cannot refuse this. Its unique index is
    // `WHERE closed_at IS NULL`, so it holds "one drawer open at a time" and
    // says nothing about two closed windows of the same person. Two that
    // overlap each sum the sale in the overlap into their own stored expected
    // figure, and the cashier held that money once.
    let (_dir, mut conn) = open_temp();
    a_second_cashier(&mut conn);
    let first = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(0)).unwrap();
    a_sale(&mut conn, AMINA, PaymentMode::Cash, 40_000, at(11, 30, 0));
    shifts::close(
        &mut conn,
        SHOP,
        first.id,
        AMINA,
        TillCount {
            counted: Money::centimes(40_000),
            note: None,
            at: Some(at(12, 0, 0)),
        },
    )
    .unwrap();

    let reaching_back = shifts::open(
        &mut conn,
        SHOP,
        AMINA,
        NewShift {
            opened_at: Some(at(11, 0, 0)),
            opening_cash: Money::ZERO,
        },
    );
    assert!(
        matches!(&reaching_back, Err(CoreError::Validation { field, .. }) if field == "opened_at"),
        "a drawer opened back inside the last one came back as {reaching_back:?}"
    );
    // The moment of the last count belongs to the shift that was counted, so
    // a new one starting on it would take that second twice.
    let on_the_dot = shifts::open(
        &mut conn,
        SHOP,
        AMINA,
        NewShift {
            opened_at: Some(at(12, 0, 0)),
            opening_cash: Money::ZERO,
        },
    );
    assert!(
        matches!(&on_the_dot, Err(CoreError::Validation { field, .. }) if field == "opened_at"),
        "a drawer opened at the second the last was counted came back as {on_the_dot:?}"
    );
    // Karim is not held to Amina's clock: this is per person, like the index.
    assert!(shifts::open(&mut conn, SHOP, KARIM, opened_at_nine(0)).is_ok());
    // And one second after the count is a new evening.
    assert!(shifts::open(
        &mut conn,
        SHOP,
        AMINA,
        NewShift {
            opened_at: Some(at(12, 0, 1)),
            opening_cash: Money::ZERO,
        },
    )
    .is_ok());
}

#[test]
fn a_till_opened_or_counted_later_than_now_is_refused() {
    // A window whose far end is in the future keeps taking in sales after it
    // was signed, so the expected figure moves under a count somebody already
    // agreed to.
    let (_dir, mut conn) = open_temp();
    let ahead = clock::now() + chrono::Duration::hours(1);
    let opened_ahead = shifts::open(
        &mut conn,
        SHOP,
        AMINA,
        NewShift {
            opened_at: Some(ahead),
            opening_cash: Money::ZERO,
        },
    );
    assert!(
        matches!(&opened_ahead, Err(CoreError::Validation { field, .. }) if field == "opened_at"),
        "a drawer opened an hour from now came back as {opened_ahead:?}"
    );

    let shift = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(0)).unwrap();
    let counted_ahead = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::ZERO,
            note: None,
            at: Some(ahead),
        },
    );
    assert!(
        matches!(&counted_ahead, Err(CoreError::Validation { field, .. }) if field == "closed_at"),
        "a drawer counted an hour from now came back as {counted_ahead:?}"
    );

    // Counted at the second it opened is not the future and is a real
    // evening: a till opened by mistake and shut again holds its opening
    // cash and nothing else.
    let closed = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::ZERO,
            note: None,
            at: Some(at(9, 0, 0)),
        },
    )
    .unwrap();
    let close = closed.close.unwrap();
    assert_eq!(close.closed_at, at(9, 0, 0));
    assert_eq!(close.expected, Money::ZERO);
}

#[test]
fn a_shift_with_no_moment_on_it_takes_the_shops_clock_at_both_ends() {
    // The default both `NewShift::opened_at` and `TillCount::at` exist for,
    // and the one T4's route passes. Nothing else in this file runs it: every
    // other case names its moments, so either `unwrap_or_else(clock::now)`
    // could answer anything at all and the suite would stay green.
    let (_dir, mut conn) = open_temp();
    let before_open = clock::now();
    let shift = shifts::open(
        &mut conn,
        SHOP,
        AMINA,
        NewShift {
            opened_at: None,
            opening_cash: Money::centimes(1_000),
        },
    )
    .unwrap();
    let after_open = clock::now();
    assert!(
        shift.opened_at >= before_open && shift.opened_at <= after_open,
        "{} is not between {before_open} and {after_open}",
        shift.opened_at
    );

    let before_close = clock::now();
    let closed = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(1_000),
            note: None,
            at: None,
        },
    )
    .unwrap();
    let after_close = clock::now();
    let close = closed.close.unwrap();
    assert!(
        close.closed_at >= before_close && close.closed_at <= after_close,
        "{} is not between {before_close} and {after_close}",
        close.closed_at
    );
    // The shop's clock and not the column's own CURRENT_TIMESTAMP, which is
    // UTC while Algiers is an hour ahead.
    let utc = chrono::Utc::now().naive_utc();
    let ahead = (close.closed_at - utc).num_seconds();
    assert!(
        (3500..=3600).contains(&ahead),
        "{} is not an Algerian hour ahead of {utc}",
        close.closed_at
    );
    assert_eq!(close.expected, Money::centimes(1_000));
}

#[test]
fn the_window_sales_outside_a_shift_is_asked_over_has_its_own_two_edges() {
    // `rung_by`'s bounds, which every other case here asks over a whole day
    // and so never touches. Asked with no shift open at all, so what is left
    // in the answer is the window and nothing else.
    let (_dir, mut conn) = open_temp();
    let at_the_start = a_sale(&mut conn, AMINA, PaymentMode::Cash, 1_000, at(10, 0, 0));
    let inside = a_sale(&mut conn, AMINA, PaymentMode::Cash, 2_000, at(11, 0, 0));
    let at_the_end = a_sale(&mut conn, AMINA, PaymentMode::Cash, 4_000, at(12, 0, 0));
    let before = a_sale(&mut conn, AMINA, PaymentMode::Cash, 8_000, at(9, 59, 59));

    let found =
        shifts::sales_outside_a_shift(&mut conn, SHOP, AMINA, at(10, 0, 0), at(12, 0, 0)).unwrap();
    let ids: Vec<i32> = found.sales.iter().map(|s| s.document_id).collect();
    assert_eq!(ids, vec![at_the_start, inside]);
    // 1 000 + 2 000, by hand: the sale at the far edge is out, and so is the
    // one a second before the near one.
    assert_eq!(found.cash, Money::centimes(3_000));
    assert!(!ids.contains(&at_the_end));
    assert!(!ids.contains(&before));
}

#[test]
fn list_answers_a_day_window_newest_first_narrowed_to_this_shop_and_optionally_one_person() {
    // T7's read: a manager's list, row only. `report`'s own takings figure is
    // a query per row and this fixture never reads one back — only which
    // rows come out and in what order.
    let (_dir, mut conn) = open_temp();
    a_second_cashier(&mut conn);

    // Yesterday's, excluded by the window alone: a real, closed shift of
    // this shop's own Amina, so nothing but the day filter drops it.
    let yesterday = day().pred_opt().unwrap();
    let before = opened_and_closed(
        &mut conn,
        SHOP,
        AMINA,
        yesterday.and_hms_opt(8, 0, 0).unwrap(),
        yesterday.and_hms_opt(9, 0, 0).unwrap(),
        0,
    );
    let amina = opened_and_closed(&mut conn, SHOP, AMINA, at(8, 0, 0), at(9, 0, 0), 100_000);
    let karim = opened_and_closed(&mut conn, SHOP, KARIM, at(10, 0, 0), at(11, 0, 0), 50_000);

    // Another shop's shift for the same person: the one way to tell the
    // `shop_id` filter from the `user_id` one (`a_second_shop`'s own reason).
    a_second_shop(&mut conn);
    let elsewhere = shifts::open(
        &mut conn,
        OTHER_SHOP,
        AMINA,
        NewShift {
            opened_at: Some(at(12, 0, 0)),
            opening_cash: Money::ZERO,
        },
    )
    .unwrap()
    .id;

    // Newest first: Karim opened after Amina today, and neither yesterday's
    // shift nor the other shop's leaks past their filters.
    let ids: Vec<i32> = shifts::list(&mut conn, SHOP, day(), day(), None)
        .unwrap()
        .iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(ids, vec![karim, amina]);
    assert!(
        !ids.contains(&before) && !ids.contains(&elsewhere),
        "{ids:?}"
    );

    // Naming a person narrows to that person alone.
    let just_amina: Vec<i32> = shifts::list(&mut conn, SHOP, day(), day(), Some(AMINA))
        .unwrap()
        .iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(just_amina, vec![amina]);

    // Both ends of the window are shop-calendar days and both are included:
    // widening `from` back to yesterday brings the excluded shift back in.
    let wider: Vec<i32> = shifts::list(&mut conn, SHOP, yesterday, day(), None)
        .unwrap()
        .iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(wider, vec![karim, amina, before]);
}
