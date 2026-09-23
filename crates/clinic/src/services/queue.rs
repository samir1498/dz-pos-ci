//! The waiting queue (C4 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! a patient is added on arrival, called in (the next in line, or one the
//! doctor picks out of order), then either seen or gone unseen.
//!
//! The line is the desk's order (C6b, Samir 2026-09-23 19:32): a new
//! arrival goes at the end, and `reorder` puts the day in whatever order
//! the desk drags it into, a booked patient who came early wherever they
//! are placed. The next patient called is the first of that order still
//! waiting. No rule of the software moves anybody. One day at a
//! time, on the shop's clock: today's list is today's arrivals, and an entry
//! left live last night does not stand in this morning's line.
//!
//! A booked patient comes in through `check_in` (C6b): the desk marks the
//! appointment arrived, and the patient joins today's queue with the entry
//! naming the booking. Marking it twice adds nothing; a patient the desk
//! already queued as a walk-in has that entry linked instead of a second
//! one. The appointment itself is not written: arrived is read off the
//! queue.
//!
//! Every refusal is a conflict naming the column that refuses it
//! (`patient_id`, `called_at`, `seen_at`, `left_at`, `day`), or `queue` when
//! nobody is waiting, the way the patient service names `archived_at`.
//! Every write is one transaction with its audit row, through the kernel's
//! `services::audit::record`; the entry's id travels in the JSON.

use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::{audit, clock};

use crate::audit_actions::{
    ACTION_QUEUE_ADD, ACTION_QUEUE_ARRIVE, ACTION_QUEUE_CALL, ACTION_QUEUE_LEFT,
    ACTION_QUEUE_REORDER, ACTION_QUEUE_SEEN,
};
use crate::models::patient::Patient;
use crate::models::queue_entry::QueueEntryInsert;
use crate::repos::appointments as appointment_repo;
use crate::repos::patients as patient_repo;
use crate::repos::queue::{self as repo, Moment};

pub use crate::models::queue_entry::{QueueEntry, QueuedPatient};

/// Adds a patient of this shop to today's queue, arriving now. Another
/// shop's patient is not found, the same answer as an id nobody made; an
/// archived file is refused, and so is a second entry while the first is
/// still live today. Once that one is seen or gone, the patient may be
/// queued again.
pub fn add(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    patient_id: &str,
) -> Result<QueuedPatient, CoreError> {
    let now = clock::now();
    let day = now.date();
    conn.transaction(|conn| {
        let patient = queueable_patient(conn, shop_id, patient_id)?;
        if repo::live_for(conn, shop_id, &patient.id, day)?.is_some() {
            return Err(repo::already_waiting());
        }
        let made = arrive(
            conn,
            (shop_id, user_id),
            patient,
            None,
            ACTION_QUEUE_ADD,
            now,
        )?;
        repo::get(conn, shop_id, &made.id)
    })
}

/// Marks a booked patient arrived: today's queue gets an entry naming the
/// appointment, arriving now. Idempotent on the appointment: when it
/// already has an entry, that entry is the answer and nothing is written.
/// When the patient is already waiting today as a walk-in, that entry is
/// linked to the appointment rather than a second one added; waiting under
/// another booking is refused, as a second live entry is. A cancelled
/// appointment is refused, and another shop's is not found.
pub fn check_in(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    appointment_id: &str,
) -> Result<QueuedPatient, CoreError> {
    let now = clock::now();
    let day = now.date();
    conn.transaction(|conn| {
        let booked = appointment_repo::get(conn, shop_id, appointment_id)?.appointment;
        if let Some(entry) = repo::for_appointment(conn, shop_id, &booked.id)? {
            return repo::get(conn, shop_id, &entry.id);
        }
        if !booked.is_live() {
            return Err(CoreError::conflict(
                "cancelled_at",
                "this appointment was cancelled",
            ));
        }
        let patient = queueable_patient(conn, shop_id, &booked.patient_id)?;
        let id = match repo::live_for(conn, shop_id, &patient.id, day)? {
            Some(walk_in) if walk_in.appointment_id.is_none() => {
                repo::link(conn, shop_id, &walk_in.id, &booked.id, now)?;
                let after = repo::get(conn, shop_id, &walk_in.id)?;
                audit::record(
                    conn,
                    shop_id,
                    user_id,
                    audit::Change {
                        action: ACTION_QUEUE_ARRIVE,
                        entity: "queue_entry",
                        entity_id: None,
                        before: Some(as_json(&walk_in)),
                        after: Some(as_json(&after.entry)),
                    },
                )?;
                walk_in.id
            }
            Some(_) => return Err(repo::already_waiting()),
            None => {
                let booking = Some(booked.id);
                arrive(
                    conn,
                    (shop_id, user_id),
                    patient,
                    booking,
                    ACTION_QUEUE_ARRIVE,
                    now,
                )?
                .id
            }
        };
        repo::get(conn, shop_id, &id)
    })
}

