// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The no-show mark, item 5 of the book tools (C5b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`),
//! as C6b leaves it: set on any appointment, before its start or after,
//! kept on the row with its moment, and taken back by clearing it. The desk
//! decides (Samir, 2026-09-23 19:34), so a marked row is moved and cancelled
//! like any other; a row is still never cancelled and missed at once, so
//! setting one clears the other. Marking a row already marked, or clearing
//! one never marked, changes nothing and writes no audit row. The book
//! refuses the past, so a past
//! appointment here is a raw row, the way `book_service.rs` makes another
//! shop's.

use chrono::{Duration, NaiveDateTime};
use diesel::{RunQueryDsl, SqliteConnection};
use dzpos_clinic::audit_actions::{ACTION_APPOINTMENT_NO_SHOW, ACTION_APPOINTMENT_NO_SHOW_CLEAR};
use dzpos_clinic::services::appointments;
use dzpos_clinic::services::patients::Patient;
use dzpos_kernel::services::{audit, clock};

mod common;

use common::book::{at, book, conflict, day_ahead, open};
use common::{open_temp, second_shop, OWNER, SHOP};

/// A live appointment of `patient` starting at `starts_at`, written past
/// the service.
fn past(conn: &mut SqliteConnection, id: &str, patient: &Patient, starts_at: NaiveDateTime) {
    raw(conn, id, patient, starts_at, 15);
}

/// An appointment of `minutes` written past the service, whatever its start.
fn raw(
    conn: &mut SqliteConnection,
    id: &str,
    patient: &Patient,
    starts_at: NaiveDateTime,
    minutes: i32,
) {
    diesel::sql_query(format!(
        "INSERT INTO appointments (id, shop_id, patient_id, starts_at, slot_minutes, \
         created_at, updated_at) VALUES ('{id}', 1, '{}', '{}', {minutes}, \
         '2026-09-01 08:00:00', '2026-09-01 08:00:00')",
        patient.id,
        starts_at.format("%Y-%m-%d %H:%M:%S")
    ))
    .execute(conn)
    .unwrap();
}

const PAST: &str = "0199a0c1-0000-7000-8000-0000000e0001";

