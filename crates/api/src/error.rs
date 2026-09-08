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
    /// A core error a service raised. The request was well formed and a rule
    /// or the file decided.
    #[error(transparent)]
    Core(#[from] CoreError),
    /// A core error raised while reading the request itself, before any
    /// service ran. The caller wrote the value, so it is a 422 whatever the
    /// same error would mean coming out of a read.
    #[error(transparent)]
    Request(CoreError),
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
            // The code is the core's own (architecture.md: map, never
            // re-derive). Only the status is the API's to choose.
            ApiError::Core(e) => (status_for(e), e.code()),
            ApiError::Request(e) => (StatusCode::UNPROCESSABLE_ENTITY, e.code()),
            ApiError::BadRequest(_) => (StatusCode::UNPROCESSABLE_ENTITY, "bad_request"),
            ApiError::NoRoute => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::Unavailable => (StatusCode::INTERNAL_SERVER_ERROR, "storage"),
        }
    }
}

/// What a core error means once a service has run. A `Money` error here is a
/// stored row the migration's CHECK makes impossible, so the file is wrong
/// and the caller has nothing to correct: 500, not 422.
const fn status_for(e: &CoreError) -> StatusCode {
    match e {
        CoreError::Validation { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        CoreError::NotFound { .. } => StatusCode::NOT_FOUND,
        CoreError::DuplicateBarcode(_) => StatusCode::CONFLICT,
        CoreError::Money(_) | CoreError::Db(_) | CoreError::Query(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
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
