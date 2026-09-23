//! The only place patients touch diesel. Every query is scoped by `shop_id`
//! (rule 3); a function that forgets it is the bug this layer exists to make
//! visible.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;

use crate::models::patient::{Patient, PatientChanges, PatientInsert};
use crate::repos::contains_pattern;
use crate::schema::patients;

/// What the search box asked for, already split by the service into the
/// text a name is matched against and the digits a phone number is.
pub struct Search<'a> {
    /// Trimmed, non-blank. Matched anywhere in the first name, the last
    /// name, or either order of the two joined by a space, so "amina ben"
    /// finds Amina Benali.
    pub text: &'a str,
    /// The same text with the spaces, dots and dashes a person types
    /// between digits taken out, the way the service stores a number.
    /// `None` when the text holds no digit at all, so a name never matches
    /// a phone by accident.
    pub phone: Option<&'a str>,
}

/// One shop's files by surname, then first name, then id: two patients may
/// share both names, and the id keeps the order stable between two calls.
///
/// Case-insensitive through SQLite's own LIKE, which folds ASCII letters
/// only: an accented capital (`É`) does not match its small letter, and
/// Arabic has no case to fold. Archived files are left out unless the
/// caller asks for them.
pub fn list(
    conn: &mut SqliteConnection,
    shop_id: i32,
    search: Option<Search<'_>>,
    include_archived: bool,
) -> Result<Vec<Patient>, CoreError> {
    let mut query = patients::table
        .filter(patients::shop_id.eq(shop_id))
        .into_boxed();
    if !include_archived {
        query = query.filter(patients::archived_at.is_null());
    }
    if let Some(search) = search {
        let text = contains_pattern(search.text);
        let by_name = patients::first_name
            .like(text.clone())
            .escape('\\')
            .or(patients::last_name.like(text.clone()).escape('\\'))
            .or(patients::first_name
                .concat(" ")
                .concat(patients::last_name)
                .like(text.clone())
                .escape('\\'))
            .or(patients::last_name
                .concat(" ")
                .concat(patients::first_name)
                .like(text)
                .escape('\\'));
        query = match search.phone {
            // A file with no phone is not a match, and `NULL LIKE …` is
            // NULL, which an OR treats as no match. `assume_not_null` only
            // says so to the type system.
            Some(digits) => query.filter(
                by_name.or(patients::phone
                    .like(contains_pattern(digits))
                    .escape('\\')
                    .assume_not_null()),
            ),
            None => query.filter(by_name),
        };
    }
    Ok(query
        .order((
            patients::last_name.asc(),
            patients::first_name.asc(),
            patients::id.asc(),
        ))
        .select(Patient::as_select())
        .load(conn)?)
}

/// One file, archived or not: a queue entry or an appointment that names an
/// archived patient still has to read the name back.
pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: &str) -> Result<Patient, CoreError> {
    patients::table
        .filter(patients::shop_id.eq(shop_id))
        .filter(patients::id.eq(id))
        .select(Patient::as_select())
        .first(conn)
        .optional()?
        .ok_or_else(|| not_found(id))
}

pub fn insert(conn: &mut SqliteConnection, row: &PatientInsert) -> Result<Patient, CoreError> {
    Ok(diesel::insert_into(patients::table)
        .values(row)
        .returning(Patient::as_returning())
        .get_result(conn)?)
}

pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
    changes: &PatientChanges,
) -> Result<Patient, CoreError> {
    diesel::update(
        patients::table
            .filter(patients::shop_id.eq(shop_id))
            .filter(patients::id.eq(id)),
    )
    .set(changes)
    .returning(Patient::as_returning())
    .get_result(conn)
    .optional()?
    .ok_or_else(|| not_found(id))
}

/// Stamps `archived_at` on a live file. A file already archived is left as
/// it is and read as not found here; the service reads it first and says
/// which of the two it was.
pub fn archive(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
    at: NaiveDateTime,
) -> Result<Patient, CoreError> {
    diesel::update(
        patients::table
            .filter(patients::shop_id.eq(shop_id))
            .filter(patients::id.eq(id))
            .filter(patients::archived_at.is_null()),
    )
    .set((
        patients::archived_at.eq(Some(at)),
        patients::updated_at.eq(at),
    ))
    .returning(Patient::as_returning())
    .get_result(conn)
    .optional()?
    .ok_or_else(|| not_found(id))
}

fn not_found(id: &str) -> CoreError {
    CoreError::NotFoundText {
        entity: "patient",
        id: id.to_string(),
    }
}
