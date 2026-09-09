//! The core's error enum. It wraps `DbError` and `MoneyError` from the
//! layers below; the API maps it to a status and the UI translates
//! `code()`, so no Rust or SQL text ever reaches a screen.

use crate::db::DbError;
use crate::money::MoneyError;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{field} is invalid: {message}")]
    Validation { field: String, message: String },
    #[error("{entity} {id} does not exist in this shop")]
    NotFound { entity: &'static str, id: i32 },
    #[error("barcode {0} is already used in this shop")]
    DuplicateBarcode(String),
    /// A number series the shop hands out (in-store barcodes, later the
    /// document numbers) has no next value. Not a validation failure: the
    /// user did nothing wrong, and the API answers 409 so the UI can say
    /// the series is spent rather than "check your input".
    #[error("the {series} series is exhausted")]
    Exhausted { series: &'static str },
    #[error(transparent)]
    Money(#[from] MoneyError),
    #[error(transparent)]
    Db(#[from] DbError),
    #[error("query failed: {0}")]
    Query(#[from] diesel::result::Error),
}

impl CoreError {
    /// Stable key the UI translates. Never the message.
    pub const fn code(&self) -> &'static str {
        match self {
            CoreError::Validation { .. } => "validation",
            CoreError::NotFound { .. } => "not_found",
            CoreError::DuplicateBarcode(_) => "duplicate_barcode",
            CoreError::Exhausted { .. } => "exhausted",
            CoreError::Money(_) => "money",
            CoreError::Db(_) | CoreError::Query(_) => "storage",
        }
    }

    pub fn validation(field: &str, message: &str) -> Self {
        CoreError::Validation {
            field: field.to_string(),
            message: message.to_string(),
        }
    }
}
