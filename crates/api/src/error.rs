//! One place maps the core's errors to a status and a body. Every failure
//! leaves as `{ "error": { "code", "message" } }`; the UI translates the
//! code and never shows the message.

use axum::extract::rejection::JsonRejection;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use dzpos_core::error::CoreError;
#[cfg(feature = "retail")]
use dzpos_core::error::RetailError;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// A core error a service raised. The request was well formed and a rule
    /// or the file decided.
    #[error(transparent)]
    Core(#[from] CoreError),
    /// A retail error a shop service raised: one of the eight variants S4 of
    /// `a-kernel-crate-and-retail-as-the-first-module` moved out of
    /// `CoreError` (`DuplicateBarcode`, `PaymentAboveDebt`, `CreditLimit`,
    /// `PartyIds`, `Unstamped`, `UnpricedReversal`, `Render`, `Workbook`), or
    /// any `CoreError` the shop service raised unchanged, arriving wrapped in
    /// `RetailError::Kernel`. Mapped beside `Core` so a client sees the same
    /// status, code and body it always has for every one of these failures;
    /// this crate is the only one that has to know two enums exist.
    #[cfg(feature = "retail")]
    #[error(transparent)]
    Retail(#[from] RetailError),
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
    /// No session, or one that is not standing any more. Its own code beside
    /// `unauthorized` and `auth_refused`, because the three send a screen
    /// three different ways: the launch token is the app being started wrong
    /// and nothing a person can fix, a refused credential is the PIN just
    /// typed, and this is "sign in again" on a screen that thought it already
    /// had. Which of the four ways the session died is deliberately not said
    /// (`services::sessions` says why).
    #[error("this call carried no session, or one that is no longer standing")]
    SessionRequired,
    /// The device token a LAN caller showed names no paired phone of this
    /// shop, or one that was revoked. Its own code and not the core's
    /// `auth_refused`: a phone reads the two differently (a dead pairing
    /// is "ask a manager for a new QR", a wrong PIN is "type it again"),
    /// and on 2026-09-16 a mistyped PIN on the phone wiped its pairing
    /// because both came back as the same code. Which of the two the token
    /// is, unissued or revoked, is still not said.
    #[error("this phone is not paired with this shop, or its pairing was revoked")]
    DeviceRefused,
    /// A write reached this API on a route the permission table does not
    /// name. Nobody can say who is allowed to do it, so nobody is: the gate
    /// fails closed rather than waving a write through because a row was
    /// forgotten. `crates/api/tests/route_gates.rs` is what stops this ever
    /// reaching a shop; this is what happens if it ever does.
    #[error("this write is on a route no permission has been decided for")]
    UngatedWrite,
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
    /// Only on `forbidden`: the name of the permission the route wanted, the
    /// same spelling `Permission::as_str` writes and `PermissionDto`
    /// serialises. The same exception for the same reason: a screen that had
    /// to work out which permission a 403 was about would be restating the
    /// table in `services::permissions`, and the refusal already knows.
    #[serde(skip_serializing_if = "Option::is_none")]
    permission: Option<&'static str>,
}

/// What an error carries besides its code and its sentence. One value per
/// optional field of the payload, filled by the one error that knows it and
/// left empty by every other, so the body of an ordinary refusal is the two
/// keys it always was.
#[cfg_attr(test, derive(Debug, PartialEq))]
struct Figures {
    balance_after_centimes: Option<i64>,
    credit_limit_centimes: Option<i64>,
    field: Option<String>,
    outstanding_centimes: Option<i64>,
    party_side: Option<&'static str>,
    missing_ids: Option<Vec<&'static str>>,
    retry_after_seconds: Option<i64>,
    permission: Option<&'static str>,
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
        permission: None,
    };
}

impl ApiError {
    /// What the wire carries as `message`. A storage fault keeps its SQL
    /// text on the server: the driver names tables and columns, and
    /// docs/architecture.md promises no Rust or SQL text ever reaches a
    /// screen.
    fn message(&self) -> String {
        match self {
            #[cfg(feature = "retail")]
            ApiError::Core(CoreError::Query(_) | CoreError::Db(_))
            | ApiError::Request(CoreError::Query(_) | CoreError::Db(_))
            | ApiError::Retail(RetailError::Kernel(CoreError::Query(_) | CoreError::Db(_))) => {
                "the shop file could not complete the operation".to_owned()
            }
            #[cfg(not(feature = "retail"))]
            ApiError::Core(CoreError::Query(_) | CoreError::Db(_))
            | ApiError::Request(CoreError::Query(_) | CoreError::Db(_)) => {
                "the shop file could not complete the operation".to_owned()
            }
            other => other.to_string(),
        }
    }

