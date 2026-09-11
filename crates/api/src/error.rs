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

/// The envelope's payload. The figures beside the code are the one exception
/// to "a code and a sentence": the till has to say by how much a credit limit
/// was passed and the fiche by how much a payment overshot, and re-deriving
/// either on the screen would be a second answer to what a customer owes
/// (architecture.md rule 2). They are left out of every other error's body
/// rather than sent as nulls, so nothing else on the wire changed shape.
#[derive(Serialize)]
struct Payload {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    balance_after_centimes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    credit_limit_centimes: Option<i64>,
    /// Which field of the request the refusal is about. Every validation
    /// error names one, so the screen can put the message under the input
    /// rather than reading the name out of the sentence.
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    /// What the customer still owes, on a payment that asked for more.
    #[serde(skip_serializing_if = "Option::is_none")]
    outstanding_centimes: Option<i64>,
    /// The same exception, for the same reason, on the facture's party
    /// blocks: the till has to say which side is short and of what, and
    /// working that out on the screen would be a second reading of décret
    /// 05-468 art. 3. Absent from every other error.
    #[serde(skip_serializing_if = "Option::is_none")]
    party_side: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    missing_ids: Option<Vec<&'static str>>,
    /// How long a locked-out user has to wait, in seconds. The same
    /// exception, for the same reason: the sign-in screen counts it down and
    /// the wait is a figure the caller never sent. Absent from every other
    /// error.
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after_seconds: Option<i64>,
}

/// What an error carries besides its code and its sentence. One value per
/// optional field of the payload, filled by the one error that knows it and
/// left empty by every other, so the body of an ordinary refusal is the two
/// keys it always was.
struct Figures {
    balance_after_centimes: Option<i64>,
    credit_limit_centimes: Option<i64>,
    field: Option<String>,
    outstanding_centimes: Option<i64>,
    party_side: Option<&'static str>,
    missing_ids: Option<Vec<&'static str>>,
    retry_after_seconds: Option<i64>,
}

impl Figures {
    const NONE: Self = Self {
        balance_after_centimes: None,
        credit_limit_centimes: None,
        field: None,
        outstanding_centimes: None,
        party_side: None,
        missing_ids: None,
        retry_after_seconds: None,
    };
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

    /// What an error carries besides its code and its sentence. A credit
    /// refusal names what the sale would have taken the customer to and the
    /// limit it passed; a payment above the debt names what is actually
    /// owed, and the field it is about, so the form can say "you can take at
    /// most this much" without asking the balance again; a facture the party
    /// blocks refuse names the side that is short and the identifiers it is
    /// short of, because working that out on the screen would be a second
    /// reading of décret 05-468 art. 3. Every other error carries none of
    /// them, and the fields are then absent from the body.
    fn figures(&self) -> Figures {
        match self {
            ApiError::Core(CoreError::CreditLimit {
                balance_after,
                credit_limit,
            })
            | ApiError::Request(CoreError::CreditLimit {
                balance_after,
                credit_limit,
            }) => Figures {
                balance_after_centimes: Some(balance_after.as_centimes()),
                credit_limit_centimes: Some(credit_limit.as_centimes()),
                ..Figures::NONE
            },
            ApiError::Core(CoreError::PaymentAboveDebt {
                outstanding_centimes,
            })
            | ApiError::Request(CoreError::PaymentAboveDebt {
                outstanding_centimes,
            }) => Figures {
                field: Some("amount_centimes".to_owned()),
                outstanding_centimes: Some(*outstanding_centimes),
                ..Figures::NONE
            },
            // Every validation error already names the field it is about;
            // only the payment one used to say so on the wire, and a screen
            // that wanted to put the message under the input had to read it
            // out of the sentence. The name travels beside the code now, for
            // all of them.
            ApiError::Core(CoreError::Validation { field, .. })
            | ApiError::Request(CoreError::Validation { field, .. })
            // A conflict names its field too: the screen puts the message
            // under the input the way it does for a validation, and only the
            // code and the status say the two apart.
            | ApiError::Core(CoreError::Conflict { field, .. })
            | ApiError::Request(CoreError::Conflict { field, .. }) => Figures {
                field: Some(field.clone()),
                ..Figures::NONE
            },
            ApiError::Core(CoreError::LockedOut {
                retry_after_seconds,
            })
            | ApiError::Request(CoreError::LockedOut {
                retry_after_seconds,
            }) => Figures {
                retry_after_seconds: Some(*retry_after_seconds),
                ..Figures::NONE
            },
            ApiError::Core(CoreError::PartyIds { side, missing })
            | ApiError::Request(CoreError::PartyIds { side, missing }) => Figures {
                party_side: Some(side.as_str()),
                missing_ids: Some(missing.clone()),
                ..Figures::NONE
            },
            _ => Figures::NONE,
        }
    }
}

