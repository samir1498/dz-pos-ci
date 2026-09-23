//! The day list (item 6 of the book tools, C5b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! one day of the cabinet as the desk reads it out, the booked appointments
//! in time order beside that day's walk-in queue in arrival order. Data
//! only: printing it is the screens' work (C6).
//!
//! The appointments are the book's live ones, a missed one with its mark
//! included; the queue is every entry of the day, waiting, called, seen or
//! gone, as `queue` keeps them.

use chrono::NaiveDate;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;

use crate::repos::queue as queue_repo;
use crate::services::appointments::{self, BookedPatient};
use crate::services::queue::QueuedPatient;

/// One day of the cabinet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayList {
    pub day: NaiveDate,
    pub appointments: Vec<BookedPatient>,
    pub walk_ins: Vec<QueuedPatient>,
}

/// The day's appointments and walk-ins, each with the patient's names.
pub fn day(
    conn: &mut SqliteConnection,
    shop_id: i32,
    day: NaiveDate,
) -> Result<DayList, CoreError> {
    Ok(DayList {
        day,
        appointments: appointments::day(conn, shop_id, day)?,
        walk_ins: queue_repo::day_list(conn, shop_id, day)?,
    })
}