/// The patient when the file is this shop's and not archived.
fn queueable_patient(
    conn: &mut SqliteConnection,
    shop_id: i32,
    patient_id: &str,
) -> Result<Patient, CoreError> {
    let patient = patient_repo::get(conn, shop_id, patient_id)?;
    if patient.archived_at.is_some() {
        return Err(CoreError::conflict(
            "patient_id",
            "this patient's file is archived; open it again before queueing them",
        ));
    }
    Ok(patient)
}

/// A new entry on `now`'s day, arriving at `now`, at the end of the day's
/// order, with its audit row. `who` is the shop and the user writing it.
fn arrive(
    conn: &mut SqliteConnection,
    who: (i32, i32),
    patient: Patient,
    appointment_id: Option<String>,
    action: &'static str,
    now: chrono::NaiveDateTime,
) -> Result<QueueEntry, CoreError> {
    let (shop_id, user_id) = who;
    let row = QueueEntryInsert {
        id: uuid::Uuid::now_v7().to_string(),
        shop_id,
        patient_id: patient.id,
        day: now.date(),
        arrived_at: now,
        created_at: now,
        updated_at: now,
        appointment_id,
        position: repo::next_position(conn, shop_id, now.date())?,
    };
    let made = repo::insert(conn, &row)?;
    audit::record(
        conn,
        shop_id,
        user_id,
        audit::Change {
            action,
            entity: "queue_entry",
            entity_id: None,
            before: None,
            after: Some(as_json(&made)),
        },
    )?;
    Ok(made)
}

/// Puts today's queue in the order the desk dragged it into: the listed
/// entries take places 1, 2, 3 in the order given, and any entry of today
/// not listed (one added by another desk since this one last read the
/// list) follows them in the order it already had. Every entry of the day
/// may be listed, whatever its state. Refused as malformed (a 422 on
/// `ids`): an id listed twice, or one that is not an entry of today's queue
/// of this shop. Answers the day's list in its new order; an order the
/// same as before writes nothing.
pub fn reorder(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    ids: &[String],
) -> Result<Vec<QueuedPatient>, CoreError> {
    let now = clock::now();
    let today = now.date();
    conn.transaction(|conn| {
        let current = repo::day_list(conn, shop_id, today)?;
        let mut order: Vec<&QueueEntry> = Vec::with_capacity(current.len());
        for id in ids {
            let Some(found) = current.iter().find(|q| &q.entry.id == id) else {
                return Err(CoreError::validation(
                    "ids",
                    &format!("{id} is not an entry of today's queue"),
                ));
            };
            if order.iter().any(|e| &e.id == id) {
                return Err(CoreError::validation(
                    "ids",
                    &format!("{id} is listed twice"),
                ));
            }
            order.push(&found.entry);
        }
        for q in &current {
            if !order.iter().any(|e| e.id == q.entry.id) {
                order.push(&q.entry);
            }
        }
        let mut moved = false;
        for (place, entry) in (1_i32..).zip(order.iter()) {
            if entry.position != place {
                repo::set_position(conn, shop_id, &entry.id, place, now)?;
                moved = true;
            }
        }
        if moved {
            let ids_of = |list: Vec<&str>| {
                serde_json::json!({ "day": today.format("%Y-%m-%d").to_string(), "ids": list })
                    .to_string()
            };
            audit::record(
                conn,
                shop_id,
                user_id,
                audit::Change {
                    action: ACTION_QUEUE_REORDER,
                    entity: "queue",
                    entity_id: None,
                    before: Some(ids_of(
                        current.iter().map(|q| q.entry.id.as_str()).collect(),
                    )),
                    after: Some(ids_of(order.iter().map(|e| e.id.as_str()).collect())),
                },
            )?;
        }
        repo::day_list(conn, shop_id, today)
    })
}

