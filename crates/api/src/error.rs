//! One place maps the core's errors to a status and a body. Every failure
//! leaves as `{ "error": { "code", "message" } }`; the UI translates the
//! code and never shows the message.

use axum::extract::rejection::JsonRejection;
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
        CoreError::DuplicateBarcode(_) | CoreError::Exhausted { .. } => StatusCode::CONFLICT,
        CoreError::Money(_) | CoreError::Db(_) | CoreError::Query(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

/// serde's own rejection text carries its internals and the whole DTO field
/// list ("Failed to parse the request body as JSON: name: EOF while
/// parsing..."). None of it helps a caller and all of it describes the
/// server, so the body a caller sees is one of two fixed sentences.
impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        match unknown_field(&rejection.body_text()) {
            Some(field) => ApiError::BadRequest(format!("unknown field {field}")),
            None => ApiError::BadRequest("invalid JSON body".to_string()),
        }
    }
}

/// The field name serde names between backticks after "unknown field". serde
/// gives no structured form of it, so the text is where it has to come from.
fn unknown_field(text: &str) -> Option<String> {
    let after = text.split_once("unknown field `")?.1;
    // serde closes the name with "`, expected ..." (or "` at line" when
    // there is nothing to expect), so the name runs up to that marker, not
    // to the first backtick: a key like bo`gus is reported whole.
    let end = after
        .find("`, expected")
        .or_else(|| after.find("` at line"))
        .or_else(|| after.rfind('`'))?;
    let name = &after[..end];
    if name.is_empty() || name.contains(char::is_whitespace) {
        return None;
    }
    Some(name.to_string())
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

#[cfg(test)]
mod unknown_field_tests {
    use super::unknown_field;

    #[test]
    fn the_whole_name_is_kept_even_with_a_backtick_inside() {
        let text = "Failed to deserialize the JSON body into the target type: \
                    unknown field `bo`gus`, expected one of `name`, `barcode` at line 1 column 12";
        assert_eq!(unknown_field(text).as_deref(), Some("bo`gus"));
    }

    #[test]
    fn a_plain_name_and_a_missing_marker_still_parse() {
        assert_eq!(
            unknown_field("unknown field `bogus`, expected `name`").as_deref(),
            Some("bogus")
        );
        assert_eq!(
            unknown_field("unknown field `bogus` at line 1").as_deref(),
            Some("bogus")
        );
        assert_eq!(unknown_field("something else"), None);
    }
}
