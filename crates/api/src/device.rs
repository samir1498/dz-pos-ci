//! Which phone is asking, for callers off this machine.
//!
//! Three gates, not two (docs/architecture.md § Transport and auth will say
//! so at the T7 sweep): the launch token says the caller may reach the
//! server at all, this one says which paired phone a LAN caller is, and the
//! session says which person is acting. The desktop on loopback shows no
//! device token — a process on this machine is already inside the launch
//! token's trust — so loopback callers pass bare and only off-loopback
//! callers must show `X-Dzpos-Device`.
//!
//! ## Failures
//!
//! No device header where one is due is `session_required`, the same 401 a
//! caller with no credential at all gets: nothing presented, nothing to
//! check. A token nobody issued and a revoked one are both `device_refused`,
//! one code for the two so a scanner learns nothing either way, and a code
//! of its own rather than the core's `auth_refused` so a phone can tell a
//! dead pairing (re-pair) from a wrong PIN on `/auth/login` (type it
//! again). A live token slides `last_seen_at` forward, on UTC like the
//! session's own idle clock.
//!
//! `ConnectInfo` is only present when the server installs it (both serve
//! sites do); its absence means a test harness driving the router
//! directly, which counts as loopback and passes. The layer sits outside
//! `session::require`: which phone before which person, and a revoked phone
//! fails before any session is even looked up.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, FromRequestParts, Request, State};
use axum::http::header;
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;
use dzpos_core::error::CoreError;
use dzpos_core::services::pairing;

use crate::error::ApiError;
use crate::session;
use crate::AppState;

/// The header a paired phone shows its device token in. Its own header and
/// not `Authorization`, for the reason `session.rs` gives: one header
/// cannot carry two credentials, and the launch token already rides that
/// one on every route.
pub const DEVICE_HEADER: &str = "x-dzpos-device";

/// The phone this request arrived from, resolved by the middleware below.
/// Handlers that need it take one of these. Audit rows do not name it yet:
/// they are written in services without request context, so plumbing the
/// device through is its own change, tracked open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairedPhone {
    pub id: i32,
}

impl<S: Send + Sync> FromRequestParts<S> for PairedPhone {
    type Rejection = ApiError;

    /// Read out of the request's extensions, where `require` put it. Written
    /// as a refusal rather than a panic for the reason `CurrentUser`'s
    /// extractor gives: a route added outside the guard should answer, not
    /// fall over.
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<PairedPhone>()
            .copied()
            .ok_or(ApiError::SessionRequired)
    }
}

pub async fn require(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let off_loopback = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .is_some_and(|peer| !peer.0.ip().is_loopback());
    if !off_loopback {
        return Ok(next.run(req).await);
    }
    let Some(shown) = device_token(req.headers()) else {
        return Err(ApiError::SessionRequired);
    };
    let shop = state.shop_id;
    let at = session::now();
    let digest = pairing::token_digest(&shown);
    let row = state
        .blocking(move |c| pairing::device_for_request(c, shop, &digest, at))
        .await
        .map_err(|e| match e {
            ApiError::Core(CoreError::AuthRefused) => ApiError::DeviceRefused,
            other => other,
        })?;
    req.extensions_mut().insert(PairedPhone { id: row.id });
    Ok(next.run(req).await)
}

/// The token the phone showed, if it showed one. Owned, because it crosses
/// onto a blocking thread for the digest and the lookup.
fn device_token(headers: &header::HeaderMap) -> Option<String> {
    let value = headers.get(DEVICE_HEADER)?.to_str().ok()?.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use axum::body::Body;

    fn with(headers: &[(&str, &str)]) -> Request {
        let mut req = Request::builder().uri("/");
        for (name, value) in headers {
            req = req.header(*name, *value);
        }
        match req.body(Body::empty()) {
            Ok(req) => req,
            Err(e) => panic!("the test built a request that will not build: {e}"),
        }
    }

    #[test]
    fn the_phones_header_is_read() {
        assert_eq!(
            device_token(with(&[(DEVICE_HEADER, "abc123")]).headers()).as_deref(),
            Some("abc123")
        );
        assert_eq!(
            device_token(with(&[(DEVICE_HEADER, "   ")]).headers()),
            None
        );
        assert_eq!(device_token(with(&[]).headers()), None);
        assert_eq!(
            device_token(with(&[(DEVICE_HEADER, "  abc123  ")]).headers()).as_deref(),
            Some("abc123")
        );
    }
}
