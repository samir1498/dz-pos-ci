// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The count of sales rung with no drawer open (features.md §1, the till
//! shifts): a sale tagged `till.sale_outside_shift` is counted at the next
//! drawer that person opens, over the stretch since their last close, or
//! since midnight of the day when they have never closed. Split off
//! `shifts_service.rs` when that file passed the size limit; the fixture
//! and the expected figures follow the same rule as there, written by hand.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::audit_actions;
use dzpos_core::money::{Money, PaymentMode};
use dzpos_core::services::shifts::{self, NewShift, TillCount};

mod common;
use common::open_temp;
use common::shifts::{a_sale, at, day, AMINA};

const SHOP: i32 = 1;

/// The fixture day's drawer, opened at 09:00 with the float given.
fn opened_at_nine(opening_cash: i64) -> NewShift {
    NewShift {
        opened_at: Some(at(9, 0, 0)),
        opening_cash: Money::centimes(opening_cash),
    }
}

/// Rings a sale this person held no drawer for, and dates the tag the log
/// keeps for it.
///
/// Two steps, and the second one is not decoration. `audit::record` stamps
/// `created_at` from `services::clock::now`, which is the real wall clock
/// and has nothing injecting it — the same gap the two wall-clock gates sit
/// on — so a row written on a fixture dated 2026-09-14 lands today whatever
/// `issued_at` says, and no arrangement of the fixture can put it inside the
/// window under test. The row is still the one `tag_if_outside_a_shift`
/// actually writes, action and entity and user all its, named by the same
/// constant the service names; only the moment is moved.
fn a_sale_rung_with_no_drawer(
    conn: &mut SqliteConnection,
    user_id: i32,
    issued_at: NaiveDateTime,
) -> i32 {
    let document_id = a_sale(conn, user_id, PaymentMode::Cash, 100_000, issued_at);
    assert!(
        shifts::tag_if_outside_a_shift(conn, SHOP, user_id, document_id, issued_at).unwrap(),
        "the fixture rang a sale inside a drawer it meant to be outside"
    );
    let stamped = issued_at.format("%Y-%m-%d %H:%M:%S");
    let action = audit_actions::ACTION_SALE_OUTSIDE_SHIFT;
    diesel::sql_query(format!(
        "UPDATE audit_log SET created_at = '{stamped}' \
         WHERE action = '{action}' AND entity_id = {document_id}"
    ))
    .execute(conn)
    .unwrap();
    document_id
}

/// A sale rung between an evening close and midnight is counted at the next
/// morning's drawer, because that morning's stretch reaches back to the
/// close rather than to midnight.
///
/// This is the case the old floor lost. `max(midnight, last close)` put the
/// start of the morning's stretch at 00:00, after the 20:00 sale; the
/// evening's own report had ended at its 18:00 close, before it. So the sale
/// sat in the gap between two reports and was counted by neither, which is
/// the one outcome this figure exists to prevent — the cash was in the
/// drawer somebody counted the next day.
#[test]
fn a_sale_rung_after_the_evening_close_is_counted_at_the_next_mornings_drawer() {
    let (_dir, mut conn) = open_temp();

    // The fixture day: opened at 09:00, counted at 18:00 and gone home.
    let evening = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(500_000)).unwrap();
    shifts::close(
        &mut conn,
        SHOP,
        evening.id,
        AMINA,
        TillCount {
            counted: Money::centimes(500_000),
            note: None,
            at: Some(at(18, 0, 0)),
        },
    )
    .unwrap();

    // Then somebody rings one at 20:00 with the drawer shut.
    a_sale_rung_with_no_drawer(&mut conn, AMINA, at(20, 0, 0));

    // The next morning's drawer, 08:00 to 18:00.
    let next = day().succ_opt().unwrap();
    let morning = shifts::open(
        &mut conn,
        SHOP,
        AMINA,
        NewShift {
            opened_at: Some(next.and_hms_opt(8, 0, 0).unwrap()),
            opening_cash: Money::centimes(500_000),
        },
    )
    .unwrap();
    shifts::close(
        &mut conn,
        SHOP,
        morning.id,
        AMINA,
        TillCount {
            counted: Money::centimes(500_000),
            note: None,
            at: Some(next.and_hms_opt(18, 0, 0).unwrap()),
        },
    )
    .unwrap();

    let report = shifts::report(&mut conn, SHOP, morning.id).unwrap();
    assert_eq!(
        report.rung_outside_shift, 1,
        "the sale rung at 20:00 with the drawer shut was counted by nobody"
    );

    // And it is not a term in the arithmetic: the expected figure is the
    // float alone, because that sale fell outside this shift's own window.
    assert_eq!(report.expected, Money::centimes(500_000));
}

