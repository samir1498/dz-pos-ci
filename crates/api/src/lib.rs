//! The product's only entry point. The desktop webview, the browser
//! preview and, later, the phone are equal callers (architecture.md rule 2),
//! so these routes and their tests are the product's contract.

pub mod dto;
pub mod error;
pub mod routes;
pub mod token;

use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::http::{header, HeaderValue, Method};
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post, put};
use axum::Router;
use dzpos_core::db::Conn;
use dzpos_core::error::CoreError;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::error::ApiError;
pub use crate::token::LaunchToken;

/// The owner the first migration seeds. Every document, ledger row and audit
/// entry carries a user from the first sale (features.md §5).
/// TODO(M4): the user comes from the request identity, not from here.
pub const SEEDED_OWNER_USER_ID: i32 = 1;

/// The connection and the one shop this server answers for. A caller never
/// chooses the shop (rule 3); the process is started with it.
#[derive(Clone)]
pub struct AppState {
    conn: Arc<Mutex<Conn>>,
    pub shop_id: i32,
    /// Who the writes are recorded under until M4 brings login.
    pub user_id: i32,
}

impl AppState {
    pub fn open(db: impl AsRef<Path>, shop_id: i32) -> Result<Self, CoreError> {
        let conn = dzpos_core::db::open(db)?;
        Ok(AppState {
            conn: Arc::new(Mutex::new(conn)),
            shop_id,
            user_id: SEEDED_OWNER_USER_ID,
        })
    }

    /// Runs one closure against the connection. A poisoned lock means an
    /// earlier handler panicked; the answer is a 500, never another panic.
    pub fn with_conn<T>(
        &self,
        f: impl FnOnce(&mut Conn) -> Result<T, CoreError>,
    ) -> Result<T, ApiError> {
        let mut guard = self.conn.lock().map_err(|_| ApiError::Unavailable)?;
        f(&mut guard).map_err(ApiError::from)
    }

    /// Same, off the async executor. diesel is synchronous, so a query that
    /// waits on the file must not hold a runtime thread.
    pub async fn blocking<T, F>(&self, f: F) -> Result<T, ApiError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Conn) -> Result<T, CoreError> + Send + 'static,
    {
        let me = self.clone();
        tokio::task::spawn_blocking(move || me.with_conn(f))
            .await
            .map_err(|_| ApiError::Unavailable)?
    }
}

/// The origins the product's own screens are served from: the browser
/// preview on 5173 and the two the Tauri webview uses.
const OWN_ORIGINS: [HeaderValue; 4] = [
    HeaderValue::from_static("http://127.0.0.1:5173"),
    HeaderValue::from_static("http://localhost:5173"),
    HeaderValue::from_static("tauri://localhost"),
    HeaderValue::from_static("http://tauri.localhost"),
];

pub fn router(state: AppState, token: &LaunchToken) -> Router {
    router_with_origin(state, token, None)
}

/// Same routes, plus one more origin the operator names. The UI served from
/// the WSL box and opened on the laptop is a real case and it is a decision,
/// never a default.
/// Turns the `--allow-origin` flag into a header value the CORS list will
/// take. Checked before the server binds: tower-http panics on `*` inside
/// `AllowOrigin::list`, and it did so after the listening line was printed,
/// so a harness waiting on that line hung on a dead server. `null` would be
/// echoed back to any sandboxed page. A trailing slash or a path never
/// matches what a browser sends, so it is refused rather than silently
/// never matching.
pub fn origin_from_flag(flag: &str) -> Result<HeaderValue, String> {
    let uri: axum::http::Uri = flag.parse().map_err(|_| format!("{flag}: not a URL"))?;
    let scheme = uri.scheme_str().unwrap_or_default();
    if !["http", "https", "tauri"].contains(&scheme) {
        return Err(format!("{flag}: the scheme must be http, https or tauri"));
    }
    if uri.host().is_none_or(str::is_empty) {
        return Err(format!("{flag}: no host"));
    }
    if uri
        .path_and_query()
        .is_some_and(|p| p.as_str() != "" && p.as_str() != "/")
        || flag.ends_with('/')
    {
        return Err(format!(
            "{flag}: an origin is scheme://host[:port], no path and no trailing slash"
        ));
    }
    HeaderValue::from_str(flag).map_err(|_| format!("{flag}: not a valid header value"))
}

pub fn router_with_origin(
    state: AppState,
    token: &LaunchToken,
    extra: Option<HeaderValue>,
) -> Router {
    let mut origins = OWN_ORIGINS.to_vec();
    origins.extend(extra);

    // Who may call (docs/architecture.md, "Transport and auth"): the process
    // that holds the launch token. Loopback is not a boundary, any process
    // or page on the machine can open 127.0.0.1, so the token is what says
    // "this is the desktop's own screen". Only /health answers without it.
    // TODO(M4): which user is calling. Roles arrive with users; the request
    // identity slot is this middleware, the token stays the outer check.
    // The health route owes the same JSON 405 as every guarded one; the
    // method fallback below is the one on the router inside the guard.
    let open = Router::new().route(
        "/health",
        get(routes::health).fallback(routes::method_not_allowed),
    );
    let guarded = Router::new()
        .route("/categories", get(routes::categories::list))
        .route("/products", get(routes::products::list))
        .route("/products", post(routes::products::create))
        .route(
            "/products/{id}",
            get(routes::products::get_one).put(routes::products::update),
        )
        .route("/settings", get(routes::settings::read))
        .route("/settings/store", put(routes::settings::update_store))
        .route("/settings/regime", post(routes::settings::change_regime))
        .fallback(routes::not_found)
        .method_not_allowed_fallback(routes::method_not_allowed)
        .layer(from_fn_with_state(token.clone(), token::require));

    open.merge(guarded)
        // Every browser on the machine can reach 127.0.0.1, so
        // allow_origin(Any) let any page a user happened to open read and
        // write the till. The list is what keeps a stranger's page from
        // being handed the answer, and the preflight is answered here,
        // before the token check, because a browser sends it bare.
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods([Method::GET, Method::POST, Method::PUT])
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]),
        )
        .with_state(state)
}

/// Binds loopback only, on `port`, and reports the port actually bound so
/// a caller passing 0 can tell the UI where to look. A till's database
/// must never be reachable from the shop's wifi; LAN mode in M6 is a
/// deliberate, separate decision.
pub async fn bind(port: u16) -> std::io::Result<(tokio::net::TcpListener, u16)> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    let bound = listener.local_addr()?.port();
    Ok((listener, bound))
}

#[cfg(test)]
mod origin_tests {
    use super::origin_from_flag;

    #[test]
    fn a_plain_origin_is_accepted() {
        for ok in [
            "http://100.111.55.62:5173",
            "http://localhost:5174",
            "https://till.example",
            "tauri://localhost",
        ] {
            assert!(origin_from_flag(ok).is_ok(), "{ok} was refused");
        }
    }

    #[test]
    fn a_wildcard_null_slash_path_or_space_is_refused_before_binding() {
        for bad in [
            "*",
            "null",
            "http://x:5173/",
            "http://a b",
            "http://x:5173/products",
            "ftp://x",
            "localhost:5173",
        ] {
            assert!(origin_from_flag(bad).is_err(), "{bad} was accepted");
        }
    }
}
