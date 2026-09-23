//! The patient file (C3 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! open one, correct it, read it, search the list, archive it. Nothing is
//! ever deleted: the queue and the book will point at a patient, and an
//! archived file only leaves the search.
//!
//! Every write is one transaction with its audit row, recorded through the
//! kernel's `services::audit::record`. The row's `entity_id` is an integer
//! column and a patient's id is a UUID, so the id travels in the JSON.
//! The notes do not: the audit log is the owner's to read, and a copy of
//! every medical note in it would be a second place that text lives. The
//! row says whether the file carries notes, not what they are.
//!
//! `notes` itself is doctor-only (C3b, Samir's ruling 2026-09-23):
//! `may_see_notes` is where `create` and `update` ask, and the API's DTO
//! layer asks the same function to decide what a read gets back, so the
//! rule is answered once rather than in the handler.

use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::permissions::{can, Permission, Role};
use dzpos_kernel::services::{audit, bounded_field, clock};

use crate::audit_actions::{ACTION_PATIENT_ARCHIVE, ACTION_PATIENT_CREATE, ACTION_PATIENT_UPDATE};
use crate::models::patient::{PatientChanges, PatientInsert};
use crate::repos::patients as repo;

pub use crate::models::patient::{NewPatient, Patient, Sex};

/// The longest note a file keeps. Longer than the kernel's printable-field
/// bound (200), because notes are never printed on a ticket and a cabinet's
/// running note on a patient is paragraphs, not a line.
pub const MAX_NOTES_CHARS: usize = 4000;

/// The fewest and most digits a phone number may carry once the separators
/// are gone. Four covers an internal extension; fifteen is the longest
/// number ITU-T E.164 allows, and a leading `+` is not a digit.
const PHONE_DIGITS: std::ops::RangeInclusive<usize> = 4..=15;

/// The earliest birth date a file accepts. A typo that lands a patient in
/// the nineteenth century is refused rather than stored.
const EARLIEST_BIRTH: (i32, u32, u32) = (1900, 1, 1);

/// Whether `role` may read or write a patient's notes: the doctor only
/// (Samir's ruling, 2026-09-23, C3b of the clinic plan). The one place this
/// crate asks the kernel's permission table about notes, so `create`,
/// `update` and the API's DTO layer never work the question out twice.
pub fn may_see_notes(role: Role) -> bool {
    can(role, Permission::ViewPatientNotes)
}

/// Whether `fields.notes` actually asks for a value once trimmed, the same
/// rule `notes` below normalises by: a receptionist's form sending `""` or
/// `"   "` for a box it never showed is not "sending notes", so `create`
/// and `update` refuse a caller without `ViewPatientNotes` only when there
/// is a real value in it, and not for a blank the write would have turned
/// into `None` anyway.
fn asks_to_write_notes(fields: &NewPatient) -> bool {
    fields
        .notes
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty())
}

/// Opens a file. The id is a fresh UUID v7 made here, never by the file,
/// and both stamps are the shop's clock.
///
/// A caller without `ViewPatientNotes` who sends a real `notes` value is
/// refused before anything is validated or written; one who leaves it out,
/// or sends a blank, opens a file with none, same as today.
pub fn create(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    fields: NewPatient,
    role: Role,
) -> Result<Patient, CoreError> {
    if asks_to_write_notes(&fields) && !may_see_notes(role) {
        return Err(CoreError::forbidden(Permission::ViewPatientNotes));
    }
    let clean = validate(fields)?;
    let now = clock::now();
    let row = PatientInsert {
        id: uuid::Uuid::now_v7().to_string(),
        shop_id,
        first_name: clean.first_name,
        last_name: clean.last_name,
        sex: clean.sex,
        date_of_birth: clean.date_of_birth,
        phone: clean.phone,
        notes: clean.notes,
        created_at: now,
        updated_at: now,
    };
    conn.transaction(|conn| {
        let created = repo::insert(conn, &row)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: ACTION_PATIENT_CREATE,
                entity: "patient",
                entity_id: None,
                before: None,
                after: Some(as_json(&created)),
            },
        )?;
        Ok(created)
    })
}

/// Rewrites a file with the whole of what the caller sent. An archived file
/// may still be corrected: a wrong date of birth is wrong whether or not the
/// patient still comes.
///
/// A caller without `ViewPatientNotes` who sends a real `notes` value is
/// refused before anything is written; one who leaves it out, or sends a
/// blank, keeps the file's stored notes exactly as they were, rather than
/// the whole-file rewrite clearing them the way an omitted phone number
/// would. A receptionist correcting a phone number can never erase what
/// the doctor wrote, whether their own form sent no `notes` field at all
/// or an empty one.
pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
    fields: NewPatient,
    role: Role,
) -> Result<Patient, CoreError> {
    if asks_to_write_notes(&fields) && !may_see_notes(role) {
        return Err(CoreError::forbidden(Permission::ViewPatientNotes));
    }
    let sees_notes = may_see_notes(role);
    let clean = validate(fields)?;
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        let notes = if sees_notes {
            clean.notes
        } else {
            before.notes.clone()
        };
        let changes = PatientChanges {
            first_name: clean.first_name,
            last_name: clean.last_name,
            sex: clean.sex,
            date_of_birth: clean.date_of_birth,
            phone: clean.phone,
            notes,
            updated_at: clock::now(),
        };
        let after = repo::update(conn, shop_id, id, &changes)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: ACTION_PATIENT_UPDATE,
                entity: "patient",
                entity_id: None,
                before: Some(as_json(&before)),
                after: Some(as_json(&after)),
            },
        )?;
        Ok(after)
    })
}