/// The floor of the stretch, the other end of the test above: a sale already
/// counted at one drawer is not counted again at the next one.
///
/// The count reaches back to this person's last close and no further. Without
/// that floor every drawer would carry every untagged sale that person ever
/// rang, and the figure would climb for the rest of the shop's life — a
/// number that only ever grows is one nobody reads twice.
#[test]
fn a_tagged_sale_before_the_last_close_is_not_counted_at_the_next_drawer() {
    let (_dir, mut conn) = open_temp();

    // Rung at 07:00 with the shutters up and no drawer open yet.
    a_sale_rung_with_no_drawer(&mut conn, AMINA, at(7, 0, 0));

    // The day's drawer, 09:00 to 18:00. Its stretch starts at midnight —
    // nothing has ever been closed — so the 07:00 sale is this drawer's to
    // answer for, which is what makes the next assertion mean something: the
    // sale was counted once, here.
    let today = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(500_000)).unwrap();
    shifts::close(
        &mut conn,
        SHOP,
        today.id,
        AMINA,
        TillCount {
            counted: Money::centimes(500_000),
            note: None,
            at: Some(at(18, 0, 0)),
        },
    )
    .unwrap();
    assert_eq!(
        shifts::report(&mut conn, SHOP, today.id)
            .unwrap()
            .rung_outside_shift,
        1,
        "the 07:00 sale belongs to the drawer whose stretch covers it"
    );

    // The next morning's drawer, 08:00 to 18:00. Its stretch starts at
    // yesterday's 18:00 close, which is after the 07:00 sale.
    let next = day().succ_opt().unwrap();
    let tomorrow = shifts::open(
        &mut conn,
        SHOP,
        AMINA,
        NewShift {
            opened_at: Some(next.and_hms_opt(8, 0, 0).unwrap()),
            opening_cash: Money::centimes(500_000),
        },
    )
    .unwrap();
    shifts::close(
        &mut conn,
        SHOP,
        tomorrow.id,
        AMINA,
        TillCount {
            counted: Money::centimes(500_000),
            note: None,
            at: Some(next.and_hms_opt(18, 0, 0).unwrap()),
        },
    )
    .unwrap();
    assert_eq!(
        shifts::report(&mut conn, SHOP, tomorrow.id)
            .unwrap()
            .rung_outside_shift,
        0,
        "yesterday's tagged sale was counted a second time at this drawer"
    );
}

/// The midnight fallback and its two edges: somebody who has never closed a
/// drawer answers for the day they are in, from 00:00:00 inclusive, and for
/// nothing before it.
///
/// `rung_outside_a_shift` falls back to `opened_at.date().and_time(MIN)` and
/// `repos::audit` filters `created_at >= from`, so midnight itself is inside
/// the stretch and the second before it is outside. A sale at 23:59:59 the
/// night before is yesterday's business; a sale at 00:00:00 is today's, and a
/// count that skipped it would lose it for good, because this person has no
/// earlier close for a later stretch to reach back to.
#[test]
fn the_midnight_floor_takes_the_stroke_of_midnight_and_not_the_second_before_it() {
    let (_dir, mut conn) = open_temp();

    // 23:59:59 the night before: outside the stretch.
    let yesterday = day().pred_opt().unwrap();
    a_sale_rung_with_no_drawer(&mut conn, AMINA, yesterday.and_hms_opt(23, 59, 59).unwrap());
    // 00:00:00 on the fixture day: the first moment inside it.
    a_sale_rung_with_no_drawer(&mut conn, AMINA, day().and_hms_opt(0, 0, 0).unwrap());

    // Her first drawer ever, so there is no close for the stretch to start
    // at and midnight is the floor.
    let first = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(500_000)).unwrap();
    shifts::close(
        &mut conn,
        SHOP,
        first.id,
        AMINA,
        TillCount {
            counted: Money::centimes(500_000),
            note: None,
            at: Some(at(18, 0, 0)),
        },
    )
    .unwrap();

    assert_eq!(
        shifts::report(&mut conn, SHOP, first.id)
            .unwrap()
            .rung_outside_shift,
        1,
        "the stretch takes the sale at 00:00:00 and leaves the one at 23:59:59"
    );
}
