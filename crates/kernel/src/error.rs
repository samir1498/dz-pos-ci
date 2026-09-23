//! The core's error enum. It wraps `DbError` and `MoneyError` from the
//! layers below; the API maps it to a status and the UI translates
//! `code()`, so no Rust or SQL text ever reaches a screen.
//!
//! Domain-free since S4 of `a-kernel-crate-and-retail-as-the-first-module`:
//! the six variants that named a shop concept (`DuplicateBarcode`,
//! `PaymentAboveDebt`, `CreditLimit`, `Unstamped`, `UnpricedReversal`,
//! `PartyIds`, and `PartySide` beside it) and the two that wrapped a
//! retail-only dependency for no shop reason at all (`Render`, wrapping
//! `askama::Error`; `Workbook`, wrapping `rust_xlsxwriter::XlsxError`, both
//! raised only by the print and export code S3 already moved to
//! `dzpos-retail` whole) moved to `dzpos_retail::error::RetailError`, which
//! wraps this enum rather than repeating it. `crates/api/src/error.rs` maps
//! both.

use crate::db::DbError;
use crate::money::MoneyError;
use crate::services::permissions::Permission;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{field} is invalid: {message}")]
    Validation { field: String, message: String },
    #[error("{entity} {id} does not exist in this shop")]
    NotFound { entity: &'static str, id: i32 },
    /// A value another row of this shop already holds, where the file says
    /// only one may. Not a `Validation`: what the caller sent is well formed
    /// and what refuses it is a row that is already there, so the screen has
    /// a different sentence to say and a different thing to offer. The field
    /// travels so the message lands under the input.
    ///
    /// `DuplicateBarcode` predates this and keeps its own code: a barcode is
    /// answered by offering to generate one, which is not what any other
    /// clash offers.
    #[error("{field} is already used in this shop: {message}")]
    Conflict { field: String, message: String },
    /// A number series the shop hands out (in-store barcodes, the document
    /// numbers) has no next value. Not a validation failure: the user did
    /// nothing wrong, and the API answers 409 so the UI can say the series is
    /// spent rather than "check your input".
    ///
    /// Owned rather than `&'static str`: a document series is named for the
    /// year it counts in (`doc_facture:2026`, features.md §4) and the year is
    /// read at run time.
    #[error("the {series} series is exhausted")]
    Exhausted { series: String },
    /// A credential that did not match: a wrong PIN, a wrong password, a name
    /// nobody in the shop answers to, or a user who has been deactivated.
    ///
    /// One variant for all four on purpose. The password screen asks for a
    /// name, so an error that said "no such user" would let anybody standing
    /// at the till read the staff list off the login box one guess at a time,
    /// and one that said "deactivated" would say who used to work here. The
    /// message names no field for the same reason.
    #[error("that is not a credential this shop accepts")]
    AuthRefused,
    /// Too many wrong credentials on one user: the shop counter is a public
    /// place and the file makes whoever is standing at it wait
    /// (features.md §5). The wait travels with the code because the screen has
    /// to count it down, and it is a figure the caller never sent.
    #[error("too many wrong attempts; this user may try again in {retry_after_seconds} seconds")]
    LockedOut { retry_after_seconds: i64 },
    /// A credential could not be hashed, or a stored hash could not be read
    /// back as one. Its own variant and not a `Validation`: the input is
    /// bytes this crate chose the shape of and the parameters are its own,
    /// so a failure here is a bug in the app and never something a caller
    /// can correct. The API answers 500 and the message is fixed; the cause
    /// stays on the source chain for the server's log.
    ///
    /// Never raised by a credential that simply did not match, and never by
    /// the `'!unset'` sentinel: an unparseable stored hash on a sign-in is a
    /// refusal, so the file fails closed rather than 500ing its way open.
    #[error("the credential could not be hashed")]
    Hash(#[source] argon2::password_hash::Error),
    /// A role asked for something `services::permissions::can` refuses. The
    /// permission travels so the caller can say which one was missing
    /// instead of a bare "forbidden" (M4 T2 puts this on the wire as the
    /// `forbidden` code and the permission's name).
    #[error("this role does not have the {permission} permission")]
    Forbidden { permission: Permission },
    #[error(transparent)]
    Money(#[from] MoneyError),
    #[error(transparent)]
    Db(#[from] DbError),
    #[error("query failed: {0}")]
    Query(#[from] diesel::result::Error),
    /// A file the app owns (a backup copy, the shop file being replaced)
    /// could not be read, written or moved. The message is fixed on purpose:
    /// `io::Error` prints the path it failed on, and no path belongs on the
    /// wire. The cause stays on the `source` for the server's own log.
    #[error("the shop's files could not complete the operation")]
    Io(#[from] std::io::Error),
}

impl CoreError {
    /// Stable key the UI translates. Never the message.
    pub const fn code(&self) -> &'static str {
        match self {
            CoreError::Validation { .. } => "validation",
            CoreError::NotFound { .. } => "not_found",
            CoreError::Conflict { .. } => "conflict",
            CoreError::Exhausted { .. } => "exhausted",
            CoreError::AuthRefused => "auth_refused",
            CoreError::LockedOut { .. } => "locked_out",
            CoreError::Forbidden { .. } => "forbidden",
            CoreError::Money(_) => "money",
            CoreError::Db(_) | CoreError::Query(_) | CoreError::Io(_) | CoreError::Hash(_) => {
                "storage"
            }
        }
    }

    pub fn validation(field: &str, message: &str) -> Self {
        CoreError::Validation {
            field: field.to_string(),
            message: message.to_string(),
        }
    }

    pub fn conflict(field: &str, message: &str) -> Self {
        CoreError::Conflict {
            field: field.to_string(),
            message: message.to_string(),
        }
    }

    pub const fn forbidden(permission: Permission) -> Self {
        CoreError::Forbidden { permission }
    }
}
