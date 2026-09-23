//! One appointment in the book: a patient given a slot of the one doctor's
//! day, at a start on the slot grid, for the slot length in force when it
//! was booked.

use chrono::{Duration, NaiveDateTime};
use diesel::prelude::*;

use crate::schema::appointments;

/// An appointment as the table holds it. The row and the model are one
/// struct, as a patient's are.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Selectable)]
#[diesel(table_name = appointments)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Appointment {
    /// A UUID v7, as its 36-character hyphenated text.
    pub id: String,
    pub shop_id: i32,
    pub patient_id: String,
    /// The shop clock's local start, on a whole minute of the slot grid.
    pub starts_at: NaiveDateTime,
    /// The slot length when it was booked or last moved, 1 to 240.
    pub slot_minutes: i32,
    /// A short reason for the visit; never clinical notes.
    pub note: Option<String>,
    /// `None` while the slot is held; the moment it was given back after.
    pub cancelled_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Appointment {
    /// Still holding its slot: the rows the migration's partial index counts.
    pub const fn is_live(&self) -> bool {
        self.cancelled_at.is_none()
    }

    /// The first minute after the slot. `None` only past the end of chrono's
    /// calendar, which a start the table accepted cannot reach; the caller
    /// treats it as reaching to the end of time, the safe side for an overlap.
    pub fn ends_at(&self) -> Option<NaiveDateTime> {
        self.starts_at
            .checked_add_signed(Duration::minutes(i64::from(self.slot_minutes)))
    }
}

/// An appointment with the names of the patient it points at, the way every
/// read of the book answers: a slot with only an id in it is not something
/// the desk can read out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookedPatient {
    pub appointment: Appointment,
    pub first_name: String,
    pub last_name: String,
}

/// An appointment as the service writes it the first time: the id, the slot
/// length and every stamp made by the service.
#[derive(Debug, Insertable)]
#[diesel(table_name = appointments)]
pub(crate) struct AppointmentInsert {
    pub id: String,
    pub shop_id: i32,
    pub patient_id: String,
    pub starts_at: NaiveDateTime,
    pub slot_minutes: i32,
    pub note: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
