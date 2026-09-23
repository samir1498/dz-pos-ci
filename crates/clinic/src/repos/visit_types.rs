//! The only place the visit types touch diesel. Every query is scoped by
//! `shop_id` (rule 3).

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;

use crate::models::visit_type::VisitType;
use crate::schema::visit_types;

/// The name index is the table's only unique one besides the fresh id, so
/// a unique violation is always a second type of the same name.
fn name_taken(e: DieselError) -> CoreError {
    match e {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
            CoreError::conflict("name", "a visit type of this name already exists")
        }
        other => CoreError::from(other),
    }
}

pub fn insert(conn: &mut SqliteConnection, row: &VisitType) -> Result<VisitType, CoreError> {
    diesel::insert_into(visit_types::table)
        .values(row)
        .returning(VisitType::as_returning())
        .get_result(conn)
        .map_err(name_taken)
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: &str) -> Result<VisitType, CoreError> {
    visit_types::table
        .filter(visit_types::shop_id.eq(shop_id))
        .filter(visit_types::id.eq(id))
        .select(VisitType::as_select())
        .first(conn)
        .optional()?
        .ok_or_else(|| CoreError::NotFoundText {
            entity: "visit_type",
            id: id.to_string(),
        })
}

/// Every type of this shop, by name.
pub fn all(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<VisitType>, CoreError> {
    Ok(visit_types::table
        .filter(visit_types::shop_id.eq(shop_id))
        .order((visit_types::name.asc(), visit_types::id.asc()))
        .select(VisitType::as_select())
        .load(conn)?)
}

pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: &str,
    name: &str,
    minutes: i32,
    at: NaiveDateTime,
) -> Result<(), CoreError> {
    diesel::update(
        visit_types::table
            .filter(visit_types::shop_id.eq(shop_id))
            .filter(visit_types::id.eq(id)),
    )
    .set((
        visit_types::name.eq(name),
        visit_types::minutes.eq(minutes),
        visit_types::updated_at.eq(at),
    ))
    .execute(conn)
    .map_err(name_taken)?;
    Ok(())
}

pub fn delete(conn: &mut SqliteConnection, shop_id: i32, id: &str) -> Result<(), CoreError> {
    diesel::delete(
        visit_types::table
            .filter(visit_types::shop_id.eq(shop_id))
            .filter(visit_types::id.eq(id)),
    )
    .execute(conn)?;
    Ok(())
}
