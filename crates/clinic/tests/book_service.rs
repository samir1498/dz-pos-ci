// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `services::appointments` and `services::slot_length`, C5 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`:
//! book on the slot grid, never in the past, never over a live slot of any
//! length; cancel, move, read a day or a Sunday-to-Saturday week; never see
//! another shop's. Every expected value is written out here, never read back
//! off the service under test.
//!
//! The book refuses the past on the wall clock, so every case books on days
//! counted from tomorrow.

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use diesel::RunQueryDsl;
use diesel::SqliteConnection;
use dzpos_clinic::audit_actions::{
    ACTION_APPOINTMENT_BOOK, ACTION_APPOINTMENT_CANCEL, ACTION_APPOINTMENT_MOVE,
    ACTION_SLOT_MINUTES_SET,
};
use dzpos_clinic::services::appointments::{self, BookedPatient, NewAppointment};
use dzpos_clinic::services::patients::{self, Patient};
use dzpos_clinic::services::slot_length;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::permissions::Role;
use dzpos_kernel::services::{audit, clock};

mod common;

use common::{named, open_temp, second_shop, OWNER, SHOP};

fn open(conn: &mut SqliteConnection, first: &str, last: &str) -> Patient {
    patients::create(conn, SHOP, OWNER, named(first, last), Role::Owner).unwrap()
}

/// `days` after today on the shop's clock.
fn day_ahead(days: i64) -> NaiveDate {
    clock::now().date() + Duration::days(days)
}

fn at(day: NaiveDate, hh: u32, mm: u32) -> NaiveDateTime {
    day.and_time(NaiveTime::from_hms_opt(hh, mm, 0).unwrap())
}

fn book(
    conn: &mut SqliteConnection,
    shop: i32,
    patient: &Patient,
    starts_at: NaiveDateTime,
) -> Result<BookedPatient, CoreError> {
    let user = if shop == SHOP { OWNER } else { 2 };
    appointments::book(
        conn,
        shop,
        user,
        NewAppointment {
            patient_id: patient.id.clone(),
            starts_at,
            note: None,
        },
    )
}

fn set_slot(conn: &mut SqliteConnection, minutes: u32) {
    assert_eq!(
        slot_length::set_slot_minutes(conn, SHOP, OWNER, minutes).unwrap(),
        minutes
    );
}

/// Each row as `HH:MM Name`, the day part left to the case.
fn read(found: &[BookedPatient]) -> Vec<String> {
    found
        .iter()
        .map(|b| {
            format!(
                "{} {}",
                b.appointment.starts_at.format("%Y-%m-%d %H:%M"),
                b.last_name
            )
        })
        .collect()
}

/// The field and message of a conflict; anything else fails the test.
fn conflict<T: std::fmt::Debug>(result: Result<T, CoreError>) -> (String, String) {
    match result {
        Err(CoreError::Conflict { field, message }) => (field, message),
        other => panic!("expected a conflict, got {other:?}"),
    }
}

fn invalid<T: std::fmt::Debug>(result: Result<T, CoreError>) -> String {
    match result {
        Err(CoreError::Validation { field, .. }) => field,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

fn not_found<T: std::fmt::Debug>(result: Result<T, CoreError>) -> &'static str {
    match result {
        Err(CoreError::NotFoundText { entity, .. }) => entity,
        other => panic!("expected not found, got {other:?}"),
    }
}

fn audit_rows(conn: &mut SqliteConnection, action: &str) -> usize {
    audit::by_action(conn, SHOP, action).unwrap().len()
}

fn rows_in_table(conn: &mut SqliteConnection) -> i64 {
    #[derive(diesel::QueryableByName)]
    struct N {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        n: i64,
    }
    diesel::sql_query("SELECT COUNT(*) AS n FROM appointments")
        .get_result::<N>(conn)
        .unwrap()
        .n
}

#[test]
fn a_booked_slot_is_refused_by_the_service_before_the_index_sees_it() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let haddad = open(&mut conn, "Karim", "Haddad");
    let nine = at(day_ahead(1), 9, 0);
    let made = book(&mut conn, SHOP, &benali, nine).unwrap();
    assert_eq!(made.appointment.slot_minutes, 15);
    assert_eq!(made.appointment.starts_at, nine);
    assert_eq!(made.last_name, "Benali");

    // The service's own words; the index's are "another desk booked this
    // slot at the same moment", so this fails if the service's check goes.
    assert_eq!(
        conflict(book(&mut conn, SHOP, &haddad, nine)),
        (
            "starts_at".to_string(),
            "this slot is already booked".to_string()
        )
    );
    assert_eq!(rows_in_table(&mut conn), 1);
    assert_eq!(audit_rows(&mut conn, ACTION_APPOINTMENT_BOOK), 1);
}

