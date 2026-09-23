//! One appointment in the book: a patient given a slot of the one doctor's
//! day, at a start on the slot grid, for the slot length in force when it
//! was booked.

use chrono::{Duration, NaiveDateTime};
use diesel::prelude::*;

use crate::schema::appointments;

dzpos_kernel::text_enum! {
    /// The `appointments.call_outcome` CHECK's two values: what came of the
    /// desk's confirmation call (C6b).
    CallOutcome {
        Confirmed => "confirmed",
        NoAnswer => "no_answer",
    }
}

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
    /// How long it runs, 1 to 240: its visit type's length, or the slot
    /// length without one, copied when it was booked and kept by a move.
    pub slot_minutes: i32,
    /// A short reason for the visit; never clinical notes.
    pub note: Option<String>,
    /// `None` while the slot is held; the moment it was given back after.
    pub cancelled_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    /// `None` unless the desk marked the patient as not coming, before the
    /// start or after it; never beside `cancelled_at`.
    pub no_show_at: Option<NaiveDateTime>,
    /// What came of the desk's confirmation call; `None` before a call or
    /// once cleared (C6b).
    pub call_outcome: Option<CallOutcome>,
    /// When that outcome was recorded; set exactly when there is one.
    pub call_at: Option<NaiveDateTime>,
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
    /// The number the desk rings to confirm (C6b), as the file holds it.
    pub phone: Option<String>,
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
