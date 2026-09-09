//! The product's only entry point. The desktop webview, the browser
//! preview and, later, the phone are equal callers (architecture.md rule 2),
//! so these routes and their tests are the product's contract.

pub mod daily;
pub mod dto;
pub mod error;
pub mod routes;
pub mod token;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::http::{header, HeaderValue, Method};
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post, put};
use axum::Router;
use chrono::NaiveDateTime;
use dzpos_core::db::Conn;
use dzpos_core::error::CoreError;
use dzpos_core::services::backup::{self, Backup, Summary};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::error::ApiError;
pub use crate::token::LaunchToken;

/// Where backups land when the caller names no folder: beside the shop file.
/// One folder per shop file, so two tills on one machine never share copies.
pub const BACKUP_DIR_NAME: &str = "backups";

/// The connection and the one shop this server answers for. A caller never
/// chooses the shop (rule 3); the process is started with it.
///
/// The file's own path is held too, because a restore replaces it: the
/// connection alone cannot say which file it is open on once it is closed.
#[derive(Clone)]
pub struct AppState {
    /// `None` once the shop file has been closed and could not be reopened.
    /// An empty slot refuses every caller, which is the point: a stand-in
    /// connection would be a working, empty database, and the next backup
    /// would copy that over a real copy and prune a good one to make room.
    /// Nothing fills the slot again; the app has to be restarted.
    conn: Arc<Mutex<Option<Conn>>>,
    db_path: Arc<PathBuf>,
    backup_dir: Arc<PathBuf>,
    pub shop_id: i32,
}

impl AppState {
    pub fn open(db: impl AsRef<Path>, shop_id: i32) -> Result<Self, CoreError> {
        let dir = default_backup_dir(db.as_ref());
        AppState::open_with_backup_dir(db, shop_id, dir)
    }