#[test]
fn a_slot_of_another_length_is_refused_where_the_two_overlap() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let haddad = open(&mut conn, "Karim", "Haddad");
    let day = day_ahead(1);

    // A 30-minute slot at 09:00 still holds 09:15 after the cabinet moves
    // to 15 minutes, and frees 09:30.
    set_slot(&mut conn, 30);
    book(&mut conn, SHOP, &benali, at(day, 9, 0)).unwrap();
    set_slot(&mut conn, 15);
    let (field, _) = conflict(book(&mut conn, SHOP, &haddad, at(day, 9, 15)));
    assert_eq!(field, "starts_at");
    let half_past = book(&mut conn, SHOP, &haddad, at(day, 9, 30)).unwrap();
    assert_eq!(half_past.appointment.slot_minutes, 15);

    // And the other way: a 15-minute slot at 11:15 sits inside a 30-minute
    // one asked for at 11:00, though neither start is the other's.
    book(&mut conn, SHOP, &benali, at(day, 11, 15)).unwrap();
    set_slot(&mut conn, 30);
    let (field, message) = conflict(book(&mut conn, SHOP, &haddad, at(day, 11, 0)));
    assert_eq!(field, "starts_at");
    assert_eq!(
        message,
        "this time runs into the 15-minute appointment booked at 11:15"
    );
    // A slot that ends exactly where a live one starts does not run into
    // it: 11:00 for 15 minutes ends at 11:15, the start held above. A read
    // that took in rows starting at the new slot's end would refuse it.
    set_slot(&mut conn, 15);
    let adjacent = book(&mut conn, SHOP, &haddad, at(day, 11, 0)).unwrap();
    assert_eq!(adjacent.appointment.slot_minutes, 15);
    assert_eq!(rows_in_table(&mut conn), 4);
}

/// The overlap check looks back as far as the longest slot the table holds,
/// not the length in force: a 120-minute slot booked at 08:00 still holds
/// 09:15 after the cabinet moves to 15 minutes.
#[test]
fn a_long_slot_booked_earlier_still_holds_its_last_quarter() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let haddad = open(&mut conn, "Karim", "Haddad");
    let day = day_ahead(1);
    set_slot(&mut conn, 120);
    book(&mut conn, SHOP, &benali, at(day, 8, 0)).unwrap();
    set_slot(&mut conn, 15);
    let (field, message) = conflict(book(&mut conn, SHOP, &haddad, at(day, 9, 15)));
    assert_eq!(field, "starts_at");
    assert_eq!(
        message,
        "this time runs into the 120-minute appointment booked at 08:00"
    );
    // 10:00 is where it ends.
    book(&mut conn, SHOP, &haddad, at(day, 10, 0)).unwrap();
}

/// The grid counts minutes from midnight, hours included: with 25-minute
/// slots 09:00 is minute 540, 15 past a start, and 09:10 is minute 550, a
/// start (22 slots of 25).
#[test]
fn a_grid_that_does_not_divide_the_hour_counts_from_midnight() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let day = day_ahead(1);
    set_slot(&mut conn, 25);
    assert_eq!(
        invalid(book(&mut conn, SHOP, &benali, at(day, 9, 0))),
        "starts_at"
    );
    let made = book(&mut conn, SHOP, &benali, at(day, 9, 10)).unwrap();
    assert_eq!(made.appointment.slot_minutes, 25);
}

