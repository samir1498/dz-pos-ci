//! A cabinet's patient file: identity and notes, nothing owed
//! (`context/research/20260922-what-clinic-software-provides.md`).

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;

use crate::schema::patients;

dzpos_kernel::text_enum! {
    /// The `patients.sex` CHECK's two values, on the Rust side. Optional on
    /// the file: a patient is registered at the desk before anyone asks.
    Sex {
        Female => "female",
        Male => "male",
    }
}

/// A patient as the rest of the app sees it. The row and the model are one
/// struct: no column needs converting on the way out, the way a customer's
/// `*_centimes` columns become `Money`.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Selectable)]
#[diesel(table_name = patients)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Patient {
    /// A UUID v7, as its 36-character hyphenated text.
    pub id: String,
    pub shop_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub sex: Option<Sex>,
    pub date_of_birth: Option<NaiveDate>,
    pub phone: Option<String>,
    pub notes: Option<String>,
    /// `None` while the file is live; the moment it left the search when
    /// it is archived.
    pub archived_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// What a caller hands over, on a create and on an update alike: the whole
/// file, never a patch, so a field left empty clears the column. Raw as
/// typed; `services::patients` trims, checks and normalises it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewPatient {
    pub first_name: String,
    pub last_name: String,
    pub sex: Option<Sex>,
    pub date_of_birth: Option<NaiveDate>,
    pub phone: Option<String>,
    pub notes: Option<String>,
}

/// A file as the service writes it the first time: every column, the id and
/// both stamps made by the service rather than left to the file.
#[derive(Debug, Insertable)]
#[diesel(table_name = patients)]
pub(crate) struct PatientInsert {
    pub id: String,
    pub shop_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub sex: Option<Sex>,
    pub date_of_birth: Option<NaiveDate>,
    pub phone: Option<String>,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// The columns an update rewrites. `treat_none_as_null`: the update carries
/// the whole file, so a phone number taken off has to reach the column;
/// without it diesel reads `None` as "leave this one alone".
///
/// `archived_at` is not here: archiving is its own write, with its own audit
/// action, and an update never takes a file in or out of the search.
#[derive(Debug, AsChangeset)]
#[diesel(table_name = patients, treat_none_as_null = true)]
pub(crate) struct PatientChanges {
    pub first_name: String,
    pub last_name: String,
    pub sex: Option<Sex>,
    pub date_of_birth: Option<NaiveDate>,
    pub phone: Option<String>,
    pub notes: Option<String>,
    pub updated_at: NaiveDateTime,
}
