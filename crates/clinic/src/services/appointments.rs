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
//! itself left out of the overlap, and keeps the length the appointment
//! copied when it was booked (C5b): a visit type edited since, or a slot
//! length changed since, does not stretch or shrink a booking by moving it.
//!
//! An appointment the patient will not come to, or did not, is marked with
//! the moment (`mark_no_show`, C5b), kept on the row, and taken back with
//! `clear_no_show`. The desk decides when (Samir, 2026-09-23 19:34): a call
//! can say before the start that the patient won't come, so the mark takes
//! any start, and a marked row is moved or cancelled like any other. A row
//! is never cancelled and missed at once (the table's CHECK): cancelling
//! clears the mark, and marking a cancelled row takes the cancellation
//! back, which puts the slot in the book again and is refused only when
//! another appointment has taken it since.
//!
//! What came of the desk's confirmation call, confirmed or no answer, is
//! recorded on the row with its moment (`record_call`, C6b) and cleared
//! with `clear_call`. It is a note: nothing else about the booking moves
//! because of it.
//!
//! Validation (a 422) is a start the book could never hold: off the grid or
//! in the past. Conflict (a 409) is a start somebody else holds, a cancelled
//! row moved, or an archived file. Every write is one transaction with its
//! audit row.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::{audit, clock, optional_field};

use crate::audit_actions::{
    ACTION_APPOINTMENT_BOOK, ACTION_APPOINTMENT_CALL, ACTION_APPOINTMENT_CANCEL,
    ACTION_APPOINTMENT_MOVE, ACTION_APPOINTMENT_NO_SHOW, ACTION_APPOINTMENT_NO_SHOW_CLEAR,
};
use crate::models::appointment::AppointmentInsert;
use crate::repos::appointments as repo;
use crate::repos::patients as patient_repo;
use crate::services::{absence_blocks, slot_length, visit_types, working_hours};

pub use crate::models::appointment::{Appointment, BookedPatient, CallOutcome};

/// The longest slot the table holds (its CHECK). How far back from a new
/// start a live appointment can begin and still run into it.
const LONGEST_STORED_SLOT_MINUTES: i64 = 240;

/// What the desk asks for. The length is the named visit type's, or the
/// slot-length setting's without one; never a number the caller sends.
#[derive(Debug, Clone, Default)]
pub struct NewAppointment {
    pub patient_id: String,
    pub starts_at: NaiveDateTime,
    pub note: Option<String>,
    /// A visit type of this shop whose length the appointment copies.
    pub visit_type_id: Option<String>,
}

/// Books a patient of this shop at `starts_at`, for the named visit type's
/// length or the slot length in force. Another shop's patient or visit type
/// is not found; an archived file is refused.
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
        let length = match new.visit_type_id.as_deref() {
            Some(id) => visit_types::get(conn, shop_id, id)?.minutes,
            None => slot,
        };
        check_slot(conn, shop_id, new.starts_at, slot, length, now, None)?;
        let row = AppointmentInsert {
            id: uuid::Uuid::now_v7().to_string(),
            shop_id,
            patient_id,
            starts_at: new.starts_at,
            slot_minutes: length,
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
/// can be booked again at once. A no-show mark on it is cleared: the row is
/// one or the other.
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
/// gets, as one update of the same row (see the module doc). It keeps its
/// own length.
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
        let length = before.appointment.slot_minutes;
        check_slot(conn, shop_id, starts_at, slot, length, now, Some(id))?;
        repo::reschedule(conn, shop_id, id, starts_at, length, now)?;
        written(conn, shop_id, user_id, &before, ACTION_APPOINTMENT_MOVE)
    })
}

/// Marks an appointment as one the patient did not or will not come to,
/// stamped now (C5b). Any start, past or not: the desk decides. A cancelled
/// row is taken back into the book by it, so its slot is checked again for
/// another appointment that took it since, the one refusal left. A row
/// already marked is answered as it stands, its first stamp kept and no
/// audit row written: the desk pressing twice changed nothing (Samir,
/// 2026-09-23 19:34).
pub fn mark_no_show(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
) -> Result<BookedPatient, CoreError> {
    let now = clock::now();
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        if before.appointment.no_show_at.is_some() {
            return Ok(before);
        }
        if !before.appointment.is_live() {
            let a = &before.appointment;
            refuse_overlap(conn, shop_id, a.starts_at, a.slot_minutes, Some(id))?;
        }
        repo::set_no_show(conn, shop_id, id, Some(now), now)?;
        written(conn, shop_id, user_id, &before, ACTION_APPOINTMENT_NO_SHOW)
    })
}

