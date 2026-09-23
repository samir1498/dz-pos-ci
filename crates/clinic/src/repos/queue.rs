//! The only place the waiting queue touches diesel. Every query is scoped by
//! `shop_id` (rule 3), and every read that joins the patient scopes the
//! patient by it too.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;

use crate::models::queue_entry::{QueueEntry, QueueEntryInsert, QueuedPatient};
use crate::schema::{appointments, patients, queue_entries};

/// Which of the three later moments a stamp writes.
#[derive(Debug, Clone, Copy)]
pub enum Moment {
    Called,
    Seen,
    Left,
}

/// The one table read everything below shares: an entry of this shop beside
/// the names of its patient, of the same shop, and the start of the
/// appointment it came in for, if it did.
type Row = (QueueEntry, String, String, Option<NaiveDateTime>);

fn joined(row: Row) -> QueuedPatient {
    let (entry, first_name, last_name, appointment_starts_at) = row;
    QueuedPatient {
        entry,
        first_name,
        last_name,
        appointment_starts_at,
    }
}

/// The entry, its patient and its appointment, in the shape `Row` reads.
/// The appointment is joined on its id and its shop both, so a link that
/// named another shop's booking (the service refuses one) reads as none.
macro_rules! joined_query {
    ($shop_id:expr) => {
        queue_entries::table
            .inner_join(patients::table)
            .left_join(
                appointments::table.on(queue_entries::appointment_id
                    .eq(appointments::id.nullable())
                    .and(appointments::shop_id.eq($shop_id))),
            )
            .filter(queue_entries::shop_id.eq($shop_id))
            .filter(patients::shop_id.eq($shop_id))
            .select((
                QueueEntry::as_select(),
                patients::first_name,
                patients::last_name,
                appointments::starts_at.nullable(),
            ))
    };
}

/// A new arrival. The two unique indexes on the table besides the fresh id
/// are the one-live-entry-per-day one and the one-entry-per-appointment
/// one, so a unique violation here is one of those: two desks that queued
/// the same patient, or marked the same booking arrived, in the same
/// instant, both past the service's own check.
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
    joined_query!(shop_id)
        .filter(queue_entries::id.eq(id))
        .first::<Row>(conn)
        .optional()?
        .map(joined)
        .ok_or_else(|| not_found(id))
}

/// One day's entries in the desk's order. A tie in place, which the service
/// never writes, falls back to arrival order, and two arrivals inside the
/// same stamp are told apart by the id, a UUID v7 that sorts in the order
/// it was made.
pub fn day_list(
    conn: &mut SqliteConnection,
    shop_id: i32,
    day: NaiveDate,
) -> Result<Vec<QueuedPatient>, CoreError> {
    Ok(joined_query!(shop_id)
        .filter(queue_entries::day.eq(day))
        .order((
            queue_entries::position.asc(),
            queue_entries::arrived_at.asc(),
            queue_entries::id.asc(),
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

/// The entry made for this appointment, whatever its day or state: there
/// is at most one (the migration's index).
pub fn for_appointment(
    conn: &mut SqliteConnection,
    shop_id: i32,
    appointment_id: &str,
) -> Result<Option<QueueEntry>, CoreError> {
    Ok(queue_entries::table
        .filter(queue_entries::shop_id.eq(shop_id))
        .filter(queue_entries::appointment_id.eq(appointment_id))
        .select(QueueEntry::as_select())
        .first(conn)
        .optional()?)
}

/// Names the appointment on a walk-in's entry that has none yet. The
/// `WHERE` keeps a second writer from relinking an entry the service read
/// as a walk-in; that case, and the index refusing a second entry for the
/// appointment, read as the patient already waiting.
pub fn link(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
    appointment_id: &str,
    at: NaiveDateTime,
) -> Result<(), CoreError> {
    let changed = diesel::update(
        queue_entries::table
            .filter(queue_entries::shop_id.eq(shop_id))
            .filter(queue_entries::id.eq(id))
            .filter(queue_entries::appointment_id.is_null()),
    )
    .set((
        queue_entries::appointment_id.eq(Some(appointment_id)),
        queue_entries::updated_at.eq(at),
    ))
    .execute(conn)
    .map_err(|e| match e {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => already_waiting(),
        other => CoreError::from(other),
    })?;
    if changed == 1 {
        Ok(())
    } else {
        Err(already_waiting())
    }
}

/// The first of the day in the desk's order still waiting to be called:
/// not called, not gone. (Not seen follows: the table refuses seen without
/// called.)
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
        .order((
            queue_entries::position.asc(),
            queue_entries::arrived_at.asc(),
            queue_entries::id.asc(),
        ))
        .select(QueueEntry::as_select())
        .first(conn)
        .optional()?)
}

/// The place after the day's last, 1 on an empty day: where a new arrival
/// goes.
pub fn next_position(
    conn: &mut SqliteConnection,
    shop_id: i32,
    day: NaiveDate,
) -> Result<i32, CoreError> {
    let last: Option<i32> = queue_entries::table
        .filter(queue_entries::shop_id.eq(shop_id))
        .filter(queue_entries::day.eq(day))
        .select(diesel::dsl::max(queue_entries::position))
        .first(conn)?;
    last.unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| CoreError::validation("position", "the day's queue is full"))
}

/// Puts one entry at `position` and moves `updated_at` with it.
pub fn set_position(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
    position: i32,
    at: NaiveDateTime,
) -> Result<(), CoreError> {
    let changed = diesel::update(
        queue_entries::table
            .filter(queue_entries::shop_id.eq(shop_id))
            .filter(queue_entries::id.eq(id)),
    )
    .set((
        queue_entries::position.eq(position),
        queue_entries::updated_at.eq(at),
    ))
    .execute(conn)?;
    if changed == 1 {
        Ok(())
    } else {
        Err(not_found(id))
    }
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
