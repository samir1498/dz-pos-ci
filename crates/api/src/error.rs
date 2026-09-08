//! One place maps the core's errors to a status and a body. Every failure
//! leaves as `{ "error": { "code", "message" } }`; the UI translates the
//! code and never shows the message.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use dzpos_core::error::CoreError;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    Core(#[from] CoreError),
    /// A body that did not parse, or a value outside what the wire types
    /// allow. The request never reached a service.
    #[error("{0}")]
    BadRequest(String),
    #[error("no such route")]
    NoRoute,
    #[error("the database connection is unusable")]
    Unavailable,
}

#[derive(Serialize)]
struct Body {
    error: Payload,
}

#[derive(Serialize)]
struct Payload {
    code: &'static str,
    message: String,
}

impl ApiError {
    fn parts(&self) -> (StatusCode, &'static str) {
        match self {
            ApiError::Core(CoreError::Validation { .. } | CoreError::Money(_)) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "validation")
            }
            ApiError::Core(CoreError::NotFound { .. }) => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::Core(CoreError::DuplicateBarcode(_)) => {
                (StatusCode::CONFLICT, "duplicate_barcode")
            }
            ApiError::Core(CoreError::Db(_) | CoreError::Query(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "storage")
            }
            ApiError::BadRequest(_) => (StatusCode::UNPROCESSABLE_ENTITY, "bad_request"),
            ApiError::NoRoute => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::Unavailable => (StatusCode::INTERNAL_SERVER_ERROR, "storage"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = self.parts();
        let message = self.to_string();
        (
            status,
            Json(Body {
                error: Payload { code, message },
            }),
        )
            .into_response()
    }
}
