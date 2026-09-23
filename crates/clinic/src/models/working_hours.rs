//! The cabinet's working week: for each weekday, Sunday first, the ranges
//! the book is open in. A lunch break is two ranges; a day with none is
//! closed.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::schema::working_hours;

/// Days in the week, and so in `WorkingWeek::days`.
pub const DAYS_IN_WEEK: usize = 7;

/// One open range of a day, in minutes from midnight, half-open: a visit
/// may end on `closes_minute` and may not start on it. 1440 closes at
/// midnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenRange {
    pub opens_minute: i32,
    pub closes_minute: i32,
}

/// The week as the service answers it: `days[0]` is Sunday and `days[6]`
/// Saturday, each day's ranges in time order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkingWeek {
    pub days: [Vec<OpenRange>; DAYS_IN_WEEK],
}

/// A row as the table holds it.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Selectable)]
#[diesel(table_name = working_hours)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct WorkingHoursRow {
    pub weekday: i32,
    pub opens_minute: i32,
    pub closes_minute: i32,
}

/// A row as the service writes it.
#[derive(Debug, Insertable)]
#[diesel(table_name = working_hours)]
pub(crate) struct WorkingHoursInsert {
    pub id: String,
    pub shop_id: i32,
    pub weekday: i32,
    pub opens_minute: i32,
    pub closes_minute: i32,
    pub created_at: NaiveDateTime,
}
