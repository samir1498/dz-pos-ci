// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Check-in, C6b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`:
//! the desk marks a booked patient arrived and the patient joins today's
//! queue, the entry naming the booking. Marking it twice adds nothing, and
//! a patient already queued as a walk-in has that entry linked instead.
//! Appointments are booked for tomorrow, since the book refuses the past
//! and today may have no future slot left when the suite runs late.

use dzpos_clinic::audit_actions::{ACTION_QUEUE_ADD, ACTION_QUEUE_ARRIVE};
use dzpos_clinic::services::{appointments, queue};
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::audit;

mod common;

use common::book::{at, book, conflict, day_ahead, open};
use common::{open_temp, second_shop, OWNER, SHOP};

#[test]
fn a_booked_patient_marked_arrived_joins_todays_queue_with_the_booking_named() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let starts = at(day_ahead(1), 9, 0);
    let booked = book(&mut conn, &benali, starts).unwrap();
    let id = booked.appointment.id.as_str();

    let entry = queue::check_in(&mut conn, SHOP, OWNER, id).unwrap();
    assert_eq!(entry.entry.patient_id, benali.id);
    assert_eq!(entry.entry.appointment_id.as_deref(), Some(id));
    assert_eq!(entry.appointment_starts_at, Some(starts));
    assert!(entry.entry.is_live());

    let today = queue::today(&mut conn, SHOP).unwrap();
    assert_eq!(today.len(), 1);
    assert_eq!(today[0].appointment_starts_at, Some(starts));
    let rows = audit::by_action(&mut conn, SHOP, ACTION_QUEUE_ARRIVE).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0]
        .after
        .as_deref()
        .unwrap()
        .contains(&format!(r#""appointment_id":"{id}""#)));
}

#[test]
fn marking_the_same_booking_arrived_again_adds_no_second_entry_whatever_became_of_the_first() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let booked = book(&mut conn, &benali, at(day_ahead(1), 9, 0)).unwrap();
    let id = booked.appointment.id.as_str();

    let first = queue::check_in(&mut conn, SHOP, OWNER, id).unwrap();
    let again = queue::check_in(&mut conn, SHOP, OWNER, id).unwrap();
    assert_eq!(again, first);
    // Seen, the entry is no longer live, and still the answer.
    queue::call(&mut conn, SHOP, OWNER, &first.entry.id).unwrap();
    queue::mark_seen(&mut conn, SHOP, OWNER, &first.entry.id).unwrap();
    let after_seen = queue::check_in(&mut conn, SHOP, OWNER, id).unwrap();
    assert_eq!(after_seen.entry.id, first.entry.id);
    assert!(after_seen.entry.seen_at.is_some());

    assert_eq!(queue::today(&mut conn, SHOP).unwrap().len(), 1);
    assert_eq!(
        audit::by_action(&mut conn, SHOP, ACTION_QUEUE_ARRIVE)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn a_patient_already_waiting_as_a_walk_in_has_that_entry_linked() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let walk_in = queue::add(&mut conn, SHOP, OWNER, &benali.id).unwrap();
    assert_eq!(walk_in.entry.appointment_id, None);
    assert_eq!(walk_in.appointment_starts_at, None);
    let booked = book(&mut conn, &benali, at(day_ahead(1), 9, 0)).unwrap();

    let linked = queue::check_in(&mut conn, SHOP, OWNER, &booked.appointment.id).unwrap();
    assert_eq!(linked.entry.id, walk_in.entry.id);
    assert_eq!(linked.entry.arrived_at, walk_in.entry.arrived_at);
    assert_eq!(
        linked.entry.appointment_id.as_deref(),
        Some(booked.appointment.id.as_str())
    );
    assert_eq!(queue::today(&mut conn, SHOP).unwrap().len(), 1);
    let rows = audit::by_action(&mut conn, SHOP, ACTION_QUEUE_ARRIVE).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0]
        .before
        .as_deref()
        .unwrap()
        .contains(r#""appointment_id":null"#));
    assert_eq!(
        audit::by_action(&mut conn, SHOP, ACTION_QUEUE_ADD)
            .unwrap()
            .len(),
        1
    );
}

/// The refusals left: a patient waiting under another booking (a second
/// live entry, which the queue always refused), a cancelled booking, and
/// another shop's, which is not found.
#[test]
fn a_second_booking_while_waiting_a_cancelled_one_and_another_shops_are_refused() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let morning = book(&mut conn, &benali, at(day_ahead(1), 9, 0)).unwrap();
    let evening = book(&mut conn, &benali, at(day_ahead(1), 17, 0)).unwrap();
    queue::check_in(&mut conn, SHOP, OWNER, &morning.appointment.id).unwrap();
    let (field, _) = conflict(queue::check_in(
        &mut conn,
        SHOP,
        OWNER,
        &evening.appointment.id,
    ));
    assert_eq!(field, "patient_id");

    let haddad = open(&mut conn, "Karim", "Haddad");
    let gone = book(&mut conn, &haddad, at(day_ahead(2), 9, 0)).unwrap();
    appointments::cancel(&mut conn, SHOP, OWNER, &gone.appointment.id).unwrap();
    let (field, _) = conflict(queue::check_in(
        &mut conn,
        SHOP,
        OWNER,
        &gone.appointment.id,
    ));
    assert_eq!(field, "cancelled_at");

    let other = second_shop(&mut conn);
    match queue::check_in(&mut conn, other, 2, &evening.appointment.id) {
        Err(CoreError::NotFoundText { entity, .. }) => assert_eq!(entity, "appointment"),
        other => panic!("expected not found, got {other:?}"),
    }
    assert_eq!(queue::today(&mut conn, SHOP).unwrap().len(), 1);
}
