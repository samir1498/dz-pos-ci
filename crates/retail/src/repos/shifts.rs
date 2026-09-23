//! The only place `shifts` touches diesel. Every query is scoped by
//! `shop_id` (rule 3), and the ones a single person's drawer is read through
//! are scoped by `opened_by` as well: two cashiers hold overlapping shifts on
//! purpose, so a query that forgot the user would hand each of them the
//! other one's evening.
//!
//! `services::shifts` is what calls all of it, and the only thing that does.
//! `list_between` is the exception and still waits for the shift list screen;
//! the round trip below is what says it works until then.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::shift::{Shift, ShiftCloseWrite, ShiftRow, ShiftRowWrite};
use crate::schema::shifts;

/// Opens a drawer. The unique index refuses a second open shift for the same
/// person, and that refusal comes back as a `Conflict` rather than a bare
/// query error so the caller has a field and a sentence to show.
pub fn insert(conn: &mut SqliteConnection, write: &ShiftRowWrite) -> Result<Shift, CoreError> {
    let row: ShiftRow = match diesel::insert_into(shifts::table)
        .values(write)
        .returning(ShiftRow::as_returning())
        .get_result(conn)
    {
        Ok(row) => row,
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        )) => {
            return Err(CoreError::Conflict {
                field: "opened_by".to_string(),
                message: "that person already has a till open".to_string(),
            })
        }
        Err(other) => return Err(CoreError::Query(other)),
    };
    Ok(Shift::from(row))
}

/// One shift of this shop.
pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Shift, CoreError> {
    let row: ShiftRow = shifts::table
        .filter(shifts::shop_id.eq(shop_id))
        .filter(shifts::id.eq(id))
        .select(ShiftRow::as_select())
        .first(conn)
        .optional()?
        .ok_or(CoreError::NotFound {
            entity: "shift",
            id,
        })?;
    Ok(Shift::from(row))
}

/// The one open shift this person holds, or None. The index makes "the one"
/// true, so this reads a single row rather than the newest of several.
pub fn open_for(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
) -> Result<Option<Shift>, CoreError> {
    let row: Option<ShiftRow> = shifts::table
        .filter(shifts::shop_id.eq(shop_id))
        .filter(shifts::opened_by.eq(user_id))
        .filter(shifts::closed_at.is_null())
        .select(ShiftRow::as_select())
        .first(conn)
        .optional()?;
    Ok(row.map(Shift::from))
}

/// Writes the four close columns and the note in one UPDATE, and only onto a
/// shift of this shop that is still open. A second close finds no row and is
/// answered `NotFound` rather than overwriting a count somebody signed.
pub fn close(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    write: &ShiftCloseWrite,
) -> Result<Shift, CoreError> {
    let row: ShiftRow = diesel::update(
        shifts::table
            .filter(shifts::shop_id.eq(shop_id))
            .filter(shifts::id.eq(id))
            .filter(shifts::closed_at.is_null()),
    )
    .set(write)
    .returning(ShiftRow::as_returning())
    .get_result(conn)
    .optional()?
    .ok_or(CoreError::NotFound {
        entity: "shift",
        id,
    })?;
    Ok(Shift::from(row))
}

/// The shop's shifts opened inside a half open window, newest first, and
/// optionally one person's alone. Half open on `opened_at` for the reason
/// `clock::Period::moments` gives: every moment of the last day counts and
/// the next day's first does not.
///
/// `user_id` is the manager's list narrowing to one person, not the per-user
/// scoping every other query in this file carries: a manager runs the whole
/// floor off the plain window, and the filter is added only when a screen
/// asks for it.
pub fn list_between(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
    user_id: Option<i32>,
) -> Result<Vec<Shift>, CoreError> {
    let mut query = shifts::table
        .filter(shifts::shop_id.eq(shop_id))
        .filter(shifts::opened_at.ge(from))
        .filter(shifts::opened_at.lt(until))
        .into_boxed();
    if let Some(user_id) = user_id {
        query = query.filter(shifts::opened_by.eq(user_id));
    }
    let rows: Vec<ShiftRow> = query
        .order((shifts::opened_at.desc(), shifts::id.desc()))
        .select(ShiftRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(Shift::from).collect())
}

/// The latest moment this person's drawer was counted, or None when they have
/// never closed one.
///
/// The partial unique index is `WHERE closed_at IS NULL`, so it refuses two
/// drawers open at once and says nothing at all about two closed windows. Two
/// closed shifts of one person that overlap would each sum the sale in the
/// overlap into their own stored expected figure, and the cashier held that
/// money once. This is what `services::shifts::open` holds a new `opened_at`
/// against.
///
/// `MAX` and not "the newest row's close": the list is ordered by `opened_at`,
/// and a shift opened earlier can be counted later.
pub fn last_close_for(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
) -> Result<Option<NaiveDateTime>, CoreError> {
    let latest: Option<Option<NaiveDateTime>> = shifts::table
        .filter(shifts::shop_id.eq(shop_id))
        .filter(shifts::opened_by.eq(user_id))
        .select(diesel::dsl::max(shifts::closed_at))
        .first(conn)
        .optional()?;
    Ok(latest.flatten())
}

/// Every shift one person has held, newest first. This is the list a sale is
/// held against to see whether it fell inside any of that person's own
/// windows; a shop-wide list would say a sale belonged to a shift somebody
/// else was holding.
pub fn list_for_user(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
) -> Result<Vec<Shift>, CoreError> {
    let rows: Vec<ShiftRow> = shifts::table
        .filter(shifts::shop_id.eq(shop_id))
        .filter(shifts::opened_by.eq(user_id))
        .order((shifts::opened_at.desc(), shifts::id.desc()))
        .select(ShiftRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(Shift::from).collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "../../tests/unit/repos_shifts.rs"]
mod tests;
