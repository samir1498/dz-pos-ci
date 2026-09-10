//! The core's error enum. It wraps `DbError` and `MoneyError` from the
//! layers below; the API maps it to a status and the UI translates
//! `code()`, so no Rust or SQL text ever reaches a screen.

use crate::db::DbError;
use crate::money::{Money, MoneyError};

/// Which half of a facture a `PartyIds` refusal is about. The two blocks are
/// filled in from two different screens, so the side is what tells the till
/// whether to send the cashier to the settings or to the customer's fiche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartySide {
    Seller,
    Buyer,
}

impl PartySide {
    /// The stable key the UI translates, the way an error code is.
    pub const fn as_str(self) -> &'static str {
        match self {
            PartySide::Seller => "seller",
            PartySide::Buyer => "buyer",
        }
    }
}

impl std::fmt::Display for PartySide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{field} is invalid: {message}")]
    Validation { field: String, message: String },
    #[error("{entity} {id} does not exist in this shop")]
    NotFound { entity: &'static str, id: i32 },
    #[error("barcode {0} is already used in this shop")]
    DuplicateBarcode(String),
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
    /// A payment for more than is owed, on either ledger. Its own variant
    /// rather than a `Validation`, because the only useful thing to say back
    /// is a figure the caller never sent: what is outstanding right now. The
    /// code stays `validation`, so a screen that already translates it says
    /// the same sentence and reads the amount out of the payload.
    ///
    /// The party is not named, because both sides raise it: money over what a
    /// customer owes is an avoir's business and never a credit balance a
    /// payment quietly opened, and money over what the shop owes a supplier
    /// is an advance somebody writes on purpose.
    #[error("a payment is never more than what is owed")]
    PaymentAboveDebt { outstanding_centimes: i64 },
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
    /// A credit sale the customer's limit will not carry (features.md §1).
    /// Not a validation failure: every field the caller sent is well formed,
    /// and what refuses the sale is what the customer already owes. The two
    /// amounts travel with the code because the till has to say by how much
    /// and against what, and a screen may not re-derive either.
    #[error(
        "this sale would leave {} centimes owed against a credit limit of {} centimes",
        balance_after.as_centimes(),
        credit_limit.as_centimes()
    )]
    CreditLimit {
        balance_after: Money,
        credit_limit: Money,
    },
    /// A facture whose party blocks do not carry what décret 05-468 art. 3
    /// asks of them (`a_company_buyer_without_a_nis_refuses_the_facture_and_burns_no_number`). Not a validation failure
    /// on a field the caller sent: the basket is well formed and what
    /// refuses the paper sits on the settings page or on the customer's
    /// fiche. The side and the identifiers travel with the code because the
    /// till has to say where to go and what is missing, and a screen may not
    /// work either out from the rule (architecture.md rule 2).
    #[error("the {side} block of a facture is missing {}", missing.join(", "))]
    PartyIds {
        side: PartySide,
        missing: Vec<&'static str>,
    },
    /// A ledger row handed to a repo with no moment on it. The column's
    /// default is SQLite's CURRENT_TIMESTAMP, which is UTC, while every
    /// period this app answers for is a stretch of days on the shop's
    /// calendar (UTC+1): a payment taken at 00:30 in Algiers would be stored
    /// on the day before and fall out of the day the shop counted its
    /// drawer. The caller stamps it from `services::clock`.
    ///
    /// Its own variant and not a `Validation`: no field a caller sent is
    /// wrong, and nobody using the app can correct it. It is a mistake in
    /// this crate, so the API answers 500 and the code is the storage one.
    #[error("a row of {entity} is stamped from the shop clock, never left to the file's default")]
    Unstamped { entity: &'static str },
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
    /// A printed template failed to render. The template and the data it is
    /// given are both the app's own, so this is a bug in the app and never
    /// something a caller can correct; the API answers 500 and the message
    /// stays fixed, like the file one.
    #[error("the document could not be rendered for printing")]
    Render(#[from] askama::Error),
    /// A workbook this app was writing could not be finished. Like `Render`,
    /// the data and the layout are both the app's own, so this is a bug here
    /// and never something a caller can correct; the message is fixed and the
    /// writer's own text stays on the source chain for the server's log.
    ///
    /// Reading a workbook is not this: a file a shop uploaded is input, and a
    /// file that is not a workbook at all comes back as a `Validation` the
    /// import screen can put under the file picker.
    #[error("the workbook could not be written")]
    Workbook(#[from] rust_xlsxwriter::XlsxError),
}

impl CoreError {
    /// Stable key the UI translates. Never the message.
    pub const fn code(&self) -> &'static str {
        match self {
            CoreError::Validation { .. } | CoreError::PaymentAboveDebt { .. } => "validation",
            CoreError::NotFound { .. } => "not_found",
            CoreError::DuplicateBarcode(_) => "duplicate_barcode",
            CoreError::Conflict { .. } => "conflict",
            CoreError::Exhausted { .. } => "exhausted",
            CoreError::CreditLimit { .. } => "credit_limit",
            CoreError::PartyIds { .. } => "party_ids",
            CoreError::Money(_) => "money",
            CoreError::Db(_)
            | CoreError::Query(_)
            | CoreError::Io(_)
            | CoreError::Unstamped { .. } => "storage",
            CoreError::Render(_) => "print",
            CoreError::Workbook(_) => "workbook",
        }
    }

    /// A document the printer refuses. `reason` says which rule the stored
    /// row breaks; it stays on the server, on the error's source chain,
    /// because the wire message for a render failure is fixed.
    pub fn render(reason: &'static str) -> Self {
        CoreError::Render(askama::Error::custom(reason))
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
}
