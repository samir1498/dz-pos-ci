//! One arrival in the waiting room: a patient who came in on a day, and the
//! moments they were called, seen, or left unseen
//! (`context/research/20260922-a-doctors-cabinet-day-on-paper.md`).

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;

use crate::schema::queue_entries;

/// A queue entry as the table holds it. The row and the model are one
/// struct, as a patient's are.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Selectable)]
#[diesel(table_name = queue_entries)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct QueueEntry {
    /// A UUID v7, as its 36-character hyphenated text.
    pub id: String,
    pub shop_id: i32,
    pub patient_id: String,
    /// The shop clock's local day the patient arrived on.
    pub day: NaiveDate,
    pub arrived_at: NaiveDateTime,
    /// `None` until the doctor calls the patient in.
    pub called_at: Option<NaiveDateTime>,
    /// `None` until the patient has been seen; never set without `called_at`.
    pub seen_at: Option<NaiveDateTime>,
    /// Set when the patient left without being seen; never beside `seen_at`.
    pub left_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl QueueEntry {
    /// Still waiting or in with the doctor: neither seen nor gone. The one
    /// entry per patient per day the migration's partial index allows.
    pub const fn is_live(&self) -> bool {
        self.seen_at.is_none() && self.left_at.is_none()
    }
}

/// An entry with the names of the patient it points at, the way the day's
/// list and every answer about one entry are read: a queue of ids is not
/// something the desk can call out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedPatient {
    pub entry: QueueEntry,
    pub first_name: String,
    pub last_name: String,
}

/// An entry as the service writes it the first time: the id, the day and
/// every stamp made by the service.
#[derive(Debug, Insertable)]
#[diesel(table_name = queue_entries)]
pub(crate) struct QueueEntryInsert {
    pub id: String,
    pub shop_id: i32,
    pub patient_id: String,
    pub day: NaiveDate,
    pub arrived_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