/// One file of this shop, archived or not. Another shop's id is not found,
/// the same answer as an id nobody ever made.
pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: &str) -> Result<Patient, CoreError> {
    repo::get(conn, shop_id, id)
}

/// The list, or the part of it a search box matches: a piece of the first
/// name, the last name, both in either order, or the phone number however
/// its digits were spaced. Blank text is no filter. Archived files are left
/// out unless `include_archived`.
pub fn search(
    conn: &mut SqliteConnection,
    shop_id: i32,
    text: Option<&str>,
    include_archived: bool,
) -> Result<Vec<Patient>, CoreError> {
    let text = text.map(str::trim).filter(|t| !t.is_empty());
    let digits = text.map(strip_phone_separators);
    let search = text.map(|text| repo::Search {
        text,
        phone: digits
            .as_deref()
            .filter(|d| d.chars().any(|c| c.is_ascii_digit())),
    });
    repo::list(conn, shop_id, search, include_archived)
}

/// Takes a live file out of the search. Archiving one already archived is
/// refused as a conflict, so a second click never moves the stamp or writes
/// a second audit row.
pub fn archive(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
) -> Result<Patient, CoreError> {
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        if before.archived_at.is_some() {
            return Err(CoreError::conflict(
                "archived_at",
                "this patient's file is already archived",
            ));
        }
        let after = repo::archive(conn, shop_id, id, clock::now())?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: ACTION_PATIENT_ARCHIVE,
                entity: "patient",
                entity_id: None,
                before: Some(as_json(&before)),
                after: Some(as_json(&after)),
            },
        )?;
        Ok(after)
    })
}

/// The file with every field checked and normalised: names trimmed and
/// required, blanks made `None`, the phone reduced to its digits, the notes
/// bounded, the birth date inside a life.
fn validate(fields: NewPatient) -> Result<NewPatient, CoreError> {
    Ok(NewPatient {
        first_name: required_name("first_name", &fields.first_name)?,
        last_name: required_name("last_name", &fields.last_name)?,
        sex: fields.sex,
        date_of_birth: birth_date(fields.date_of_birth)?,
        phone: phone(fields.phone.as_deref())?,
        notes: notes(fields.notes.as_deref())?,
    })
}

fn required_name(field: &str, value: &str) -> Result<String, CoreError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(CoreError::validation(
            field,
            "a patient's file needs a name",
        ));
    }
    bounded_field(field, value)?;
    Ok(value.to_string())
}

fn birth_date(value: Option<NaiveDate>) -> Result<Option<NaiveDate>, CoreError> {
    let Some(day) = value else {
        return Ok(None);
    };
    let (y, m, d) = EARLIEST_BIRTH;
    if NaiveDate::from_ymd_opt(y, m, d).is_some_and(|earliest| day < earliest) {
        return Err(CoreError::validation(
            "date_of_birth",
            "a date of birth before 1900 is a typing mistake",
        ));
    }
    if day > clock::now().date() {
        return Err(CoreError::validation(
            "date_of_birth",
            "a date of birth cannot be after today",
        ));
    }
    Ok(Some(day))
}

/// The separators a person types between the digits of a number, taken out
/// so `0555 12-34.56` and `0555123456` are the same number to the search.
fn strip_phone_separators(value: &str) -> String {
    value
        .chars()
        .filter(|c| !matches!(c, ' ' | '.' | '-' | '(' | ')'))
        .collect()
}

fn phone(value: Option<&str>) -> Result<Option<String>, CoreError> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    let stripped = strip_phone_separators(value);
    let digits = stripped.strip_prefix('+').unwrap_or(&stripped);
    if !digits.chars().all(|c| c.is_ascii_digit()) || !PHONE_DIGITS.contains(&digits.len()) {
        return Err(CoreError::validation(
            "phone",
            "a phone number is 4 to 15 digits, with a + in front at most",
        ));
    }
    Ok(Some(stripped))
}

fn notes(value: Option<&str>) -> Result<Option<String>, CoreError> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    if value.chars().count() > MAX_NOTES_CHARS {
        return Err(CoreError::validation(
            "notes",
            "longer than a patient's notes may be (4000 characters)",
        ));
    }
    Ok(Some(value.to_string()))
}

/// The file as the audit row stores it: every field but the notes, which the
/// module doc says why it leaves out.
fn as_json(p: &Patient) -> String {
    serde_json::json!({
        "id": p.id,
        "first_name": p.first_name,
        "last_name": p.last_name,
        "sex": p.sex.map(Sex::as_str),
        "date_of_birth": p.date_of_birth.map(|d| d.format("%Y-%m-%d").to_string()),
        "phone": p.phone,
        "has_notes": p.notes.is_some(),
        "archived_at": p.archived_at.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
    })
    .to_string()
}
