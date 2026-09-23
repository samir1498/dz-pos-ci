//! The waiting queue (C4 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! a patient is added on arrival, called in (the next in line, or one the
//! doctor picks out of order), then either seen or gone unseen. One day at a
//! time, on the shop's clock: today's list is today's arrivals, and an entry
//! left live last night does not stand in this morning's line.
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
    ACTION_QUEUE_ADD, ACTION_QUEUE_CALL, ACTION_QUEUE_LEFT, ACTION_QUEUE_SEEN,
};
use crate::models::queue_entry::QueueEntryInsert;
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
        let patient = patient_repo::get(conn, shop_id, patient_id)?;
        if patient.archived_at.is_some() {
            return Err(CoreError::conflict(
                "patient_id",
                "this patient's file is archived; open it again before queueing them",
            ));
        }
        if repo::live_for(conn, shop_id, &patient.id, day)?.is_some() {
            return Err(repo::already_waiting());
        }
        let row = QueueEntryInsert {
            id: uuid::Uuid::now_v7().to_string(),
            shop_id,
            patient_id: patient.id,
            day,
            arrived_at: now,
            created_at: now,
            updated_at: now,
        };
        let made = repo::insert(conn, &row)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: ACTION_QUEUE_ADD,
                entity: "queue_entry",
                entity_id: None,
                before: None,
                after: Some(as_json(&made)),
            },
        )?;
        repo::get(conn, shop_id, &made.id)
    })
}

/// Calls in the earliest arrival of today not yet called and not gone. An
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

/// Today's queue on the shop's clock, in arrival order, every entry with its
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
    })
    .to_string()
}
