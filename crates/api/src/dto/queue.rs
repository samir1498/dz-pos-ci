//! The clinic's waiting queue on the wire (C4 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`).
//! Behind the `clinic` feature like the routes that use it; `just types`
//! builds with the feature on, as it does for the patient file.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;
use dzpos_core::services::queue::QueuedPatient;

/// One arrival in the day's queue, with the names the desk calls out. The
/// stamps are the shop's clock, `YYYY-MM-DD HH:MM:SS`, like a patient's.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "QueueEntryDto.ts")]
pub struct QueueEntryDto {
    pub id: String,
    pub patient_id: String,
    pub first_name: String,
    pub last_name: String,
    /// `YYYY-MM-DD`, the shop clock's day the patient arrived on.
    pub day: String,
    pub arrived_at: String,
    /// Null until the patient is called in.
    pub called_at: Option<String>,
    /// Null until the patient is seen; never set without `called_at`.
    pub seen_at: Option<String>,
    /// Set when the patient left without being seen; never beside `seen_at`.
    pub left_at: Option<String>,
    /// The appointment a booked patient was marked arrived for; null for a
    /// walk-in.
    pub appointment_id: Option<String>,
    /// That appointment's start, `YYYY-MM-DD HH:MM:SS`; null for a walk-in.
    pub appointment_starts_at: Option<String>,
    /// The entry's place in its day's list as the desk ordered it, 1 first.
    pub position: i32,
}

impl From<QueuedPatient> for QueueEntryDto {
    fn from(q: QueuedPatient) -> Self {
        let stamp =
            |t: Option<chrono::NaiveDateTime>| t.map(|t| t.format(DATE_TIME_FORMAT).to_string());
        QueueEntryDto {
            id: q.entry.id,
            patient_id: q.entry.patient_id,
            first_name: q.first_name,
            last_name: q.last_name,
            day: q.entry.day.format(DATE_FORMAT).to_string(),
            arrived_at: q.entry.arrived_at.format(DATE_TIME_FORMAT).to_string(),
            called_at: stamp(q.entry.called_at),
            seen_at: stamp(q.entry.seen_at),
            left_at: stamp(q.entry.left_at),
            appointment_id: q.entry.appointment_id,
            appointment_starts_at: stamp(q.appointment_starts_at),
            position: q.entry.position,
        }
    }
}

/// Who arrived. The day and the moment are the shop clock's, never the
/// caller's.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "QueueAddDto.ts")]
#[serde(deny_unknown_fields)]
pub struct QueueAddDto {
    pub patient_id: String,
}

/// Today's queue in the order the desk dragged it into: entry ids, first
/// place first. An entry of today left out keeps its order after the ones
/// listed.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "QueueOrderDto.ts")]
#[serde(deny_unknown_fields)]
pub struct QueueOrderDto {
    pub ids: Vec<String>,
}