#[test]
fn a_start_off_the_slot_grid_is_refused() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let day = day_ahead(1);
    // Default 15: 09:10 is between two starts, and 09:15:30 is not a whole
    // minute.
    assert_eq!(
        invalid(book(&mut conn, SHOP, &benali, at(day, 9, 10))),
        "starts_at"
    );
    let with_seconds = at(day, 9, 15) + Duration::seconds(30);
    assert_eq!(
        invalid(book(&mut conn, SHOP, &benali, with_seconds)),
        "starts_at"
    );
    // A 20-minute grid counts from midnight: 09:20 is on it, 09:15 is not.
    set_slot(&mut conn, 20);
    assert_eq!(
        invalid(book(&mut conn, SHOP, &benali, at(day, 9, 15))),
        "starts_at"
    );
    book(&mut conn, SHOP, &benali, at(day, 9, 20)).unwrap();
    assert_eq!(rows_in_table(&mut conn), 1);
}

#[test]
fn a_start_in_the_past_is_refused_on_booking_and_on_a_move() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let yesterday = at(day_ahead(-1), 9, 0);
    assert_eq!(
        invalid(book(&mut conn, SHOP, &benali, yesterday)),
        "starts_at"
    );
    let made = book(&mut conn, SHOP, &benali, at(day_ahead(1), 9, 0)).unwrap();
    assert_eq!(
        invalid(appointments::move_to(
            &mut conn,
            SHOP,
            OWNER,
            &made.appointment.id,
            yesterday
        )),
        "starts_at"
    );
    assert_eq!(rows_in_table(&mut conn), 1);
}

#[test]
fn a_cancelled_slot_can_be_booked_again_and_is_not_cancelled_twice() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let haddad = open(&mut conn, "Karim", "Haddad");
    let day = day_ahead(1);
    let first = book(&mut conn, SHOP, &benali, at(day, 9, 0)).unwrap();
    let id = first.appointment.id;
    let cancelled = appointments::cancel(&mut conn, SHOP, OWNER, &id).unwrap();
    assert!(cancelled.appointment.cancelled_at.is_some());
    assert_eq!(cancelled.appointment.id, id);

    let again = book(&mut conn, SHOP, &haddad, at(day, 9, 0)).unwrap();
    assert_ne!(again.appointment.id, id);
    assert_eq!(
        read(&appointments::day(&mut conn, SHOP, day).unwrap()),
        [format!("{day} 09:00 Haddad")]
    );
    // The cancelled row is still there, and still refuses a second cancel
    // or a move.
    assert!(appointments::get(&mut conn, SHOP, &id)
        .unwrap()
        .appointment
        .cancelled_at
        .is_some());
    let (field, _) = conflict(appointments::cancel(&mut conn, SHOP, OWNER, &id));
    assert_eq!(field, "cancelled_at");
    let (field, _) = conflict(appointments::move_to(
        &mut conn,
        SHOP,
        OWNER,
        &id,
        at(day, 10, 0),
    ));
    assert_eq!(field, "cancelled_at");
    assert_eq!(rows_in_table(&mut conn), 2);
    assert_eq!(audit_rows(&mut conn, ACTION_APPOINTMENT_CANCEL), 1);
}

