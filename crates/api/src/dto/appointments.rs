//! The clinic's appointment book on the wire (C5 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`).
//! Behind the `clinic` feature like the routes that use it; `just types`
//! builds with the feature on, as it does for the patient file and the
//! queue.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;
use chrono::NaiveDateTime;
use dzpos_core::services::appointments::{BookedPatient, CallOutcome};

/// One slot in the book, with the names the desk reads out. `starts_at` is
/// the shop clock's local time, `YYYY-MM-DD HH:MM:SS`, the format a booking
/// sends it in.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "AppointmentDto.ts")]
pub struct AppointmentDto {
    pub id: String,
    pub patient_id: String,
    pub first_name: String,
    pub last_name: String,
    pub starts_at: String,
    /// How long it runs: its visit type's length, or the slot length
    /// without one, copied when it was booked and kept by a move, so it may
    /// differ from today's setting and today's type.
    pub slot_minutes: i32,
    /// A short reason for the visit, never clinical notes.
    pub note: Option<String>,
    /// Null while the slot is held. The day and week lists carry live ones
    /// only; a cancel answers with the row stamped.
    pub cancelled_at: Option<String>,
    /// When the desk marked the patient as not coming; null while unmarked.
    /// Any start, past or not; never beside `cancelled_at`.
    pub no_show_at: Option<String>,
    /// The patient's phone as the file holds it, for the confirmation call.
    pub phone: Option<String>,
    /// What came of the confirmation call; null before one or once cleared.
    pub call_outcome: Option<CallOutcomeDto>,
    /// When that outcome was recorded; set exactly when there is one.
    pub call_at: Option<String>,
}

/// What came of a confirmation call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "CallOutcomeDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum CallOutcomeDto {
    Confirmed,
    NoAnswer,
}

impl From<CallOutcome> for CallOutcomeDto {
    fn from(c: CallOutcome) -> Self {
        match c {
            CallOutcome::Confirmed => CallOutcomeDto::Confirmed,
            CallOutcome::NoAnswer => CallOutcomeDto::NoAnswer,
        }
    }
}

impl From<CallOutcomeDto> for CallOutcome {
    fn from(c: CallOutcomeDto) -> Self {
        match c {
            CallOutcomeDto::Confirmed => CallOutcome::Confirmed,
            CallOutcomeDto::NoAnswer => CallOutcome::NoAnswer,
        }
    }
}

/// What came of the call the desk just made.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export_to = "AppointmentCallDto.ts")]
#[serde(deny_unknown_fields)]
pub struct AppointmentCallDto {
    pub outcome: CallOutcomeDto,
}

impl From<BookedPatient> for AppointmentDto {
    fn from(b: BookedPatient) -> Self {
        AppointmentDto {
            id: b.appointment.id,
            patient_id: b.appointment.patient_id,
            first_name: b.first_name,
            last_name: b.last_name,
            starts_at: b.appointment.starts_at.format(DATE_TIME_FORMAT).to_string(),
            slot_minutes: b.appointment.slot_minutes,
            note: b.appointment.note,
            cancelled_at: b
                .appointment
                .cancelled_at
                .map(|t| t.format(DATE_TIME_FORMAT).to_string()),
            no_show_at: b
                .appointment
                .no_show_at
                .map(|t| t.format(DATE_TIME_FORMAT).to_string()),
            phone: b.phone,
            call_outcome: b.appointment.call_outcome.map(CallOutcomeDto::from),
            call_at: b
                .appointment
                .call_at
                .map(|t| t.format(DATE_TIME_FORMAT).to_string()),
        }
    }
}

/// A day or a week of the book: the days it covers, first and last
/// (`YYYY-MM-DD`, both included), and the live appointments in time order.
/// A week asked for by any of its days answers from its Sunday, so the
/// screen reads the range here rather than working it out again.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "AppointmentsDto.ts")]
pub struct AppointmentsDto {
    pub from: String,
    pub to: String,
    pub appointments: Vec<AppointmentDto>,
}

/// A booking. The length is the named visit type's, or the cabinet's slot
/// length without one, never a number the caller sends.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "AppointmentBookDto.ts")]
#[serde(deny_unknown_fields)]
pub struct AppointmentBookDto {
    pub patient_id: String,
    /// `YYYY-MM-DD HH:MM:SS` on the shop's clock, on the slot grid.
    pub starts_at: String,
    pub note: Option<String>,
    /// A visit type of the cabinet; may be left out.
    #[serde(default)]
    #[ts(optional)]
    pub visit_type_id: Option<String>,
}

/// Where an appointment moves to.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "AppointmentMoveDto.ts")]
#[serde(deny_unknown_fields)]
pub struct AppointmentMoveDto {
    /// `YYYY-MM-DD HH:MM:SS` on the shop's clock, on the slot grid.
    pub starts_at: String,
}

/// The book's slot length in minutes: read by anyone signed in, set by who
/// may change the other settings. 5 to 120 in steps of 5.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export_to = "SlotMinutesDto.ts")]
#[serde(deny_unknown_fields)]
pub struct SlotMinutesDto {
    pub slot_minutes: u32,
}

/// `YYYY-MM-DD HH:MM:SS` and nothing else, compared back after parsing the
/// way `parse_day` compares a day, so a `T`, a missing second or a fraction
/// answers 422 naming the field rather than booking a moment nobody sent.
pub fn parse_starts_at(text: &str) -> Result<NaiveDateTime, ApiError> {
    NaiveDateTime::parse_from_str(text, DATE_TIME_FORMAT)
        .ok()
        .filter(|t| t.format(DATE_TIME_FORMAT).to_string() == text)
        .ok_or_else(|| {
            ApiError::Request(CoreError::validation(
                "starts_at",
                "an appointment starts at a time written YYYY-MM-DD HH:MM:SS",
            ))
        })
}
