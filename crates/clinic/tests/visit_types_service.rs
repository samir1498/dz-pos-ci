// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `services::visit_types` and a booking that names one, item 3 of the book
//! tools (C5b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! the type's length becomes the appointment's, the grid stays the slot
//! setting, the overlap reads real intervals, and an appointment keeps the
//! length it copied. Every expected value is written out here.

use chrono::Weekday;
use diesel::SqliteConnection;
use dzpos_clinic::audit_actions::{
    ACTION_VISIT_TYPE_CREATE, ACTION_VISIT_TYPE_REMOVE, ACTION_VISIT_TYPE_UPDATE,
};
use dzpos_clinic::services::appointments::{self, BookedPatient, NewAppointment};
use dzpos_clinic::services::patients::Patient;
use dzpos_clinic::services::slot_length;
use dzpos_clinic::services::visit_types::{self, NewVisitType, VisitType};
use dzpos_clinic::services::working_hours;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::audit;

mod common;

use common::book::{at, book, conflict, day_ahead, invalid, next, open, range};
use common::{open_temp, second_shop, OWNER, SHOP};

fn kind(conn: &mut SqliteConnection, name: &str, minutes: i32) -> Result<VisitType, CoreError> {
    visit_types::create(
        conn,
        SHOP,
        OWNER,
        NewVisitType {
            name: name.to_string(),
            minutes,
        },
    )
}

fn book_as(
    conn: &mut SqliteConnection,
    patient: &Patient,
    starts_at: chrono::NaiveDateTime,
    visit: &VisitType,
) -> Result<BookedPatient, CoreError> {
    appointments::book(
        conn,
        SHOP,
        OWNER,
        NewAppointment {
            patient_id: patient.id.clone(),
            starts_at,
            note: None,
            visit_type_id: Some(visit.id.clone()),
        },
    )
}

#[test]
fn a_typed_booking_runs_the_types_length_and_holds_every_minute_of_it() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let haddad = open(&mut conn, "Karim", "Haddad");
    let first = kind(&mut conn, "Première consultation", 45).unwrap();
    let day = day_ahead(1);
    let made = book_as(&mut conn, &benali, at(day, 9, 0), &first).unwrap();
    assert_eq!(made.appointment.slot_minutes, 45);
    // 09:15 and 09:30 start inside 09:00-09:45; the grid is still 15.
    for inside in [at(day, 9, 15), at(day, 9, 30)] {
        let (field, message) = conflict(book(&mut conn, &haddad, inside));
        assert_eq!(field, "starts_at");
        assert_eq!(
            message,
            "this time runs into the 45-minute appointment booked at 09:00"
        );
    }
    let plain = book(&mut conn, &haddad, at(day, 9, 45)).unwrap();
    assert_eq!(plain.appointment.slot_minutes, 15);
    // A 45-minute visit at 08:30 would run into 09:00; at 08:15 it ends on it.
    conflict(book_as(&mut conn, &haddad, at(day, 8, 30), &first));
    book_as(&mut conn, &haddad, at(day, 8, 15), &first).unwrap();
    // The start is on the slot grid, not on a 45-minute one.
    let (field, _) = invalid(book_as(&mut conn, &haddad, at(day, 10, 7), &first));
    assert_eq!(field, "starts_at");
    book_as(&mut conn, &haddad, at(day, 10, 15), &first).unwrap();
}

#[test]
fn a_long_visit_type_that_would_span_the_lunch_break_is_refused() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let day = vec![range((8, 0), (12, 0)), range((14, 0), (18, 0))];
    let mut week = vec![day; 5];
    week.extend([vec![], vec![]]);
    working_hours::set_week(&mut conn, SHOP, OWNER, week).unwrap();
    let long = kind(&mut conn, "Bilan", 180).unwrap();
    let sunday = next(Weekday::Sun);
    // 11:00 to 14:00 starts and ends inside the day's two ranges and still
    // crosses the break between them.
    let (field, message) = invalid(book_as(&mut conn, &benali, at(sunday, 11, 0), &long));
    assert_eq!(field, "starts_at");
    assert_eq!(message, "this time is outside the cabinet's working hours");
    invalid(book_as(&mut conn, &benali, at(sunday, 10, 0), &long));
    book_as(&mut conn, &benali, at(sunday, 9, 0), &long).unwrap();
    book_as(&mut conn, &benali, at(sunday, 15, 0), &long).unwrap();
}