#[test]
fn a_move_keeps_one_live_row_and_the_same_id() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let haddad = open(&mut conn, "Karim", "Haddad");
    let day = day_ahead(1);
    let made = book(&mut conn, SHOP, &benali, at(day, 9, 0)).unwrap();
    let id = made.appointment.id.clone();
    book(&mut conn, SHOP, &haddad, at(day, 11, 0)).unwrap();

    let moved = appointments::move_to(&mut conn, SHOP, OWNER, &id, at(day, 10, 0)).unwrap();
    assert_eq!(moved.appointment.id, id);
    assert_eq!(moved.appointment.starts_at, at(day, 10, 0));
    assert!(moved.appointment.cancelled_at.is_none());
    assert_eq!(rows_in_table(&mut conn), 2);
    assert_eq!(
        read(&appointments::day(&mut conn, SHOP, day).unwrap()),
        [format!("{day} 10:00 Benali"), format!("{day} 11:00 Haddad")]
    );
    // 09:00 is free again.
    book(&mut conn, SHOP, &haddad, at(day, 9, 0)).unwrap();
    // A move onto somebody else's slot is the booking refusal.
    let (field, _) = conflict(appointments::move_to(
        &mut conn,
        SHOP,
        OWNER,
        &id,
        at(day, 11, 0),
    ));
    assert_eq!(field, "starts_at");
    // A move onto its own start runs into nobody: the row leaves itself out.
    let stayed = appointments::move_to(&mut conn, SHOP, OWNER, &id, at(day, 10, 0)).unwrap();
    assert_eq!(stayed.appointment.starts_at, at(day, 10, 0));
    assert_eq!(audit_rows(&mut conn, ACTION_APPOINTMENT_MOVE), 2);
    assert_eq!(audit_rows(&mut conn, ACTION_APPOINTMENT_CANCEL), 0);
}

#[test]
fn a_day_and_a_week_read_only_their_range_in_time_order() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let haddad = open(&mut conn, "Karim", "Haddad");
    let zidane = open(&mut conn, "Nadia", "Zidane");
    // A Sunday at least two days ahead, so the Saturday before it is in the
    // future too.
    let (sunday, saturday) = appointments::week_of(day_ahead(9)).unwrap();
    assert_eq!(sunday.format("%A").to_string(), "Sunday");
    assert!(sunday > day_ahead(1));
    let monday = sunday + Duration::days(1);

    // Booked later slots first, so the UUID v7 ids sort the other way from
    // the starts.
    book(&mut conn, SHOP, &benali, at(saturday, 16, 0)).unwrap();
    book(&mut conn, SHOP, &haddad, at(sunday, 11, 0)).unwrap();
    book(&mut conn, SHOP, &zidane, at(monday, 10, 0)).unwrap();
    book(&mut conn, SHOP, &zidane, at(sunday, 9, 0)).unwrap();
    // The Saturday before and the Sunday after, outside the week.
    book(
        &mut conn,
        SHOP,
        &benali,
        at(sunday - Duration::days(1), 9, 0),
    )
    .unwrap();
    book(
        &mut conn,
        SHOP,
        &haddad,
        at(sunday + Duration::days(7), 9, 0),
    )
    .unwrap();

    assert_eq!(
        read(&appointments::day(&mut conn, SHOP, sunday).unwrap()),
        [
            format!("{sunday} 09:00 Zidane"),
            format!("{sunday} 11:00 Haddad"),
        ]
    );
    let expected_week = [
        format!("{sunday} 09:00 Zidane"),
        format!("{sunday} 11:00 Haddad"),
        format!("{monday} 10:00 Zidane"),
        format!("{saturday} 16:00 Benali"),
    ];
    // Any day of the week reads the same week.
    for any in [sunday, monday, saturday] {
        assert_eq!(
            read(&appointments::week(&mut conn, SHOP, any).unwrap()),
            expected_week
        );
    }
}