#[test]
fn a_past_appointment_is_marked_missed_with_the_moment_and_the_mark_is_taken_back() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let yesterday = day_ahead(-1);
    past(&mut conn, PAST, &benali, at(yesterday, 9, 0));
    let before = clock::now();
    let marked = appointments::mark_no_show(&mut conn, SHOP, OWNER, PAST).unwrap();
    let stamp = marked.appointment.no_show_at.unwrap();
    assert!(stamp >= before - Duration::seconds(1) && stamp <= clock::now());
    assert!(marked.appointment.cancelled_at.is_none());
    // Kept on the row, and the day still lists it.
    let read = appointments::day(&mut conn, SHOP, yesterday).unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].appointment.no_show_at, Some(stamp));

    // Pressed again, the mark stands with its first stamp: nothing changed.
    let again = appointments::mark_no_show(&mut conn, SHOP, OWNER, PAST).unwrap();
    assert_eq!(again.appointment.no_show_at, Some(stamp));
    let cleared = appointments::clear_no_show(&mut conn, SHOP, OWNER, PAST).unwrap();
    assert_eq!(cleared.appointment.no_show_at, None);
    let again = appointments::clear_no_show(&mut conn, SHOP, OWNER, PAST).unwrap();
    assert_eq!(again.appointment, cleared.appointment);

    let marks = audit::by_action(&mut conn, SHOP, ACTION_APPOINTMENT_NO_SHOW).unwrap();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].user_id, OWNER);
    assert!(marks[0].after.as_deref().unwrap().contains(&format!(
        r#""no_show_at":"{}""#,
        stamp.format("%Y-%m-%d %H:%M:%S")
    )));
    assert!(marks[0]
        .before
        .as_deref()
        .unwrap()
        .contains(r#""no_show_at":null"#));
    assert_eq!(
        audit::by_action(&mut conn, SHOP, ACTION_APPOINTMENT_NO_SHOW_CLEAR)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn a_future_appointment_is_marked_missed_before_its_start() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let tomorrow = book(&mut conn, &benali, at(day_ahead(1), 9, 0)).unwrap();
    let marked =
        appointments::mark_no_show(&mut conn, SHOP, OWNER, &tomorrow.appointment.id).unwrap();
    assert!(marked.appointment.no_show_at.is_some());
    assert!(marked.appointment.cancelled_at.is_none());
}

#[test]
fn cancelling_a_missed_appointment_clears_the_mark_and_marking_a_cancelled_one_clears_the_cancel() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    past(&mut conn, PAST, &benali, at(day_ahead(-1), 9, 0));
    appointments::mark_no_show(&mut conn, SHOP, OWNER, PAST).unwrap();
    let cancelled = appointments::cancel(&mut conn, SHOP, OWNER, PAST).unwrap();
    assert!(cancelled.appointment.cancelled_at.is_some());
    assert_eq!(cancelled.appointment.no_show_at, None);

    let marked = appointments::mark_no_show(&mut conn, SHOP, OWNER, PAST).unwrap();
    assert!(marked.appointment.no_show_at.is_some());
    assert_eq!(marked.appointment.cancelled_at, None);
    // Back in the book, with its mark.
    let read = appointments::day(&mut conn, SHOP, day_ahead(-1)).unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(
        audit::by_action(&mut conn, SHOP, ACTION_APPOINTMENT_NO_SHOW)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn a_missed_appointment_is_moved_like_any_other() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    past(&mut conn, PAST, &benali, at(day_ahead(-1), 9, 0));
    appointments::mark_no_show(&mut conn, SHOP, OWNER, PAST).unwrap();
    let moved =
        appointments::move_to(&mut conn, SHOP, OWNER, PAST, at(day_ahead(1), 9, 0)).unwrap();
    assert_eq!(moved.appointment.starts_at, at(day_ahead(1), 9, 0));
}

/// Marking a cancelled row puts its slot back in the book, so a slot
/// another appointment took since is the double booking the book refuses.
/// A partial overlap, which the live index alone cannot see.
#[test]
fn a_cancelled_appointment_whose_slot_was_taken_again_is_not_marked_missed() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let haddad = open(&mut conn, "Karim", "Haddad");
    let tomorrow = day_ahead(1);
    raw(&mut conn, PAST, &benali, at(tomorrow, 9, 0), 30);
    appointments::cancel(&mut conn, SHOP, OWNER, PAST).unwrap();
    book(&mut conn, &haddad, at(tomorrow, 9, 15)).unwrap();
    let (field, message) = conflict(appointments::mark_no_show(&mut conn, SHOP, OWNER, PAST));
    assert_eq!(field, "starts_at");
    assert!(message.contains("runs into"), "{message}");
    let still = appointments::get(&mut conn, SHOP, PAST)
        .unwrap()
        .appointment;
    assert!(still.cancelled_at.is_some());
    assert_eq!(still.no_show_at, None);
}

#[test]
fn another_shops_appointment_is_not_found() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    past(&mut conn, PAST, &benali, at(day_ahead(-1), 9, 0));
    let other = second_shop(&mut conn);
    for result in [
        appointments::mark_no_show(&mut conn, other, 2, PAST),
        appointments::clear_no_show(&mut conn, other, 2, PAST),
    ] {
        match result {
            Err(dzpos_kernel::error::CoreError::NotFoundText { entity, .. }) => {
                assert_eq!(entity, "appointment");
            }
            other => panic!("expected not found, got {other:?}"),
        }
    }
    assert_eq!(
        appointments::get(&mut conn, SHOP, PAST)
            .unwrap()
            .appointment
            .no_show_at,
        None
    );
}