    // Two whole functions rather than one cfg'd match arm (S5 of
    // `a-kernel-crate-and-retail-as-the-first-module`): `apps/mobile`'s
    // `lib/errors.test.ts` reads this function's own body as text between
    // its braces and pulls every quoted lowercase word out of it as a code
    // the server can send. A `#[cfg(feature = "retail")]` inside that span
    // reads as a code too (there is no server error named "retail"), and
    // that test went red proving it. The attribute sits above the function
    // instead, outside the span the walk reads, the same way the two
    // versions of `once` in `daily.rs` are two functions and not one cfg'd
    // return type.
    #[cfg(feature = "retail")]
    fn parts(&self) -> (StatusCode, &'static str) {
        match self {
            // The code is the core's own (architecture.md: map, never
            // re-derive). Only the status is the API's to choose.
            ApiError::Core(e) => (status_for(e), e.code()),
            // Same rule, for the enum that carries the shop's own variants:
            // the code is `RetailError::code`'s own, and `Kernel` delegates
            // to `CoreError::code` beneath it, so a code on the wire never
            // depends on which of the two enums the service happened to
            // return it through.
            ApiError::Retail(e) => (status_for_retail(e), e.code()),
            ApiError::Request(e) => (StatusCode::UNPROCESSABLE_ENTITY, e.code()),
            ApiError::BadRequest(_) => (StatusCode::UNPROCESSABLE_ENTITY, "bad_request"),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::SessionRequired => (StatusCode::UNAUTHORIZED, "session_required"),
            ApiError::DeviceRefused => (StatusCode::UNAUTHORIZED, "device_refused"),
            ApiError::UngatedWrite => (StatusCode::INTERNAL_SERVER_ERROR, "ungated_write"),
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

    /// Retail-only build (S5): the same match, minus the one variant this
    /// crate does not have without the feature.
    #[cfg(not(feature = "retail"))]
    fn parts(&self) -> (StatusCode, &'static str) {
        match self {
            ApiError::Core(e) => (status_for(e), e.code()),
            ApiError::Request(e) => (StatusCode::UNPROCESSABLE_ENTITY, e.code()),
            ApiError::BadRequest(_) => (StatusCode::UNPROCESSABLE_ENTITY, "bad_request"),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::SessionRequired => (StatusCode::UNAUTHORIZED, "session_required"),
            ApiError::DeviceRefused => (StatusCode::UNAUTHORIZED, "device_refused"),
            ApiError::UngatedWrite => (StatusCode::INTERNAL_SERVER_ERROR, "ungated_write"),
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
            ApiError::Core(e) | ApiError::Request(e) => figures_of_core(e),
            #[cfg(feature = "retail")]
            ApiError::Retail(e) => figures_of_retail(e),
            _ => Figures::NONE,
        }
    }
}

/// The figures a plain `CoreError` carries, shared by `ApiError::Core`,
/// `ApiError::Request` (both wrap `CoreError` directly) and
/// `ApiError::Retail`'s own `Kernel` arm (a `CoreError` a shop service raised
/// unchanged), so which of the two enums a service happened to return it
/// through never changes what reaches the wire.
///
/// Every validation error already names the field it is about; only the
/// payment one used to say so on the wire, and a screen that wanted to put
/// the message under the input had to read it out of the sentence. The name
/// travels beside the code now, for all of them. A conflict names its field
/// too, for the same reason.
fn figures_of_core(e: &CoreError) -> Figures {
    match e {
        CoreError::Validation { field, .. } | CoreError::Conflict { field, .. } => Figures {
            field: Some(field.clone()),
            ..Figures::NONE
        },
        CoreError::LockedOut {
            retry_after_seconds,
        } => Figures {
            retry_after_seconds: Some(*retry_after_seconds),
            ..Figures::NONE
        },
        // The permission the route wanted, travelling back out of the very
        // check that asked for it (M4 T1's `require`), so neither a route
        // nor a screen restates which one it was.
        CoreError::Forbidden { permission } => Figures {
            permission: Some(permission.as_str()),
            ..Figures::NONE
        },
        _ => Figures::NONE,
    }
}

/// The figures a `RetailError` carries. A credit refusal names what the sale
/// would have taken the customer to and the limit it passed; a payment above
/// the debt names what is actually owed, and the field it is about, so the
/// form can say "you can take at most this much" without asking the balance
/// again; a facture the party blocks refuse names the side that is short and
/// the identifiers it is short of, because working that out on the screen
/// would be a second reading of décret 05-468 art. 3. `Kernel` delegates to
/// `figures_of_core`, unchanged; every other variant carries none of these
/// and the fields are absent from the body.
#[cfg(feature = "retail")]
fn figures_of_retail(e: &RetailError) -> Figures {
    match e {
        RetailError::Kernel(inner) => figures_of_core(inner),
        RetailError::CreditLimit {
            balance_after,
            credit_limit,
        } => Figures {
            balance_after_centimes: Some(balance_after.as_centimes()),
            credit_limit_centimes: Some(credit_limit.as_centimes()),
            ..Figures::NONE
        },
        RetailError::PaymentAboveDebt {
            outstanding_centimes,
        } => Figures {
            field: Some("amount_centimes".to_owned()),
            outstanding_centimes: Some(*outstanding_centimes),
            ..Figures::NONE
        },
        RetailError::PartyIds { side, missing } => Figures {
            party_side: Some(side.as_str()),
            missing_ids: Some(missing.clone()),
            ..Figures::NONE
        },
        _ => Figures::NONE,
    }
}

/// What a core error means once a service has run. A `Money` error here is a
/// stored row the migration's CHECK makes impossible, so the file is wrong
/// and the caller has nothing to correct: 500, not 422.
const fn status_for(e: &CoreError) -> StatusCode {
    match e {
        CoreError::Validation { .. } => StatusCode::UNPROCESSABLE_ENTITY,
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
        // A role's own refusal (M4 T1, `services::permissions::can`). The
        // request is well formed; a different user is what would carry it
        // out. T2 is what actually asks a session for a role; this arm only
        // keeps `status_for` exhaustive now that `CoreError` has the variant.
        CoreError::Forbidden { .. } => StatusCode::FORBIDDEN,
        CoreError::Exhausted { .. } | CoreError::Conflict { .. } => StatusCode::CONFLICT,
        // A row handed over without a moment on it is the same kind of
        // thing: the caller sent nothing wrong and cannot correct it, so it
        // is this crate's bug and never the shop's.
        CoreError::Money(_) | CoreError::Db(_) | CoreError::Query(_) | CoreError::Io(_)
        // A credential this app could not hash, with parameters and input
        // shapes it chose itself: its own bug, like the one above.
        | CoreError::Hash(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// What a retail error means once a shop service has run. `Kernel` delegates
/// to `status_for`, unchanged, so a `CoreError` a shop service raised
/// unmodified answers with the same status it always has. The other eight
/// arms are exactly the statuses `status_for` gave their variants before S4
/// of `a-kernel-crate-and-retail-as-the-first-module` moved them out of
/// `CoreError`: nothing a client can see changed, only which enum carries it
/// on the way here.
#[cfg(feature = "retail")]
const fn status_for_retail(e: &RetailError) -> StatusCode {
    match e {
        RetailError::Kernel(inner) => status_for(inner),
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
        RetailError::CreditLimit { .. }
        | RetailError::PaymentAboveDebt { .. }
        | RetailError::PartyIds { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        RetailError::DuplicateBarcode(_) => StatusCode::CONFLICT,
        // A template that will not render is the app's own bug: the
        // template ships in the binary and the data comes from a row the
        // core just read, so the caller has nothing to correct.
        // A row handed over without a moment on it is the same kind of
        // thing: the caller sent nothing wrong and cannot correct it, so it
        // is this crate's bug and never the shop's.
        // A workbook that will not write is the same: the columns and the
        // rows are both the app's own.
        RetailError::Unstamped { .. }
        | RetailError::UnpricedReversal { .. }
        | RetailError::Render(_)
        | RetailError::Workbook(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
            permission,
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
                    permission,
                },
            }),
        )
            .into_response();
        // RFC 7235: a 401 names the scheme it wants. All four of this API's
        // 401s do, and they are told apart by the code in the body:
        // `unauthorized` is the launch token the process was started with,
        // `auth_refused` is the person standing at the till getting their PIN
        // wrong, `session_required` is a screen whose session has stopped
        // standing, and `device_refused` is a phone whose pairing is gone. A
        // screen acts differently on each and cannot read the status alone.
        if matches!(
            self,
            ApiError::Unauthorized
                | ApiError::SessionRequired
                | ApiError::DeviceRefused
                | ApiError::Core(CoreError::AuthRefused)
        ) || retail_auth_refused(&self)
        {
            res.headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        res
    }
}

/// The fifth of the five ways this API answers 401, split out of the
/// `matches!` above because `matches!`'s pattern list is not somewhere a
/// `#[cfg]` can sit (S5 of `a-kernel-crate-and-retail-as-the-first-module`):
/// a shop's own `AuthRefused`, carried in `RetailError::Kernel`. Always
/// `false` without the feature, since the variant it names does not exist.
#[cfg(feature = "retail")]
fn retail_auth_refused(e: &ApiError) -> bool {
    matches!(
        e,
        ApiError::Retail(RetailError::Kernel(CoreError::AuthRefused))
    )
}

#[cfg(not(feature = "retail"))]
fn retail_auth_refused(_e: &ApiError) -> bool {
    false
}

#[cfg(test)]
#[path = "../tests/unit/error_messages.rs"]
mod message_and_restart_code_tests;

#[cfg(test)]
#[path = "../tests/unit/error_mapping.rs"]
mod pre_split_mapping_regression_tests;
