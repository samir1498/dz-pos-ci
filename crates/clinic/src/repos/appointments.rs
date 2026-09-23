//! The only place the appointment book touches diesel. Every query is scoped
//! by `shop_id` (rule 3), and every read that joins the patient scopes the
//! patient by it too.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;

use crate::models::appointment::{Appointment, AppointmentInsert, BookedPatient, CallOutcome};
use crate::schema::{appointments, patients};

/// The one table read the joined answers share: an appointment of this shop
/// beside the names and the phone of its patient, of the same shop.
type Row = (Appointment, String, String, Option<String>);

fn joined(row: Row) -> BookedPatient {
    let (appointment, first_name, last_name, phone) = row;
    BookedPatient {
        appointment,
        first_name,
        last_name,
        phone,
    }
}

/// The only unique index on the table besides the fresh id is the
/// one-live-start-per-shop one, so a unique violation on a write here is
/// always that one: two desks that took the same start in the same instant,
/// both past the service's own check.
fn taken_by_the_index(e: DieselError) -> CoreError {
    match e {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => CoreError::conflict(
            "starts_at",
            "another desk booked this slot at the same moment",
        ),
        other => CoreError::from(other),
    }
}

/// A new appointment.
pub fn insert(
    conn: &mut SqliteConnection,
    row: &AppointmentInsert,
) -> Result<Appointment, CoreError> {
    diesel::insert_into(appointments::table)
        .values(row)
        .returning(Appointment::as_returning())
        .get_result(conn)
        .map_err(taken_by_the_index)
}

/// One appointment of this shop with its patient's names, live or not.
pub fn get(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
) -> Result<BookedPatient, CoreError> {
    appointments::table
        .inner_join(patients::table)
        .filter(appointments::shop_id.eq(shop_id))
        .filter(patients::shop_id.eq(shop_id))
        .filter(appointments::id.eq(id))
        .select((
            Appointment::as_select(),
            patients::first_name,
            patients::last_name,
            patients::phone,
        ))
        .first::<Row>(conn)
        .optional()?
        .map(joined)
        .ok_or_else(|| not_found(id))
}

/// The live appointments starting at or after `from` and before `until`, in
/// time order. Two starts are never equal among live rows (the index), so
/// the id after the start only makes the order total on paper.
pub fn live_between(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Vec<BookedPatient>, CoreError> {
    Ok(appointments::table
        .inner_join(patients::table)
        .filter(appointments::shop_id.eq(shop_id))
        .filter(patients::shop_id.eq(shop_id))
        .filter(appointments::cancelled_at.is_null())
        .filter(appointments::starts_at.ge(from))
        .filter(appointments::starts_at.lt(until))
        .order((appointments::starts_at.asc(), appointments::id.asc()))
        .select((
            Appointment::as_select(),
            patients::first_name,
            patients::last_name,
            patients::phone,
        ))
        .load::<Row>(conn)?
        .into_iter()
        .map(joined)
        .collect())
}

/// The live appointments of this shop starting at or after `from` and
/// before `until`, other than `except`: the ones a new slot could run into.
/// No join, since the overlap check needs no name.
pub fn live_starting_between(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
    except: Option<&str>,
) -> Result<Vec<Appointment>, CoreError> {
    let mut query = appointments::table
        .filter(appointments::shop_id.eq(shop_id))
        .filter(appointments::cancelled_at.is_null())
        .filter(appointments::starts_at.ge(from))
        .filter(appointments::starts_at.lt(until))
        .select(Appointment::as_select())
        .into_boxed();
    if let Some(id) = except {
        query = query.filter(appointments::id.ne(id));
    }
    Ok(query.load(conn)?)
}

/// Gives a live slot back and clears a no-show mark with it, the row being
/// one or the other (the table's CHECK). The service has read the row and
/// refused a cancelled one; the `WHERE` here only keeps a second writer that
/// got in between from stamping it twice, and that case reads as not found.
pub fn cancel(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
    at: NaiveDateTime,
) -> Result<(), CoreError> {
    let changed = diesel::update(
        appointments::table
            .filter(appointments::shop_id.eq(shop_id))
            .filter(appointments::id.eq(id))
            .filter(appointments::cancelled_at.is_null()),
    )
    .set((
        appointments::cancelled_at.eq(Some(at)),
        appointments::no_show_at.eq(None::<NaiveDateTime>),
        appointments::updated_at.eq(at),
    ))
    .execute(conn)?;
    one_row(changed, id)
}

/// Moves a live appointment to a new start, with the slot length it now
/// takes. The index refuses a start another live row holds, the same way it
/// does on insert.
pub fn reschedule(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
    starts_at: NaiveDateTime,
    slot_minutes: i32,
    at: NaiveDateTime,
) -> Result<(), CoreError> {
    let changed = diesel::update(
        appointments::table
            .filter(appointments::shop_id.eq(shop_id))
            .filter(appointments::id.eq(id))
            .filter(appointments::cancelled_at.is_null()),
    )
    .set((
        appointments::starts_at.eq(starts_at),
        appointments::slot_minutes.eq(slot_minutes),
        appointments::updated_at.eq(at),
    ))
    .execute(conn)
    .map_err(taken_by_the_index)?;
    one_row(changed, id)
}

/// Sets or clears the no-show mark of an appointment. Setting it takes a
/// cancellation back, the row being one or the other (the table's CHECK);
/// that puts the start in the live index again, which refuses it when
/// another appointment took the slot in between the service's own check
/// and this write.
pub fn set_no_show(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
    mark: Option<NaiveDateTime>,
    at: NaiveDateTime,
) -> Result<(), CoreError> {
    let target = appointments::table
        .filter(appointments::shop_id.eq(shop_id))
        .filter(appointments::id.eq(id));
    let changed = match mark {
        Some(_) => diesel::update(target)
            .set((
                appointments::no_show_at.eq(mark),
                appointments::cancelled_at.eq(None::<NaiveDateTime>),
                appointments::updated_at.eq(at),
            ))
            .execute(conn)
            .map_err(taken_by_the_index)?,
        None => diesel::update(target)
            .set((
                appointments::no_show_at.eq(mark),
                appointments::updated_at.eq(at),
            ))
            .execute(conn)?,
    };
    one_row(changed, id)
}

/// Records what came of the confirmation call, with its moment, or clears
/// both with `None`. Any row of this shop, live or not: a call is a note of
/// the desk's and changes nothing else.
pub fn set_call(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
    call: Option<(CallOutcome, NaiveDateTime)>,
    at: NaiveDateTime,
) -> Result<(), CoreError> {
    let changed = diesel::update(
        appointments::table
            .filter(appointments::shop_id.eq(shop_id))
            .filter(appointments::id.eq(id)),
    )
    .set((
        appointments::call_outcome.eq(call.map(|(outcome, _)| outcome)),
        appointments::call_at.eq(call.map(|(_, moment)| moment)),
        appointments::updated_at.eq(at),
    ))
    .execute(conn)?;
    one_row(changed, id)
}

fn one_row(changed: usize, id: &str) -> Result<(), CoreError> {
    if changed == 1 {
        Ok(())
    } else {
        Err(not_found(id))
    }
}

fn not_found(id: &str) -> CoreError {
    CoreError::NotFoundText {
        entity: "appointment",
        id: id.to_string(),
    }
}