#[test]
fn an_appointment_keeps_its_length_when_the_type_changes_or_it_is_moved() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let haddad = open(&mut conn, "Karim", "Haddad");
    let control = kind(&mut conn, "Contrôle", 30).unwrap();
    let day = day_ahead(1);
    let made = book_as(&mut conn, &benali, at(day, 9, 0), &control).unwrap();
    let longer = visit_types::update(
        &mut conn,
        SHOP,
        OWNER,
        &control.id,
        NewVisitType {
            name: "Contrôle long".into(),
            minutes: 60,
        },
    )
    .unwrap();
    assert_eq!(
        (longer.name.as_str(), longer.minutes),
        ("Contrôle long", 60)
    );
    let id = made.appointment.id;
    assert_eq!(
        appointments::get(&mut conn, SHOP, &id)
            .unwrap()
            .appointment
            .slot_minutes,
        30
    );
    // Moved, it stays 30 minutes long: 10:30 is free again after it.
    let moved = appointments::move_to(&mut conn, SHOP, OWNER, &id, at(day, 10, 0)).unwrap();
    assert_eq!(moved.appointment.slot_minutes, 30);
    book(&mut conn, &haddad, at(day, 10, 30)).unwrap();
    // A plain booking keeps its 15 through a move after the grid changes.
    let plain = book(&mut conn, &haddad, at(day, 12, 0)).unwrap();
    assert_eq!(
        slot_length::set_slot_minutes(&mut conn, SHOP, OWNER, 20).unwrap(),
        20
    );
    let moved = appointments::move_to(
        &mut conn,
        SHOP,
        OWNER,
        &plain.appointment.id,
        at(day, 13, 20),
    )
    .unwrap();
    assert_eq!(moved.appointment.slot_minutes, 15);
    // The type now books 60.
    let again = book_as(&mut conn, &benali, at(day, 15, 0), &longer).unwrap();
    assert_eq!(again.appointment.slot_minutes, 60);
    // Removed, it books nothing more and its bookings stay as they are.
    let gone = visit_types::remove(&mut conn, SHOP, OWNER, &control.id).unwrap();
    assert_eq!(gone, longer);
    match book_as(&mut conn, &benali, at(day, 17, 0), &longer) {
        Err(CoreError::NotFoundText { entity, .. }) => assert_eq!(entity, "visit_type"),
        other => panic!("expected not found, got {other:?}"),
    }
    assert_eq!(
        appointments::get(&mut conn, SHOP, &id)
            .unwrap()
            .appointment
            .slot_minutes,
        30
    );
}

#[test]
fn a_type_is_5_to_240_minutes_on_the_grid_with_a_short_unique_name() {
    let (_dir, mut conn) = open_temp();
    for minutes in [0, 10, 20, 250, 255, -15] {
        let (field, _) = invalid(kind(&mut conn, "Contrôle", minutes));
        assert_eq!(field, "minutes", "{minutes}");
    }
    for name in ["", "   ", &"x".repeat(61)] {
        let (field, _) = invalid(kind(&mut conn, name, 30));
        assert_eq!(field, "name", "{name:?}");
    }
    assert_eq!(
        audit::by_action(&mut conn, SHOP, ACTION_VISIT_TYPE_CREATE)
            .unwrap()
            .len(),
        0
    );
    let edge = kind(&mut conn, "  Certificat  ", 15).unwrap();
    assert_eq!(edge.name, "Certificat");
    kind(&mut conn, &"é".repeat(60), 240).unwrap();
    let (field, _) = conflict(kind(&mut conn, "certificat", 30));
    assert_eq!(field, "name");
    // On a 20-minute grid 30 no longer fits and 40 does; the 15 written
    // before stays.
    assert_eq!(
        slot_length::set_slot_minutes(&mut conn, SHOP, OWNER, 20).unwrap(),
        20
    );
    invalid(kind(&mut conn, "Suivi", 30));
    kind(&mut conn, "Suivi", 40).unwrap();
    let names: Vec<(String, i32)> = visit_types::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .map(|t| (t.name, t.minutes))
        .collect();
    assert_eq!(
        names,
        [
            ("Certificat".to_string(), 15),
            ("Suivi".to_string(), 40),
            ("é".repeat(60), 240)
        ]
    );
    let (field, _) = invalid(visit_types::update(
        &mut conn,
        SHOP,
        OWNER,
        &edge.id,
        NewVisitType {
            name: "Certificat".into(),
            minutes: 30,
        },
    ));
    assert_eq!(field, "minutes");
    // On the 5-minute grid the floor itself is a type; below it, none.
    assert_eq!(
        slot_length::set_slot_minutes(&mut conn, SHOP, OWNER, 5).unwrap(),
        5
    );
    assert_eq!(kind(&mut conn, "Vaccin", 5).unwrap().minutes, 5);
    let (field, _) = invalid(kind(&mut conn, "Pansement", 0));
    assert_eq!(field, "minutes");
}

#[test]
fn the_audit_names_each_write_and_another_shops_types_are_not_found() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let made = kind(&mut conn, "Contrôle", 30).unwrap();
    visit_types::update(
        &mut conn,
        SHOP,
        OWNER,
        &made.id,
        NewVisitType {
            name: "Contrôle".into(),
            minutes: 45,
        },
    )
    .unwrap();
    let rows = audit::by_action(&mut conn, SHOP, ACTION_VISIT_TYPE_UPDATE).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].before.as_deref(),
        Some(format!(r#"{{"id":"{}","minutes":30,"name":"Contrôle"}}"#, made.id).as_str())
    );
    assert_eq!(
        rows[0].after.as_deref(),
        Some(format!(r#"{{"id":"{}","minutes":45,"name":"Contrôle"}}"#, made.id).as_str())
    );

    let other = second_shop(&mut conn);
    let theirs = visit_types::create(
        &mut conn,
        other,
        2,
        NewVisitType {
            name: "Contrôle".into(),
            minutes: 30,
        },
    )
    .unwrap();
    assert_eq!(visit_types::list(&mut conn, SHOP).unwrap().len(), 1);
    for result in [
        visit_types::get(&mut conn, SHOP, &theirs.id).map(|_| ()),
        visit_types::remove(&mut conn, SHOP, OWNER, &theirs.id).map(|_| ()),
        book_as(&mut conn, &benali, at(day_ahead(1), 9, 0), &theirs).map(|_| ()),
    ] {
        match result {
            Err(CoreError::NotFoundText { entity, .. }) => assert_eq!(entity, "visit_type"),
            other => panic!("expected not found, got {other:?}"),
        }
    }
    visit_types::remove(&mut conn, SHOP, OWNER, &made.id).unwrap();
    assert_eq!(
        audit::by_action(&mut conn, SHOP, ACTION_VISIT_TYPE_REMOVE)
            .unwrap()
            .len(),
        1
    );
}
