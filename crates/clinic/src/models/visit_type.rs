//! A kind of visit the cabinet books, with its own length.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::schema::visit_types;

/// A visit type as the table holds it.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Selectable, Insertable)]
#[diesel(table_name = visit_types)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct VisitType {
    /// A UUID v7, as its 36-character hyphenated text.
    pub id: String,
    pub shop_id: i32,
    pub name: String,
    /// How long a visit of this type runs: 5 to 240, a multiple of the grid
    /// when it was written.
    pub minutes: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