/// Calls in the first of today's queue, in the desk's order, not yet called
/// and not gone. An
/// empty line is a conflict on `queue`: there is nobody to call, which the
/// screen says rather than a 404 about an entry nobody named.
pub fn call_next(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
) -> Result<QueuedPatient, CoreError> {
    let today = clock::now().date();
    conn.transaction(|conn| {
        let Some(next) = repo::next_waiting(conn, shop_id, today)? else {
            return Err(CoreError::conflict(
                "queue",
                "nobody in today's queue is waiting to be called",
            ));
        };
        call_in(conn, shop_id, user_id, &next.id, today)
    })
}

/// Calls in one entry the doctor picked, out of arrival order. Refused when
/// the patient was already called, seen or has gone, and when the entry is
/// not today's: a patient from another day is not in the waiting room.
pub fn call(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
) -> Result<QueuedPatient, CoreError> {
    let today = clock::now().date();
    conn.transaction(|conn| call_in(conn, shop_id, user_id, id, today))
}

/// Marks a called patient seen. Refused before the patient is called, and
/// once they are seen or gone. Not limited to today: a patient called in
/// just before midnight is seen after it.
pub fn mark_seen(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
) -> Result<QueuedPatient, CoreError> {
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        refuse_finished(&before.entry)?;
        if before.entry.called_at.is_none() {
            return Err(CoreError::conflict(
                "called_at",
                "a patient is called in before they are marked seen",
            ));
        }
        write(
            conn,
            shop_id,
            user_id,
            &before,
            Moment::Seen,
            ACTION_QUEUE_SEEN,
        )
    })
}

/// Marks a patient gone without being seen, at any point before they are
/// seen: while waiting, or after a call they did not answer.
pub fn mark_left(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
) -> Result<QueuedPatient, CoreError> {
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        refuse_finished(&before.entry)?;
        write(
            conn,
            shop_id,
            user_id,
            &before,
            Moment::Left,
            ACTION_QUEUE_LEFT,
        )
    })
}

/// Today's queue on the shop's clock, in the desk's order, every entry with its
/// patient's names: the waiting, the called, the seen and the gone alike, so
/// the screen shows the whole day.
pub fn today(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<QueuedPatient>, CoreError> {
    repo::day_list(conn, shop_id, clock::now().date())
}

/// One entry of this shop, whatever its day. Another shop's is not found.
pub fn get(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
) -> Result<QueuedPatient, CoreError> {
    repo::get(conn, shop_id, id)
}

fn call_in(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
    today: NaiveDate,
) -> Result<QueuedPatient, CoreError> {
    let before = repo::get(conn, shop_id, id)?;
    refuse_finished(&before.entry)?;
    if before.entry.called_at.is_some() {
        return Err(CoreError::conflict(
            "called_at",
            "this patient has already been called in",
        ));
    }
    if before.entry.day != today {
        return Err(CoreError::conflict(
            "day",
            "only a patient in today's queue can be called in",
        ));
    }
    write(
        conn,
        shop_id,
        user_id,
        &before,
        Moment::Called,
        ACTION_QUEUE_CALL,
    )
}

/// Seen and gone are both the end of an entry; nothing is written after
/// either.
fn refuse_finished(entry: &QueueEntry) -> Result<(), CoreError> {
    if entry.seen_at.is_some() {
        return Err(CoreError::conflict(
            "seen_at",
            "this patient has already been seen",
        ));
    }
    if entry.left_at.is_some() {
        return Err(CoreError::conflict(
            "left_at",
            "this patient left without being seen",
        ));
    }
    Ok(())
}

fn write(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    before: &QueuedPatient,
    moment: Moment,
    action: &'static str,
) -> Result<QueuedPatient, CoreError> {
    repo::stamp(conn, shop_id, &before.entry.id, moment, clock::now())?;
    let after = repo::get(conn, shop_id, &before.entry.id)?;
    audit::record(
        conn,
        shop_id,
        user_id,
        audit::Change {
            action,
            entity: "queue_entry",
            entity_id: None,
            before: Some(as_json(&before.entry)),
            after: Some(as_json(&after.entry)),
        },
    )?;
    Ok(after)
}

/// The entry as the audit row stores it. No name: the patient's id points at
/// the file, and the file's own rows carry the names.
fn as_json(e: &QueueEntry) -> String {
    let stamp =
        |t: Option<chrono::NaiveDateTime>| t.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string());
    serde_json::json!({
        "id": e.id,
        "patient_id": e.patient_id,
        "day": e.day.format("%Y-%m-%d").to_string(),
        "arrived_at": e.arrived_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        "called_at": stamp(e.called_at),
        "seen_at": stamp(e.seen_at),
        "left_at": stamp(e.left_at),
        "appointment_id": e.appointment_id,
        "position": e.position,
    })
    .to_string()
}
