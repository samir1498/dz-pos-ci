//! The only place the waiting queue touches diesel. Every query is scoped by
//! `shop_id` (rule 3), and every read that joins the patient scopes the
//! patient by it too.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;

use crate::models::queue_entry::{QueueEntry, QueueEntryInsert, QueuedPatient};
use crate::schema::{patients, queue_entries};

/// Which of the three later moments a stamp writes.
#[derive(Debug, Clone, Copy)]
pub enum Moment {
    Called,
    Seen,
    Left,
}

/// The one table read everything below shares: an entry of this shop beside
/// the names of its patient, of the same shop.
type Row = (QueueEntry, String, String);

fn joined(row: Row) -> QueuedPatient {
    let (entry, first_name, last_name) = row;
    QueuedPatient {
        entry,
        first_name,
        last_name,
    }
}

/// A new arrival. The only unique index on the table besides the fresh id
/// is the one-live-entry-per-day one, so a unique violation here is always
/// that one: two desks that queued the same patient in the same instant,
/// both past the service's own check.
pub fn insert(
    conn: &mut SqliteConnection,
    row: &QueueEntryInsert,
) -> Result<QueueEntry, CoreError> {
    diesel::insert_into(queue_entries::table)
        .values(row)
        .returning(QueueEntry::as_returning())
        .get_result(conn)
        .map_err(|e| match e {
            DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => already_waiting(),
            other => CoreError::from(other),
        })
}

/// The refusal a second live entry for one patient on one day gets, from the
/// service's check and from the index alike.
pub fn already_waiting() -> CoreError {
    CoreError::conflict(
        "patient_id",
        "this patient is already in today's queue and has not been seen",
    )
}

/// One entry of this shop with its patient's names, whatever its day.
pub fn get(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
) -> Result<QueuedPatient, CoreError> {
    queue_entries::table
        .inner_join(patients::table)
        .filter(queue_entries::shop_id.eq(shop_id))
        .filter(patients::shop_id.eq(shop_id))
        .filter(queue_entries::id.eq(id))
        .select((
            QueueEntry::as_select(),
            patients::first_name,
            patients::last_name,
        ))
        .first::<Row>(conn)
        .optional()?
        .map(joined)
        .ok_or_else(|| not_found(id))
}

/// One day's entries in arrival order. Two arrivals inside the same stamp
/// are told apart by the id, a UUID v7 that sorts in the order it was made.
pub fn day_list(
    conn: &mut SqliteConnection,
    shop_id: i32,
    day: NaiveDate,
) -> Result<Vec<QueuedPatient>, CoreError> {
    Ok(queue_entries::table
        .inner_join(patients::table)
        .filter(queue_entries::shop_id.eq(shop_id))
        .filter(patients::shop_id.eq(shop_id))
        .filter(queue_entries::day.eq(day))
        .order((queue_entries::arrived_at.asc(), queue_entries::id.asc()))
        .select((
            QueueEntry::as_select(),
            patients::first_name,
            patients::last_name,
        ))
        .load::<Row>(conn)?
        .into_iter()
        .map(joined)
        .collect())
}

/// The patient's live entry on that day, if there is one.
pub fn live_for(
    conn: &mut SqliteConnection,
    shop_id: i32,
    patient_id: &str,
    day: NaiveDate,
) -> Result<Option<QueueEntry>, CoreError> {
    Ok(queue_entries::table
        .filter(queue_entries::shop_id.eq(shop_id))
        .filter(queue_entries::patient_id.eq(patient_id))
        .filter(queue_entries::day.eq(day))
        .filter(queue_entries::seen_at.is_null())
        .filter(queue_entries::left_at.is_null())
        .select(QueueEntry::as_select())
        .first(conn)
        .optional()?)
}

/// The earliest arrival of the day still waiting to be called: not called,
/// not gone. (Not seen follows: the table refuses seen without called.)
pub fn next_waiting(
    conn: &mut SqliteConnection,
    shop_id: i32,
    day: NaiveDate,
) -> Result<Option<QueueEntry>, CoreError> {
    Ok(queue_entries::table
        .filter(queue_entries::shop_id.eq(shop_id))
        .filter(queue_entries::day.eq(day))
        .filter(queue_entries::called_at.is_null())
        .filter(queue_entries::left_at.is_null())
        .order((queue_entries::arrived_at.asc(), queue_entries::id.asc()))
        .select(QueueEntry::as_select())
        .first(conn)
        .optional()?)
}

/// Writes one moment on a live entry and moves `updated_at` with it. The
/// service has read the entry and refused whatever the moment may not
/// follow; the `WHERE` here only keeps a second writer that got in between
/// from stamping a finished entry, and that case reads as not found.
pub fn stamp(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
    moment: Moment,
    at: NaiveDateTime,
) -> Result<(), CoreError> {
    let target = queue_entries::table
        .filter(queue_entries::shop_id.eq(shop_id))
        .filter(queue_entries::id.eq(id))
        .filter(queue_entries::seen_at.is_null())
        .filter(queue_entries::left_at.is_null());
    let changed = match moment {
        Moment::Called => diesel::update(target.filter(queue_entries::called_at.is_null()))
            .set((
                queue_entries::called_at.eq(Some(at)),
                queue_entries::updated_at.eq(at),
            ))
            .execute(conn)?,
        Moment::Seen => diesel::update(target)
            .set((
                queue_entries::seen_at.eq(Some(at)),
                queue_entries::updated_at.eq(at),
            ))
            .execute(conn)?,
        Moment::Left => diesel::update(target)
            .set((
                queue_entries::left_at.eq(Some(at)),
                queue_entries::updated_at.eq(at),
            ))
            .execute(conn)?,
    };
    if changed == 1 {
        Ok(())
    } else {
        Err(not_found(id))
    }
}

fn not_found(id: &str) -> CoreError {
    CoreError::NotFoundText {
        entity: "queue entry",
        id: id.to_string(),
    }
}
