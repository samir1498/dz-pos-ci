//! A period the doctor is away: the book takes no appointment inside it.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::models::appointment::BookedPatient;
use crate::schema::absence_blocks;

/// A block as the table holds it. Half-open: an appointment may end on
/// `starts_at` or start on `ends_at`.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Selectable, Insertable)]
#[diesel(table_name = absence_blocks)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AbsenceBlock {
    /// A UUID v7, as its 36-character hyphenated text.
    pub id: String,
    pub shop_id: i32,
    pub starts_at: NaiveDateTime,
    pub ends_at: NaiveDateTime,
    /// A short word for the desk, never a reason about a patient.
    pub label: Option<String>,
    pub created_at: NaiveDateTime,
}

/// A new block and every live appointment it lands on, in time order: the
/// desk moves or cancels each one, the block itself touches none of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockMade {
    pub block: AbsenceBlock,
    pub hits: Vec<BookedPatient>,
}
