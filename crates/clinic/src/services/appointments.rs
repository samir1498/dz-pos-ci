//! The appointment book (C5 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! one doctor, a slot length from `services::slot_length`, day and week
//! reads, and a refusal when a second patient takes a slot already held.
//!
//! A start sits on the slot grid counted from midnight, is not in the past
//! on the shop's clock, and does not overlap a live appointment. The overlap
//! is checked against each live row's own `slot_minutes`, not the setting:
//! a 30-minute slot booked at 09:00 before the cabinet moved to 15 minutes
//! still holds 09:15. The unique index on `(shop_id, starts_at)` for live
//! rows is what holds when two desks book the same start at once; it cannot
//! see a partial overlap, which is the service's alone.
//!
//! A move is an update of the live row, not a cancel and a new booking: the
//! id the screen holds stays, the audit has one `appointment.move` row with
//! the start before and after, and the book never shows a cancellation the
//! patient did not ask for. It runs every check a booking runs, with the row
//! itself left out of the overlap, and takes the slot length in force now.
//!
//! Validation (a 422) is a start the book could never hold: off the grid or
//! in the past. Conflict (a 409) is a start somebody else holds, a cancelled
//! row written to again, or an archived file. Every write is one
//! transaction with its audit row.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::{audit, clock, optional_field};

use crate::audit_actions::{
    ACTION_APPOINTMENT_BOOK, ACTION_APPOINTMENT_CANCEL, ACTION_APPOINTMENT_MOVE,
};
use crate::models::appointment::AppointmentInsert;
use crate::repos::appointments as repo;
use crate::repos::patients as patient_repo;
use crate::services::slot_length;

pub use crate::models::appointment::{Appointment, BookedPatient};

/// The longest slot the table holds (its CHECK). How far back from a new
/// start a live appointment can begin and still run into it.
const LONGEST_STORED_SLOT_MINUTES: i64 = 240;

/// What the desk asks for. The slot length is the setting's, never the
/// caller's.
#[derive(Debug, Clone, Default)]
pub struct NewAppointment {
    pub patient_id: String,
    pub starts_at: NaiveDateTime,
    pub note: Option<String>,
}

/// Books a patient of this shop at `starts_at` for the slot length in force.
/// Another shop's patient is not found; an archived file is refused.
pub fn book(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    new: NewAppointment,
) -> Result<BookedPatient, CoreError> {
    let now = clock::now();
    let note = optional_field("note", new.note.as_deref())?;
    conn.transaction(|conn| {
        let patient_id = bookable_patient(conn, shop_id, &new.patient_id)?;
        let slot = current_slot(conn, shop_id)?;
        check_slot(conn, shop_id, new.starts_at, slot, now, None)?;
        let row = AppointmentInsert {
            id: uuid::Uuid::now_v7().to_string(),
            shop_id,
            patient_id,
            starts_at: new.starts_at,
            slot_minutes: slot,
            note,
            created_at: now,
            updated_at: now,
        };
        let made = repo::insert(conn, &row)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: ACTION_APPOINTMENT_BOOK,
                entity: "appointment",
                entity_id: None,
                before: None,
                after: Some(as_json(&made)),
            },
        )?;
        repo::get(conn, shop_id, &made.id)
    })
}

/// Gives a live appointment's slot back. The row stays, stamped; its start
/// can be booked again at once.
pub fn cancel(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
) -> Result<BookedPatient, CoreError> {
    let now = clock::now();
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        refuse_cancelled(&before.appointment)?;
        repo::cancel(conn, shop_id, id, now)?;
        written(conn, shop_id, user_id, &before, ACTION_APPOINTMENT_CANCEL)
    })
}

/// Moves a live appointment to `starts_at`, on the terms a new booking
/// gets, as one update of the same row (see the module doc).
pub fn move_to(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
    starts_at: NaiveDateTime,
) -> Result<BookedPatient, CoreError> {
    let now = clock::now();
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        refuse_cancelled(&before.appointment)?;
        bookable_patient(conn, shop_id, &before.appointment.patient_id)?;
        let slot = current_slot(conn, shop_id)?;
        check_slot(conn, shop_id, starts_at, slot, now, Some(id))?;
        repo::reschedule(conn, shop_id, id, starts_at, slot, now)?;
        written(conn, shop_id, user_id, &before, ACTION_APPOINTMENT_MOVE)
    })
}

/// One appointment of this shop, live or cancelled. Another shop's is not
/// found.
pub fn get(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
) -> Result<BookedPatient, CoreError> {
    repo::get(conn, shop_id, id)
}

/// The live appointments starting on `day`, in time order, with each
/// patient's names. A cancelled one is not in the book any more; its trace
/// is the row itself and its audit row.
pub fn day(
    conn: &mut SqliteConnection,
    shop_id: i32,
    day: NaiveDate,
) -> Result<Vec<BookedPatient>, CoreError> {
    days(conn, shop_id, day, day)
}

/// The live appointments of the Sunday-to-Saturday week holding `any_day`,
/// in time order. The Algerian working week runs Sunday to Thursday, with
/// Friday and Saturday the weekend (Samir, 2026-09-23); no setting in the
/// app names another start, so this one is fixed.
pub fn week(
    conn: &mut SqliteConnection,
    shop_id: i32,
    any_day: NaiveDate,
) -> Result<Vec<BookedPatient>, CoreError> {
    let (first, last) = week_of(any_day)?;
    days(conn, shop_id, first, last)
}