/// The first minute of the week is in it and the first minute of the next
/// is not: Sunday 00:00 and Saturday 23:45 are read, the next Sunday 00:00
/// is not.
#[test]
fn a_week_holds_its_first_minute_and_not_the_next_weeks() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let (sunday, saturday) = appointments::week_of(day_ahead(9)).unwrap();
    assert_eq!(sunday.format("%A").to_string(), "Sunday");
    assert!(sunday > day_ahead(1));
    let next_sunday = sunday + Duration::days(7);
    for starts_at in [
        at(next_sunday, 0, 0),
        at(saturday, 23, 45),
        at(sunday, 0, 0),
    ] {
        book(&mut conn, SHOP, &benali, starts_at).unwrap();
    }
    assert_eq!(
        read(&appointments::week(&mut conn, SHOP, sunday).unwrap()),
        [
            format!("{sunday} 00:00 Benali"),
            format!("{saturday} 23:45 Benali"),
        ]
    );
    assert_eq!(
        read(&appointments::day(&mut conn, SHOP, saturday).unwrap()),
        [format!("{saturday} 23:45 Benali")]
    );
}

/// The Algerian week: Sunday to Thursday worked, Friday and Saturday the
/// weekend, so the book's week runs Sunday to Saturday (Samir, 2026-09-23).
#[test]
fn the_week_starts_on_sunday() {
    // 2026-09-20 is a Sunday, 2026-09-26 the Saturday after it.
    let sun = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
    let sat = NaiveDate::from_ymd_opt(2026, 9, 26).unwrap();
    for day in 20..=26 {
        let any = NaiveDate::from_ymd_opt(2026, 9, day).unwrap();
        assert_eq!(appointments::week_of(any).unwrap(), (sun, sat), "{any}");
    }
    // The Saturday before belongs to the week before, and the next Sunday
    // starts its own.
    let saturday_before = NaiveDate::from_ymd_opt(2026, 9, 19).unwrap();
    assert_eq!(
        appointments::week_of(saturday_before).unwrap().1,
        saturday_before
    );
    let next = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
    assert_eq!(appointments::week_of(next).unwrap().0, next);
}

#[test]
fn another_shops_appointments_and_patients_are_invisible() {
    let (_dir, mut conn) = open_temp();
    let theirs_shop = second_shop(&mut conn);
    let mine = open(&mut conn, "Amina", "Benali");
    let theirs = patients::create(
        &mut conn,
        theirs_shop,
        2,
        named("Karim", "Haddad"),
        Role::Owner,
    )
    .unwrap();
    let day = day_ahead(1);
    let their_booking = book(&mut conn, theirs_shop, &theirs, at(day, 9, 0)).unwrap();
    let id = their_booking.appointment.id.clone();

    assert_eq!(
        not_found(appointments::get(&mut conn, SHOP, &id)),
        "appointment"
    );
    assert_eq!(
        not_found(appointments::cancel(&mut conn, SHOP, OWNER, &id)),
        "appointment"
    );
    assert_eq!(
        not_found(appointments::move_to(
            &mut conn,
            SHOP,
            OWNER,
            &id,
            at(day, 10, 0)
        )),
        "appointment"
    );
    assert_eq!(
        not_found(book(&mut conn, SHOP, &theirs, at(day, 10, 0))),
        "patient"
    );
    // Their 09:00 does not hold mine.
    book(&mut conn, SHOP, &mine, at(day, 9, 0)).unwrap();
    // A row of mine pointing at their patient, which only raw SQL can make:
    // the join scopes the patient by shop too, so it is not read out.
    diesel::sql_query(format!(
        "INSERT INTO appointments (id, shop_id, patient_id, starts_at, slot_minutes, \
         created_at, updated_at) VALUES ('0199a0c1-0000-7000-8000-0000000a0bad', 1, '{}', \
         '{} 12:00:00', 15, '2026-09-23 08:00:00', '2026-09-23 08:00:00')",
        theirs.id, day
    ))
    .execute(&mut conn)
    .unwrap();
    assert_eq!(
        read(&appointments::day(&mut conn, SHOP, day).unwrap()),
        [format!("{day} 09:00 Benali")]
    );
    // Untouched, in its own shop.
    let still = appointments::get(&mut conn, theirs_shop, &id).unwrap();
    assert_eq!(still, their_booking);
}

