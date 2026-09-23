// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The confirmation call, C6b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`:
//! the desk records what came of a call, confirmed or no answer, with its
//! moment, and may clear it. A note on the booking; nothing else moves.

use chrono::Duration;
use dzpos_clinic::audit_actions::ACTION_APPOINTMENT_CALL;
use dzpos_clinic::services::appointments::{self, CallOutcome};
use dzpos_clinic::services::patients::{self, NewPatient};
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::permissions::Role;
use dzpos_kernel::services::{audit, clock};

mod common;

use common::book::{at, book, day_ahead, open};
use common::{named, open_temp, second_shop, OWNER, SHOP};

#[test]
fn a_call_is_recorded_with_its_moment_replaced_by_the_next_and_cleared() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let booked = book(&mut conn, &benali, at(day_ahead(1), 9, 0)).unwrap();
    let id = booked.appointment.id.as_str();
    assert_eq!(booked.appointment.call_outcome, None);
    assert_eq!(booked.appointment.call_at, None);

    let before = clock::now();
    let answered =
        appointments::record_call(&mut conn, SHOP, OWNER, id, CallOutcome::NoAnswer).unwrap();
    assert_eq!(
        answered.appointment.call_outcome,
        Some(CallOutcome::NoAnswer)
    );
    let stamp = answered.appointment.call_at.unwrap();
    assert!(stamp >= before - Duration::seconds(1) && stamp <= clock::now());
    // Nothing else about the booking moved.
    assert_eq!(answered.appointment.starts_at, booked.appointment.starts_at);
    assert_eq!(answered.appointment.cancelled_at, None);
    assert_eq!(answered.appointment.no_show_at, None);

    let confirmed =
        appointments::record_call(&mut conn, SHOP, OWNER, id, CallOutcome::Confirmed).unwrap();
    assert_eq!(
        confirmed.appointment.call_outcome,
        Some(CallOutcome::Confirmed)
    );
    let read = appointments::day(&mut conn, SHOP, day_ahead(1)).unwrap();
    assert_eq!(
        read[0].appointment.call_outcome,
        Some(CallOutcome::Confirmed)
    );

    let cleared = appointments::clear_call(&mut conn, SHOP, OWNER, id).unwrap();
    assert_eq!(cleared.appointment.call_outcome, None);
    assert_eq!(cleared.appointment.call_at, None);
    // Clearing again answers the row as it stands and writes nothing.
    let again = appointments::clear_call(&mut conn, SHOP, OWNER, id).unwrap();
    assert_eq!(again, cleared);

    let rows = audit::by_action(&mut conn, SHOP, ACTION_APPOINTMENT_CALL).unwrap();
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().any(|r| r
        .after
        .as_deref()
        .unwrap()
        .contains(r#""call_outcome":"no_answer""#)));
    assert!(rows
        .iter()
        .all(|r| r.user_id == OWNER && r.before.is_some()));
}

/// The book's reads carry the number the desk rings, as the file holds it.
#[test]
fn a_booking_carries_its_patients_phone_for_the_call() {
    let (_dir, mut conn) = open_temp();
    let benali = patients::create(
        &mut conn,
        SHOP,
        OWNER,
        NewPatient {
            phone: Some("0555123456".to_string()),
            ..named("Amina", "Benali")
        },
        Role::Owner,
    )
    .unwrap();
    let haddad = open(&mut conn, "Karim", "Haddad");
    let booked = book(&mut conn, &benali, at(day_ahead(1), 9, 0)).unwrap();
    assert_eq!(booked.phone.as_deref(), Some("0555123456"));
    book(&mut conn, &haddad, at(day_ahead(1), 9, 15)).unwrap();
    let read = appointments::day(&mut conn, SHOP, day_ahead(1)).unwrap();
    let phones: Vec<Option<&str>> = read.iter().map(|b| b.phone.as_deref()).collect();
    assert_eq!(phones, [Some("0555123456"), None]);
}

#[test]
fn another_shops_booking_is_not_found() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let booked = book(&mut conn, &benali, at(day_ahead(1), 9, 0)).unwrap();
    let other = second_shop(&mut conn);
    for result in [
        appointments::record_call(
            &mut conn,
            other,
            2,
            &booked.appointment.id,
            CallOutcome::Confirmed,
        ),
        appointments::clear_call(&mut conn, other, 2, &booked.appointment.id),
    ] {
        match result {
            Err(CoreError::NotFoundText { entity, .. }) => assert_eq!(entity, "appointment"),
            other => panic!("expected not found, got {other:?}"),
        }
    }
}
