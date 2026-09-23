//! The clinic's patient file on the wire (C3 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`).
//! Behind the `clinic` feature like the routes that use it; the generated
//! TypeScript is one folder for every build, so `just types` builds with
//! the feature on.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;
use dzpos_core::services::patients::{NewPatient, Patient, Sex};

/// A patient's file as a screen reads it. The id is the UUID v7's text; the
/// stamps are the shop's clock, `YYYY-MM-DD HH:MM:SS`, like a shift's.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PatientDto.ts")]
pub struct PatientDto {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub sex: Option<SexDto>,
    /// `YYYY-MM-DD`.
    pub date_of_birth: Option<String>,
    /// The digits as stored, a `+` in front at most: the service strips the
    /// spaces and dashes a person types between them.
    pub phone: Option<String>,
    /// Doctor-only (Samir's ruling, 2026-09-23): present, `null` or a
    /// string, for a caller who holds `ViewPatientNotes`; absent for
    /// everyone else, so a receptionist's screen can tell "not mine to
    /// read" from "the doctor left this file's notes empty".
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub notes: Option<Option<String>>,
    /// Null while the file is live; the moment it left the search once it is
    /// archived.
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl PatientDto {
    /// `sees_notes` is `dzpos_core::services::patients::may_see_notes(role)`,
    /// asked once by the route and passed in here rather than re-decided:
    /// this function translates, the service's `may_see_notes` decides.
    pub fn from_patient(p: Patient, sees_notes: bool) -> Self {
        PatientDto {
            id: p.id,
            first_name: p.first_name,
            last_name: p.last_name,
            sex: p.sex.map(SexDto::from),
            date_of_birth: p.date_of_birth.map(|d| d.format(DATE_FORMAT).to_string()),
            phone: p.phone,
            notes: sees_notes.then_some(p.notes),
            archived_at: p
                .archived_at
                .map(|t| t.format(DATE_TIME_FORMAT).to_string()),
            created_at: p.created_at.format(DATE_TIME_FORMAT).to_string(),
            updated_at: p.updated_at.format(DATE_TIME_FORMAT).to_string(),
        }
    }
}

/// The fields a file is written with, on a create and on an update alike.
/// The whole file travels every time: a field left out is a bug at the
/// edge, and a null clears the column.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "PatientWriteDto.ts")]
#[serde(deny_unknown_fields)]
pub struct PatientWriteDto {
    pub first_name: String,
    pub last_name: String,
    pub sex: Option<SexDto>,
    /// `YYYY-MM-DD`, and nothing else.
    pub date_of_birth: Option<String>,
    pub phone: Option<String>,
    pub notes: Option<String>,
}

impl TryFrom<PatientWriteDto> for NewPatient {
    type Error = ApiError;

    fn try_from(d: PatientWriteDto) -> Result<Self, Self::Error> {
        let date_of_birth = match d.date_of_birth.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(day) => Some(parse_day("date_of_birth", day)?),
        };
        Ok(NewPatient {
            first_name: d.first_name,
            last_name: d.last_name,
            sex: d.sex.map(Sex::from),
            date_of_birth,
            phone: d.phone,
            notes: d.notes,
        })
    }
}

/// The `patients.sex` CHECK's two values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "SexDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum SexDto {
    Female,
    Male,
}

impl From<Sex> for SexDto {
    fn from(s: Sex) -> Self {
        match s {
            Sex::Female => SexDto::Female,
            Sex::Male => SexDto::Male,
        }
    }
}

impl From<SexDto> for Sex {
    fn from(s: SexDto) -> Self {
        match s {
            SexDto::Female => Sex::Female,
            SexDto::Male => Sex::Male,
        }
    }
}