/// Takes a no-show mark back. One not marked is answered as it stands,
/// with no audit row, like clearing a call never recorded.
pub fn clear_no_show(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
) -> Result<BookedPatient, CoreError> {
    let now = clock::now();
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        if before.appointment.no_show_at.is_none() {
            return Ok(before);
        }
        repo::set_no_show(conn, shop_id, id, None, now)?;
        written(
            conn,
            shop_id,
            user_id,
            &before,
            ACTION_APPOINTMENT_NO_SHOW_CLEAR,
        )
    })
}

/// Records what came of the confirmation call, stamped now, over any
/// earlier one. Any appointment of this shop: the desk decides when a call
/// is worth writing down.
pub fn record_call(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
    outcome: CallOutcome,
) -> Result<BookedPatient, CoreError> {
    let now = clock::now();
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        repo::set_call(conn, shop_id, id, Some((outcome, now)), now)?;
        written(conn, shop_id, user_id, &before, ACTION_APPOINTMENT_CALL)
    })
}

/// Clears a recorded call. One with none is answered as it stands, with
/// nothing written: there is nothing to refuse.
pub fn clear_call(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
) -> Result<BookedPatient, CoreError> {
    let now = clock::now();
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        if before.appointment.call_outcome.is_none() {
            return Ok(before);
        }
        repo::set_call(conn, shop_id, id, None, now)?;
        written(conn, shop_id, user_id, &before, ACTION_APPOINTMENT_CALL)
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

/// The live appointments sharing a minute with `[from, until)`, in time
/// order, with each patient's names: what an absence block lands on. One
/// that ends at `from` or starts at `until` only touches it.
pub fn overlapping(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Vec<BookedPatient>, CoreError> {
    let since = from
        .checked_sub_signed(Duration::minutes(LONGEST_STORED_SLOT_MINUTES))
        .ok_or_else(out_of_calendar)?;
    Ok(repo::live_between(conn, shop_id, since, until)?
        .into_iter()
        .filter(|b| b.appointment.ends_at().is_none_or(|end| end > from))
        .collect())
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

/// The grid, the clock, the working hours, the absence blocks and the book,
/// in that order: a start the book could never hold is refused before the
/// book is read.
/// `grid` is the slot-length setting every start sits on; `length` is how
/// long this appointment runs.
fn check_slot(
    conn: &mut SqliteConnection,
    shop_id: i32,
    starts_at: NaiveDateTime,
    grid: i32,
    length: i32,
    now: NaiveDateTime,
    except: Option<&str>,
) -> Result<(), CoreError> {
    let minute_of_day = i64::from(starts_at.hour() * 60 + starts_at.minute());
    let on_grid = starts_at.second() == 0
        && starts_at.nanosecond() == 0
        && minute_of_day % i64::from(grid) == 0;
    if !on_grid {
        return Err(CoreError::validation(
            "starts_at",
            &format!("an appointment starts on the {grid}-minute grid counted from midnight"),
        ));
    }
    if starts_at < now {
        return Err(CoreError::validation(
            "starts_at",
            "an appointment cannot be booked in the past",
        ));
    }
    if let Some(week) = working_hours::week(conn, shop_id)? {
        if !working_hours::holds(&week, starts_at, length) {
            return Err(CoreError::validation(
                "starts_at",
                "this time is outside the cabinet's working hours",
            ));
        }
    }
    let until = starts_at
        .checked_add_signed(Duration::minutes(i64::from(length)))
        .ok_or_else(out_of_calendar)?;
    if let Some(block) = absence_blocks::overlapping(conn, shop_id, starts_at, until)?.first() {
        return Err(CoreError::conflict(
            "starts_at",
            &format!(
                "the doctor is away from {} to {}{}",
                block.starts_at.format("%Y-%m-%d %H:%M"),
                block.ends_at.format("%Y-%m-%d %H:%M"),
                block
                    .label
                    .as_deref()
                    .map(|l| format!(" ({l})"))
                    .unwrap_or_default()
            ),
        ));
    }
    refuse_overlap(conn, shop_id, starts_at, length, except)
}

/// A live appointment other than `except` that already holds `starts_at`,
/// or runs into `[starts_at, starts_at + length)`: the double booking.
fn refuse_overlap(
    conn: &mut SqliteConnection,
    shop_id: i32,
    starts_at: NaiveDateTime,
    length: i32,
    except: Option<&str>,
) -> Result<(), CoreError> {
    let until = starts_at
        .checked_add_signed(Duration::minutes(i64::from(length)))
        .ok_or_else(out_of_calendar)?;
    let from = starts_at
        .checked_sub_signed(Duration::minutes(LONGEST_STORED_SLOT_MINUTES))
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
        "no_show_at": a
            .no_show_at
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
        "call_outcome": a.call_outcome.map(CallOutcome::as_str),
        "call_at": a
            .call_at
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
    })
    .to_string()
}
