//! The product's only entry point. The desktop webview, the browser
//! preview and, later, the phone are equal callers (architecture.md rule 2),
//! so these routes and their tests are the product's contract.

pub mod dto;
pub mod error;
pub mod routes;

use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::routing::{get, post};
use axum::Router;
use dzpos_core::db::Conn;
use dzpos_core::error::CoreError;
use tower_http::cors::{Any, CorsLayer};

use crate::error::ApiError;

/// The connection and the one shop this server answers for. A caller never
/// chooses the shop (rule 3); the process is started with it.
#[derive(Clone)]
pub struct AppState {
    conn: Arc<Mutex<Conn>>,
    pub shop_id: i32,
}

impl AppState {
    pub fn open(db: impl AsRef<Path>, shop_id: i32) -> Result<Self, CoreError> {
        let conn = dzpos_core::db::open(db)?;
        Ok(AppState {
            conn: Arc::new(Mutex::new(conn)),
            shop_id,
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

pub fn router(state: AppState) -> Router {
    // TODO(M4): no auth. Every caller is trusted because the server binds to
    // 127.0.0.1 and one desktop owns the file. M4 brings users and roles and
    // this router gains the middleware that checks them.
    Router::new()
        .route("/health", get(routes::health))
        .route("/products", get(routes::products::list))
        .route("/products", post(routes::products::create))
        .route("/products/{id}", get(routes::products::get_one))
        .fallback(routes::not_found)
        // The browser preview on 5173 and the Tauri webview are both a
        // different origin from this server. Safe because the socket is
        // loopback-only; revisit with auth in M4.
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state)
}

/// Binds loopback only. A till's database must never be reachable from the
/// shop's wifi; LAN mode in M7 is a deliberate, separate decision.
pub async fn serve(state: AppState, port: u16) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    axum::serve(listener, router(state)).await
}

/// Binds loopback on `port` and reports the port actually bound, so a
/// caller passing 0 can tell the UI where to look.
pub async fn bind(port: u16) -> std::io::Result<(tokio::net::TcpListener, u16)> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    let bound = listener.local_addr()?.port();
    Ok((listener, bound))
}
