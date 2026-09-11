//! Who is acting on a request, and how the token gets here.
//!
//! Two gates, not one (docs/architecture.md § Transport and auth). The launch
//! token is the outer one and it is unchanged: it says the caller is the
//! process this server was started for, and `/health` is the only route
//! without it. This is the inner one, and it says which person is at the
//! keyboard. Folding them together would mean one secret that both survives a
//! restart and names a user, which is neither of the two things.
//!
//! ## How the token travels
//!
//! Two ways, one session store; the difference is only transport.
//!
//! - A browser gets an httpOnly cookie, set by the login response and sent
//!   back by the browser itself. httpOnly is the point: a script on the page
//!   cannot read it, so a cross-site script cannot walk off with a live
//!   session.
//! - The desktop reads the token out of the login body and sends it in the
//!   `X-Dzpos-Session` header. A header of its own and not `Authorization`,
//!   because `Authorization: Bearer` already carries the launch token on
//!   every guarded route and one header cannot carry two credentials. The
//!   Tauri webview also cannot set a `Cookie` header by hand, so the cookie
//!   path is not open to it.
//!
//! The header wins when both are shown. A desktop that sent a header meant
//! it; a stale cookie left in a webview is the thing that would otherwise
//! quietly decide who a request came from.

use axum::extract::{FromRequestParts, MatchedPath, Request, State};
use axum::http::request::Parts;
use axum::http::{header, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use dzpos_core::models::sql_types::Role;
use dzpos_core::services::permissions;
use dzpos_core::services::sessions::{self, Actor};

use crate::error::ApiError;
use crate::gates;
use crate::AppState;

/// The header the desktop shows its session token in.
pub const SESSION_HEADER: &str = "x-dzpos-session";
/// The cookie a browser is given it in.
pub const SESSION_COOKIE: &str = "dzpos_session";

/// Who is acting, taken off the request by the middleware below. Every route
/// that used to read `AppState::user_id` takes one of these instead, and a
/// route that forgets it will not compile against a service that wants a user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentUser {
    pub id: i32,
    pub role: Role,
}

impl From<Actor> for CurrentUser {
    fn from(a: Actor) -> Self {
        CurrentUser {
            id: a.user_id,
            role: a.role,
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = ApiError;

    /// Read out of the request's extensions, where `require` put it. The
    /// rejection is the same 401 a missing session gets, and it is
    /// unreachable through the router: every route this extractor is used on
    /// sits behind that middleware. It is written as a refusal rather than a
    /// panic because a route added outside the guard should answer, not fall
    /// over.
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CurrentUser>()
            .copied()
            .ok_or(ApiError::SessionRequired)
    }
}

/// The middleware every route but the auth ones and `/health` sits behind:
/// resolve the token into an actor, or answer 401 `session_required`.
///
/// The resolve also slides the idle time forward, so "how long since the till
/// was touched" is measured by the requests it carried and not by a clock on
/// the screen.
///
/// The permission gate (M4 T3) sits here too, once the actor is known and
/// before the handler runs: `gates::ROUTE_GATES` is looked up by the method
/// and `axum::extract::MatchedPath`, the route's own template
/// (`/customers/{id}`, not the id a caller happened to send), so a route
/// never names its permission a second time in its handler. A route this
/// layer does not sit in front of (the auth routes, `/health`) has no row
/// reachable this way and needs none: signing in is how a role comes to
/// exist. `MatchedPath` is only absent when the router answers 404 or 405
/// before a route matched, and a table lookup on no route finds nothing to
/// enforce, which is the same as leaving the refusal to the response that is
/// already on its way.
pub async fn require(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let Some(token) = token_of(&req) else {
        return Err(ApiError::SessionRequired);
    };
    let shop = state.shop_id;
    let at = now();
    let actor = state
        .blocking(move |c| sessions::resolve(c, shop, &token, at))
        .await?
        .ok_or(ApiError::SessionRequired)?;
    let current = CurrentUser::from(actor);

    if let Some(matched) = req.extensions().get::<MatchedPath>() {
        if let Some(gate) = gates::gate_for(req.method().as_str(), matched.as_str()) {
            if let Some(permission) = gate.permission {
                permissions::require(current.role, permission)?;
            }
        }
    }

    req.extensions_mut().insert(current);
    Ok(next.run(req).await)
}

/// UTC, not the shop's calendar. The idle time is a stretch of minutes and
/// `last_seen_at` is written on the same clock it is compared against; the
/// reason `services::users` gives for the lockout columns is this one.
pub(crate) fn now() -> chrono::NaiveDateTime {
    chrono::Utc::now().naive_utc()
}

/// The token this request shows, from the header first and the cookie second.
/// Owned, because it crosses onto a blocking thread.
pub fn token_of(req: &Request) -> Option<String> {
    header_token(req.headers()).or_else(|| cookie_token(req.headers()))
}

fn header_token(headers: &header::HeaderMap) -> Option<String> {
    let value = headers.get(SESSION_HEADER)?.to_str().ok()?.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

/// The one cookie this app sets, out of whatever else the browser is
/// carrying. RFC 6265 separates pairs with "; "; a value this app wrote is
/// hex, so nothing here has to decode one.
fn cookie_token(headers: &header::HeaderMap) -> Option<String> {
    let jar = headers.get(header::COOKIE)?.to_str().ok()?;
    jar.split(';').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name.trim() == SESSION_COOKIE).then(|| value.trim().to_owned())
    })
}