#[test]
fn an_archived_patient_cannot_be_booked_or_moved() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let day = day_ahead(1);
    let made = book(&mut conn, SHOP, &benali, at(day, 9, 0)).unwrap();
    patients::archive(&mut conn, SHOP, OWNER, &benali.id).unwrap();
    let (field, _) = conflict(book(&mut conn, SHOP, &benali, at(day, 10, 0)));
    assert_eq!(field, "patient_id");
    let (field, _) = conflict(appointments::move_to(
        &mut conn,
        SHOP,
        OWNER,
        &made.appointment.id,
        at(day, 10, 0),
    ));
    assert_eq!(field, "patient_id");
    // Cancelling is still allowed: the file is closed, the slot is freed.
    appointments::cancel(&mut conn, SHOP, OWNER, &made.appointment.id).unwrap();
    assert_eq!(rows_in_table(&mut conn), 1);
}

#[test]
fn the_slot_length_is_5_to_120_in_steps_of_5_and_15_until_set() {
    let (_dir, mut conn) = open_temp();
    assert_eq!(slot_length::slot_minutes(&mut conn, SHOP).unwrap(), 15);
    for refused in [0, 3, 7, 125, 240] {
        assert_eq!(
            invalid(slot_length::set_slot_minutes(
                &mut conn, SHOP, OWNER, refused
            )),
            "slot_minutes",
            "{refused}"
        );
    }
    set_slot(&mut conn, 5);
    set_slot(&mut conn, 120);
    assert_eq!(slot_length::slot_minutes(&mut conn, SHOP).unwrap(), 120);
    // The length in force again writes nothing.
    set_slot(&mut conn, 120);
    assert_eq!(audit_rows(&mut conn, ACTION_SLOT_MINUTES_SET), 2);
    // Another shop still books in fifteens.
    let theirs = second_shop(&mut conn);
    assert_eq!(slot_length::slot_minutes(&mut conn, theirs).unwrap(), 15);
}

/// Every write is recorded under the user who made it, not the owner: a
/// second user of the same cabinet books, moves and cancels.
#[test]
fn a_note_is_trimmed_and_kept_and_a_blank_one_is_none() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let day = day_ahead(1);
    let mut new = NewAppointment {
        patient_id: benali.id.clone(),
        starts_at: at(day, 9, 0),
        note: Some("  contrôle  ".to_string()),
    };
    let made = appointments::book(&mut conn, SHOP, OWNER, new.clone()).unwrap();
    assert_eq!(made.appointment.note.as_deref(), Some("contrôle"));
    new.starts_at = at(day, 9, 15);
    new.note = Some("   ".to_string());
    let blank = appointments::book(&mut conn, SHOP, OWNER, new).unwrap();
    assert_eq!(blank.appointment.note, None);
}

#[test]
fn the_audit_names_the_user_who_booked() {
    let (_dir, mut conn) = open_temp();
    diesel::sql_query(
        "INSERT INTO users (id, shop_id, name, role) VALUES (3, 1, 'Secrétaire', 'cashier')",
    )
    .execute(&mut conn)
    .unwrap();
    let desk = 3;
    let benali = open(&mut conn, "Amina", "Benali");
    let day = day_ahead(1);
    let made = appointments::book(
        &mut conn,
        SHOP,
        desk,
        NewAppointment {
            patient_id: benali.id.clone(),
            starts_at: at(day, 9, 0),
            note: None,
        },
    )
    .unwrap();
    let id = made.appointment.id;
    appointments::move_to(&mut conn, SHOP, desk, &id, at(day, 9, 15)).unwrap();
    appointments::cancel(&mut conn, SHOP, desk, &id).unwrap();
    for action in [
        ACTION_APPOINTMENT_BOOK,
        ACTION_APPOINTMENT_MOVE,
        ACTION_APPOINTMENT_CANCEL,
    ] {
        let rows = audit::by_action(&mut conn, SHOP, action).unwrap();
        assert_eq!(
            rows.iter().map(|r| r.user_id).collect::<Vec<_>>(),
            [desk],
            "{action}"
        );
    }
}