/// What a core error means once a service has run. A `Money` error here is a
/// stored row the migration's CHECK makes impossible, so the file is wrong
/// and the caller has nothing to correct: 500, not 422.
const fn status_for(e: &CoreError) -> StatusCode {
    match e {
        // A credit refusal is the request itself the server will not carry
        // out: the basket is well formed and the caller can act on it, by
        // paying another way or by resending with `override`. The two
        // amounts in the payload are what the till renders, so it sits with
        // the 422s and not with the conflicts.
        // A payment above the debt sits with them for the same reason: the
        // caller can act on it, by taking what is owed instead. So does a
        // facture the party blocks refuse: the request is well formed and
        // the caller can act on it, by filling the fiche or the settings in,
        // or by ringing the same basket up as a ticket.
        CoreError::Validation { .. }
        | CoreError::CreditLimit { .. }
        | CoreError::PaymentAboveDebt { .. }
        | CoreError::PartyIds { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        CoreError::NotFound { .. } => StatusCode::NOT_FOUND,
        // A credential that did not match. 401 and not 422: nothing in the
        // request is malformed, and what is missing is an identity the caller
        // has to establish before the route will answer at all.
        CoreError::AuthRefused => StatusCode::UNAUTHORIZED,
        // Too many wrong credentials. 429 is what a screen counting a wait
        // down reads, and the wait itself is in the payload beside the code:
        // the caller can act on it, by waiting, and working it out on the
        // screen would be a second reading of a rule that lives in the core.
        CoreError::LockedOut { .. } => StatusCode::TOO_MANY_REQUESTS,
        CoreError::DuplicateBarcode(_)
        | CoreError::Exhausted { .. }
        | CoreError::Conflict { .. } => StatusCode::CONFLICT,
        // A template that will not render is the app's own bug: the
        // template ships in the binary and the data comes from a row the
        // core just read, so the caller has nothing to correct.
        // A row handed over without a moment on it is the same kind of
        // thing: the caller sent nothing wrong and cannot correct it, so it
        // is this crate's bug and never the shop's.
        CoreError::Money(_)
        | CoreError::Db(_)
        | CoreError::Query(_)
        | CoreError::Io(_)
        | CoreError::Unstamped { .. }
        | CoreError::UnpricedReversal { .. }
        // A credential this app could not hash, with parameters and input
        // shapes it chose itself: its own bug, like the two above.
        | CoreError::Hash(_)
        // A workbook that will not write is the same: the columns and the
        // rows are both the app's own.
        | CoreError::Render(_)
        | CoreError::Workbook(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
        let Figures {
            balance_after_centimes,
            credit_limit_centimes,
            field,
            outstanding_centimes,
            party_side,
            missing_ids,
            retry_after_seconds,
        } = self.figures();
        let mut res = (
            status,
            Json(Body {
                error: Payload {
                    code,
                    message,
                    balance_after_centimes,
                    credit_limit_centimes,
                    field,
                    outstanding_centimes,
                    party_side,
                    missing_ids,
                    retry_after_seconds,
                },
            }),
        )
            .into_response();
        // RFC 7235: a 401 names the scheme it wants. Both of this API's 401s
        // do, and the two are told apart by the code in the body: `unauthorized`
        // is the launch token the process was started with, `auth_refused` is
        // the person standing at the till.
        if matches!(
            self,
            ApiError::Unauthorized | ApiError::Core(CoreError::AuthRefused)
        ) {
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