/// The Sunday on or before `day` and the Saturday after it.
pub fn week_of(day: NaiveDate) -> Result<(NaiveDate, NaiveDate), CoreError> {
    let since_sunday = day.weekday().num_days_from_sunday();
    let first = day
        .checked_sub_signed(Duration::days(i64::from(since_sunday)))
        .ok_or_else(out_of_calendar)?;
    let last = first
        .checked_add_signed(Duration::days(6))
        .ok_or_else(out_of_calendar)?;
    Ok((first, last))
}

/// Live appointments starting from `first` at midnight to the end of `last`.
fn days(
    conn: &mut SqliteConnection,
    shop_id: i32,
    first: NaiveDate,
    last: NaiveDate,
) -> Result<Vec<BookedPatient>, CoreError> {
    let until = last.succ_opt().ok_or_else(out_of_calendar)?;
    repo::live_between(
        conn,
        shop_id,
        first.and_time(NaiveTime::MIN),
        until.and_time(NaiveTime::MIN),
    )
}

/// The patient's id when the file is this shop's and not archived.
fn bookable_patient(
    conn: &mut SqliteConnection,
    shop_id: i32,
    patient_id: &str,
) -> Result<String, CoreError> {
    let patient = patient_repo::get(conn, shop_id, patient_id)?;
    if patient.archived_at.is_some() {
        return Err(CoreError::conflict(
            "patient_id",
            "this patient's file is archived; open it again before booking them",
        ));
    }
    Ok(patient.id)
}

fn current_slot(conn: &mut SqliteConnection, shop_id: i32) -> Result<i32, CoreError> {
    let minutes = slot_length::slot_minutes(conn, shop_id)?;
    i32::try_from(minutes).map_err(|_| {
        CoreError::validation(slot_length::SLOT_MINUTES, "the slot length is out of range")
    })
}

/// The grid, the clock and the book, in that order: a start the book could
/// never hold is refused before the table is read.
fn check_slot(
    conn: &mut SqliteConnection,
    shop_id: i32,
    starts_at: NaiveDateTime,
    slot: i32,
    now: NaiveDateTime,
    except: Option<&str>,
) -> Result<(), CoreError> {
    let minute_of_day = i64::from(starts_at.hour() * 60 + starts_at.minute());
    let on_grid = starts_at.second() == 0
        && starts_at.nanosecond() == 0
        && minute_of_day % i64::from(slot) == 0;
    if !on_grid {
        return Err(CoreError::validation(
            "starts_at",
            &format!("an appointment starts on the {slot}-minute grid counted from midnight"),
        ));
    }
    if starts_at < now {
        return Err(CoreError::validation(
            "starts_at",
            "an appointment cannot be booked in the past",
        ));
    }
    let from = starts_at
        .checked_sub_signed(Duration::minutes(LONGEST_STORED_SLOT_MINUTES))
        .ok_or_else(out_of_calendar)?;
    let until = starts_at
        .checked_add_signed(Duration::minutes(i64::from(slot)))
        .ok_or_else(out_of_calendar)?;
    for held in repo::live_starting_between(conn, shop_id, from, until, except)? {
        if held.starts_at == starts_at {
            return Err(CoreError::conflict(
                "starts_at",
                "this slot is already booked",
            ));
        }
        // Past the end of the calendar counts as running into the new slot.
        if held.ends_at().is_none_or(|end| end > starts_at) {
            return Err(CoreError::conflict(
                "starts_at",
                &format!(
                    "this time runs into the {}-minute appointment booked at {}",
                    held.slot_minutes,
                    held.starts_at.format("%H:%M")
                ),
            ));
        }
    }
    Ok(())
}

fn refuse_cancelled(a: &Appointment) -> Result<(), CoreError> {
    if a.is_live() {
        Ok(())
    } else {
        Err(CoreError::conflict(
            "cancelled_at",
            "this appointment was cancelled",
        ))
    }
}

/// Reads the row back after a write and records the audit row for it.
fn written(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    before: &BookedPatient,
    action: &'static str,
) -> Result<BookedPatient, CoreError> {
    let after = repo::get(conn, shop_id, &before.appointment.id)?;
    audit::record(
        conn,
        shop_id,
        user_id,
        audit::Change {
            action,
            entity: "appointment",
            entity_id: None,
            before: Some(as_json(&before.appointment)),
            after: Some(as_json(&after.appointment)),
        },
    )?;
    Ok(after)
}

fn out_of_calendar() -> CoreError {
    CoreError::validation("starts_at", "that date is outside the calendar")
}

/// The appointment as the audit row stores it. No name and no note: the
/// patient's id points at the file, and a reason for a visit is the
/// patient's business, not the log's.
fn as_json(a: &Appointment) -> String {
    serde_json::json!({
        "id": a.id,
        "patient_id": a.patient_id,
        "starts_at": a.starts_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        "slot_minutes": a.slot_minutes,
        "cancelled_at": a
            .cancelled_at
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
    })
    .to_string()
}
