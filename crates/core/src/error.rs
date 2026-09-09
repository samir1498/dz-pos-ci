//! The core's error enum. It wraps `DbError` and `MoneyError` from the
//! layers below; the API maps it to a status and the UI translates
//! `code()`, so no Rust or SQL text ever reaches a screen.

use crate::db::DbError;
use crate::money::{Money, MoneyError};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{field} is invalid: {message}")]
    Validation { field: String, message: String },
    #[error("{entity} {id} does not exist in this shop")]
    NotFound { entity: &'static str, id: i32 },
    #[error("barcode {0} is already used in this shop")]
    DuplicateBarcode(String),
    /// A payment for more than the customer owes. Its own variant rather than
    /// a `Validation`, because the only useful thing to say back is a figure
    /// the caller never sent: what is outstanding right now. The code stays
    /// `validation`, so a screen that already translates it says the same
    /// sentence and reads the amount out of the payload.
    ///
    /// Money that came in above a debt is an avoir's business (T6), never a
    /// credit balance a payment quietly opened.
    #[error("a payment is never more than what the customer owes")]
    PaymentAboveDebt { outstanding_centimes: i64 },
    /// A number series the shop hands out (in-store barcodes, later the
    /// document numbers) has no next value. Not a validation failure: the
    /// user did nothing wrong, and the API answers 409 so the UI can say
    /// the series is spent rather than "check your input".
    #[error("the {series} series is exhausted")]
    Exhausted { series: &'static str },
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
}

impl CoreError {
    /// Stable key the UI translates. Never the message.
    pub const fn code(&self) -> &'static str {
        match self {
            CoreError::Validation { .. } | CoreError::PaymentAboveDebt { .. } => "validation",
            CoreError::NotFound { .. } => "not_found",
            CoreError::DuplicateBarcode(_) => "duplicate_barcode",
            CoreError::Exhausted { .. } => "exhausted",
            CoreError::CreditLimit { .. } => "credit_limit",
            CoreError::Money(_) => "money",
            CoreError::Db(_) | CoreError::Query(_) | CoreError::Io(_) => "storage",
            CoreError::Render(_) => "print",
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
}