/// The `Set-Cookie` a sign-in writes.
///
/// `HttpOnly` so a script on the page cannot read it. `SameSite=Lax` so
/// another site cannot make the browser send it on a form post. No `Secure`,
/// deliberately: this server is plain HTTP on 127.0.0.1 and always will be
/// (LAN and hosted modes terminate TLS somewhere else, architecture.md
/// § Transport and auth), and a `Secure` cookie on a plain-HTTP origin is one
/// a browser drops, which is a sign-in that silently does not stick.
///
/// `Max-Age` is the shop's idle time, so a browser closed for an hour comes
/// back with no cookie rather than with one the server will refuse. The
/// server's own check is still the one that decides; this only saves a round
/// trip.
///
/// One thing `Lax` costs, and only in development: a page served from
/// `localhost:5173` calling an API on `127.0.0.1:4317` is cross-site to a
/// browser, whatever the two resolve to, so the cookie does not travel and
/// the preview falls back to the header. The shipped path is the Tauri
/// webview, which carries the header anyway, so this is a note about the dev
/// server and not a hole: open the preview on the same host name as the API
/// and the cookie rides along.
pub fn set_cookie(token: &str, idle_minutes: i64) -> Option<HeaderValue> {
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        idle_minutes.saturating_mul(60)
    ))
    .ok()
}

/// The `Set-Cookie` a sign-out writes: the same cookie, emptied and expired,
/// so the browser drops it rather than keeping a token the server has
/// already ended.
pub fn clear_cookie() -> HeaderValue {
    HeaderValue::from_static("dzpos_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
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
            // Every header this file's tests build is a valid one, so this
            // arm is unreachable; it is written out because the workspace
            // denies `unwrap` even here.
            Err(e) => panic!("the test built a request that will not build: {e}"),
        }
    }

    #[test]
    fn the_desktops_header_is_read() {
        assert_eq!(
            token_of(&with(&[(SESSION_HEADER, "abc123")])).as_deref(),
            Some("abc123")
        );
        assert_eq!(token_of(&with(&[(SESSION_HEADER, "   ")])), None);
        assert_eq!(token_of(&with(&[])), None);
    }

    /// The browser's cookie, picked out of whatever else the page is
    /// carrying, in either order and with the spacing a browser actually
    /// sends.
    #[test]
    fn the_browsers_cookie_is_read_out_of_the_jar() {
        for jar in [
            "dzpos_session=abc123",
            "other=1; dzpos_session=abc123",
            "dzpos_session=abc123; other=1",
            "a=1;dzpos_session=abc123;b=2",
        ] {
            assert_eq!(
                token_of(&with(&[("cookie", jar)])).as_deref(),
                Some("abc123"),
                "{jar}"
            );
        }
        // A jar with no cookie of ours, and one whose name only looks like
        // ours.
        for jar in ["other=1", "xdzpos_session=abc123", "dzpos_sessionx=abc"] {
            assert_eq!(token_of(&with(&[("cookie", jar)])), None, "{jar}");
        }
    }

    /// A stale cookie in a webview must not quietly decide who a request came
    /// from when the desktop said who it was.
    #[test]
    fn the_header_wins_over_a_cookie() {
        let req = with(&[
            (SESSION_HEADER, "from-the-desktop"),
            ("cookie", "dzpos_session=stale"),
        ]);
        assert_eq!(token_of(&req).as_deref(), Some("from-the-desktop"));
    }

    /// The three flags the cookie is worth setting for, and the one it must
    /// not carry on a plain-HTTP loopback origin.
    #[test]
    fn the_cookie_is_http_only_same_site_and_not_secure() {
        let set = set_cookie("abc123", 15).expect("a hex token makes a header value");
        let text = set.to_str().unwrap_or_default();
        assert!(text.contains("dzpos_session=abc123"), "{text}");
        assert!(text.contains("HttpOnly"), "{text}");
        assert!(text.contains("SameSite=Lax"), "{text}");
        assert!(text.contains("Path=/"), "{text}");
        assert!(text.contains("Max-Age=900"), "{text}");
        assert!(
            !text.contains("Secure"),
            "a Secure cookie on a plain-HTTP loopback origin is one the browser drops: {text}"
        );

        let cleared = clear_cookie().to_str().unwrap_or_default().to_owned();
        assert!(cleared.contains("Max-Age=0"), "{cleared}");
        assert!(cleared.contains("HttpOnly"), "{cleared}");
    }
}
