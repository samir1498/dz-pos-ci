//! One place maps the core's errors to a status and a body. Every failure
//! leaves as `{ "error": { "code", "message" } }`; the UI translates the
//! code and never shows the message.

use axum::extract::rejection::JsonRejection;
use axum::http::{header, HeaderValue, StatusCode};
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
    /// No launch token, or the wrong one. The answer is the same for a
    /// missing route so a stranger cannot map the API by its 404s.
    #[error("this call did not show the launch token")]
    Unauthorized,
    #[error("no such route")]
    NoRoute,
    #[error("this route does not take that method")]
    MethodNotAllowed,
    #[error("the database connection is unusable")]
    Unavailable,
    /// The shop file is not open in this process and nothing here will open
    /// it again. A restore closed it and could not get it back, so the file
    /// on disk is whole and this process is the part that is broken. The one
    /// thing that helps is relaunching, and the code says so rather than
    /// leaving the screen to guess at "storage".
    #[error("the shop file is not open in this app any more; close it and start it again")]
    RestartNeeded,
    /// The same, plus the half a person needs to hear first: the restore did
    /// not happen. The shop file was never renamed over, so what is on disk
    /// is the state that was always there, and the copy the owner picked was
    /// not put in place. Its own code, because the answer after the relaunch
    /// is different: the till comes back on the old data, not the restored
    /// data.
    #[error(
        "the restore did not happen and the shop file could not be reopened; nothing was replaced, so close the app and start it again"
    )]
    NotRestoredRestartNeeded,
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
    /// What the wire carries as `message`. A storage fault keeps its SQL
    /// text on the server: the driver names tables and columns, and
    /// docs/architecture.md promises no Rust or SQL text ever reaches a
    /// screen.
    fn message(&self) -> String {
        match self {
            ApiError::Core(CoreError::Query(_) | CoreError::Db(_))
            | ApiError::Request(CoreError::Query(_) | CoreError::Db(_)) => {
                "the shop file could not complete the operation".to_owned()
            }
            other => other.to_string(),
        }
    }

    fn parts(&self) -> (StatusCode, &'static str) {
        match self {
            // The code is the core's own (architecture.md: map, never
            // re-derive). Only the status is the API's to choose.
            ApiError::Core(e) => (status_for(e), e.code()),
            ApiError::Request(e) => (StatusCode::UNPROCESSABLE_ENTITY, e.code()),
            ApiError::BadRequest(_) => (StatusCode::UNPROCESSABLE_ENTITY, "bad_request"),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::NoRoute => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, "method_not_allowed"),
            ApiError::Unavailable => (StatusCode::INTERNAL_SERVER_ERROR, "storage"),
            ApiError::RestartNeeded => (StatusCode::INTERNAL_SERVER_ERROR, "restart_needed"),
            ApiError::NotRestoredRestartNeeded => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "restore_failed_restart_needed",
            ),
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
        CoreError::Money(_) | CoreError::Db(_) | CoreError::Query(_) | CoreError::Io(_) => {
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
        let message = self.message();
        let mut res = (
            status,
            Json(Body {
                error: Payload { code, message },
            }),
        )
            .into_response();
        if let ApiError::Unauthorized = self {
            // RFC 7235: a 401 names the scheme it wants.
            res.headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        res
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

    #[test]
    fn a_storage_fault_keeps_the_drivers_text_off_the_wire() {
        use super::{ApiError, CoreError};
        use diesel::result::{DatabaseErrorKind, Error};
        let raw = Error::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            Box::new("UNIQUE constraint failed: products.barcode".to_owned()),
        );
        let err = ApiError::Core(CoreError::Query(raw));
        let message = err.message();
        assert!(!message.contains("constraint"), "{message}");
        assert!(!message.contains("products"), "{message}");
        assert_eq!(err.parts().1, "storage");
        assert_eq!(
            ApiError::Core(CoreError::validation("name", "is empty")).message(),
            CoreError::validation("name", "is empty").to_string(),
            "a rule's own message still goes through"
        );
    }
}

#[cfg(test)]
mod restart_code_tests {
    use super::{ApiError, StatusCode};

    /// The two answers a closed shop file can get, and they are not the same
    /// answer. Both are 500 and both mean relaunch, but one of them also says
    /// the restore did not happen, and that is what decides which data the
    /// owner will be looking at afterwards. A screen can only tell them apart
    /// by the code.
    #[test]
    fn a_closed_shop_file_says_relaunch_and_says_whether_it_was_restored() {
        assert_eq!(
            ApiError::RestartNeeded.parts(),
            (StatusCode::INTERNAL_SERVER_ERROR, "restart_needed")
        );
        assert_eq!(
            ApiError::NotRestoredRestartNeeded.parts(),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "restore_failed_restart_needed"
            )
        );

        let says = ApiError::NotRestoredRestartNeeded.message();
        assert!(says.contains("did not happen"), "{says}");
        assert!(says.contains("start it again"), "{says}");
    }
}
