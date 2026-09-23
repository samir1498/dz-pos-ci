//! Visit types (item 3 of the book tools, C5b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! a name and a length, kept on the settings grid beside the slot length and
//! written with the same `EditSettings`. A booking that names one takes its
//! length; one that names none takes the slot length, as in C5. The grid
//! every start sits on stays the slot-length setting either way.
//!
//! The length is 5 to 240 minutes and a multiple of the grid in force when
//! the type is written. A later change of the grid does not reopen the
//! types already written: a visit of any length still starts on the grid,
//! and the overlap check reads each appointment's own length.
//!
//! An appointment copies the length and keeps no pointer to the type, so
//! editing or removing a type changes no booking already made.

use chrono::NaiveDateTime;
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::{audit, clock};

use crate::audit_actions::{
    ACTION_VISIT_TYPE_CREATE, ACTION_VISIT_TYPE_REMOVE, ACTION_VISIT_TYPE_UPDATE,
};
use crate::repos::visit_types as repo;
use crate::services::slot_length;

pub use crate::models::visit_type::VisitType;

/// The shortest and longest a type may run: 240 is the book's own ceiling.
pub const MIN_VISIT_MINUTES: i32 = 5;
pub const MAX_VISIT_MINUTES: i32 = 240;
/// The longest name the table holds.
pub const MAX_NAME_CHARS: usize = 60;

/// A type as the settings screen writes it.
#[derive(Debug, Clone, Default)]
pub struct NewVisitType {
    pub name: String,
    pub minutes: i32,
}

/// Every type of this shop, by name.
pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<VisitType>, CoreError> {
    repo::all(conn, shop_id)
}

/// One type of this shop. Another shop's is not found.
pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: &str) -> Result<VisitType, CoreError> {
    repo::get(conn, shop_id, id)
}

pub fn create(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    new: NewVisitType,
) -> Result<VisitType, CoreError> {
    let name = name(&new.name)?;
    let now = clock::now();
    conn.transaction(|conn| {
        minutes(conn, shop_id, new.minutes)?;
        let made = repo::insert(
            conn,
            &VisitType {
                id: uuid::Uuid::now_v7().to_string(),
                shop_id,
                name,
                minutes: new.minutes,
                created_at: now,
                updated_at: now,
            },
        )?;
        record(
            conn,
            shop_id,
            user_id,
            ACTION_VISIT_TYPE_CREATE,
            None,
            Some(&made),
        )?;
        Ok(made)
    })
}

/// Renames a type or changes its length, on the rules a new one meets.
/// Bookings already made keep the length they copied.
pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
    new: NewVisitType,
) -> Result<VisitType, CoreError> {
    let name = name(&new.name)?;
    let now: NaiveDateTime = clock::now();
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        minutes(conn, shop_id, new.minutes)?;
        repo::update(conn, shop_id, id, &name, new.minutes, now)?;
        let after = repo::get(conn, shop_id, id)?;
        record(
            conn,
            shop_id,
            user_id,
            ACTION_VISIT_TYPE_UPDATE,
            Some(&before),
            Some(&after),
        )?;
        Ok(after)
    })
}

/// Removes a type and answers with it as it was. Bookings made with it
/// keep their length.
pub fn remove(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
) -> Result<VisitType, CoreError> {
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        repo::delete(conn, shop_id, id)?;
        record(
            conn,
            shop_id,
            user_id,
            ACTION_VISIT_TYPE_REMOVE,
            Some(&before),
            None,
        )?;
        Ok(before)
    })
}

fn name(text: &str) -> Result<String, CoreError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(CoreError::validation("name", "a visit type has a name"));
    }
    if text.chars().count() > MAX_NAME_CHARS {
        return Err(CoreError::validation(
            "name",
            "a visit type's name is 60 characters at most",
        ));
    }
    Ok(text.to_string())
}

/// 5 to 240 and a multiple of the slot length in force.
fn minutes(conn: &mut SqliteConnection, shop_id: i32, minutes: i32) -> Result<(), CoreError> {
    let grid = slot_length::slot_minutes(conn, shop_id)?;
    let fits = (MIN_VISIT_MINUTES..=MAX_VISIT_MINUTES).contains(&minutes)
        && u32::try_from(minutes).is_ok_and(|m| m.is_multiple_of(grid));
    if fits {
        Ok(())
    } else {
        Err(CoreError::validation(
            "minutes",
            &format!("a visit runs 5 to 240 minutes, a multiple of the {grid}-minute slot"),
        ))
    }
}

fn record(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    action: &'static str,
    before: Option<&VisitType>,
    after: Option<&VisitType>,
) -> Result<(), CoreError> {
    audit::record(
        conn,
        shop_id,
        user_id,
        audit::Change {
            action,
            entity: "visit_type",
            entity_id: None,
            before: before.map(as_json),
            after: after.map(as_json),
        },
    )?;
    Ok(())
}

fn as_json(t: &VisitType) -> String {
    serde_json::json!({ "id": t.id, "name": t.name, "minutes": t.minutes }).to_string()
}