    pub fn open_with_backup_dir(
        db: impl AsRef<Path>,
        shop_id: i32,
        backup_dir: impl AsRef<Path>,
    ) -> Result<Self, CoreError> {
        let conn = dzpos_core::db::open(db.as_ref())?;
        Ok(AppState {
            conn: Arc::new(Mutex::new(Some(conn))),
            db_path: Arc::new(db.as_ref().to_path_buf()),
            backup_dir: Arc::new(backup_dir.as_ref().to_path_buf()),
            shop_id,
        })
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    /// A copy of the shop file, taken at `at`, and the folder pruned back to
    /// thirty.
    pub fn create_backup(&self, at: NaiveDateTime) -> Result<Backup, ApiError> {
        let dir = Arc::clone(&self.backup_dir);
        self.with_conn(|conn| backup::create(conn, &dir, at))
    }

    /// Replaces the shop file with one of its copies, then reopens the
    /// connection on it, so the running server answers from the restored
    /// file without the app being restarted.
    ///
    /// The order is the whole guarantee, and it is:
    ///
    /// 1. check the copy (`backup::verify`); a copy that is torn or that a
    ///    newer app version wrote never gets as far as touching the file;
    /// 2. take the connection's lock, so no handler is mid-query;
    /// 3. `VACUUM INTO <db>.before-restore-<stamp>.sqlite`, the safety copy,
    ///    beside the shop file and never counted among the thirty;
    /// 4. copy the chosen backup to `<db>.restoring.tmp`, in the same folder
    ///    so the rename below cannot cross a file system;
    /// 5. close the live connection, which is what releases the file handle,
    ///    leaving the slot under the lock empty;
    /// 6. delete the live file's `-wal` and `-shm` sidecars;
    /// 7. rename `<db>.restoring.tmp` over the shop file;
    /// 8. reopen through `db::open`, which runs any migration the copy is
    ///    behind on, and put that connection back into the slot.
    ///
    /// A failure before step 5 leaves the shop file exactly as it was. A
    /// failure at 6 leaves it as it was too, and step 8 puts the connection
    /// back. If step 8 itself fails, the file on disk is whole but this
    /// process has no connection to it: the slot stays empty and every route
    /// that needs the shop file answers 500 until the app is restarted. That
    /// is deliberate. The path is logged so the operator knows which file.
    pub fn restore(&self, backup_path: &Path) -> Result<Summary, ApiError> {
        let summary = backup::verify(backup_path).map_err(ApiError::Request)?;
        let mut guard = self.conn.lock().map_err(|_| ApiError::Unavailable)?;
        let db = self.db_path.as_path();

        // The shop's clock, the same one the copies are named for, plus the
        // millisecond so two restores in one second keep two safety copies.
        let stamp = crate::routes::backups::now().format("%Y%m%d-%H%M%S%3f");
        let safety = sibling(db, &format!(".before-restore-{stamp}.sqlite"));
        let live = guard.as_mut().ok_or(ApiError::Unavailable)?;
        backup::copy_to(live, &safety).map_err(ApiError::from)?;

        let staged = sibling(db, ".restoring.tmp");
        // A leftover from a run that died mid-restore would make the copy
        // below fail; it describes nothing that is still wanted.
        remove_if_present(&staged).map_err(core_io)?;
        std::fs::copy(backup_path, &staged).map_err(core_io)?;

        // From here the shop file is being replaced. Closing the connection
        // is what releases the handle, and the slot stays empty until a
        // reopen fills it.
        drop(guard.take());

        if let Err(e) = swap_in(&staged, db) {
            let _ = std::fs::remove_file(&staged);
            // The shop file was never renamed over, so it is the one that
            // was always there; reopening it is the whole recovery.
            *guard = Some(dzpos_core::db::open(db).map_err(CoreError::from)?);
            return Err(ApiError::from(e));
        }

        match dzpos_core::db::open(db) {
            Ok(conn) => {
                *guard = Some(conn);
                Ok(summary)
            }
            Err(e) => {
                // The file on disk is whole (the restored copy, with the
                // safety copy beside it); what failed is reopening it. The
                // slot stays empty on purpose, so nothing runs against a
                // stand-in that would look like an empty shop.
                eprintln!(
                    "dz-pos: the shop file at {} could not be reopened after the restore: {e}",
                    db.display()
                );
                Err(ApiError::from(CoreError::from(e)))
            }
        }
    }

    /// Runs one closure against the connection. A poisoned lock means an
    /// earlier handler panicked; the answer is a 500, never another panic.
    pub fn with_conn<T>(
        &self,
        f: impl FnOnce(&mut Conn) -> Result<T, CoreError>,
    ) -> Result<T, ApiError> {
        let mut guard = self.conn.lock().map_err(|_| ApiError::Unavailable)?;
        // Empty means a restore closed the file and could not reopen it.
        // Every caller is refused from here on, which is what keeps a backup
        // or a query from running against nothing.
        let conn = guard.as_mut().ok_or(ApiError::Unavailable)?;
        f(conn).map_err(ApiError::from)
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

/// `<db folder>/backups`. A shop file with no parent (a bare `t.db` on a
/// relative path) keeps the folder in the working directory.
pub fn default_backup_dir(db: &Path) -> PathBuf {
    match db.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(BACKUP_DIR_NAME),
        _ => PathBuf::from(BACKUP_DIR_NAME),
    }
}

/// `<db>` with `suffix` appended to the whole file name, not to its
/// extension: `shop.db` becomes `shop.db.restoring.tmp`, which no backup
/// name pattern matches and no `db::open` will ever be handed.
fn sibling(db: &Path, suffix: &str) -> PathBuf {
    let mut name = db.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

fn remove_if_present(path: &Path) -> Result<(), std::io::Error> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// The sidecars belong to the file that is going, and SQLite would replay a
/// leftover `-wal` into whatever file it finds under that name. They go
/// before the rename, and a rename inside one folder is atomic.
fn swap_in(staged: &Path, db: &Path) -> Result<(), dzpos_core::error::CoreError> {
    for sidecar in ["-wal", "-shm", "-journal"] {
        remove_if_present(&sibling(db, sidecar))?;
    }
    std::fs::rename(staged, db)?;
    Ok(())
}

fn core_io(e: std::io::Error) -> ApiError {
    ApiError::from(CoreError::from(e))
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
        .route("/backups", get(routes::backups::list))
        .route("/backups", post(routes::backups::create))
        .route("/backups/{name}/restore", post(routes::backups::restore))
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
