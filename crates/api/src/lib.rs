//! The product's only entry point. The desktop webview, the browser
//! preview and, later, the phone are equal callers (architecture.md rule 2),
//! so these routes and their tests are the product's contract.

pub mod daily;
pub mod dto;
pub mod error;
pub mod gates;
pub mod routes;
pub mod session;
pub mod token;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::extract::DefaultBodyLimit;
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
pub use crate::session::CurrentUser;
pub use crate::token::LaunchToken;

/// What a restore leaves behind: what the shop file now holds, and the name
/// of the copy of the old one taken on the way. The owner is told that name
/// because it is the only record of what was replaced, and nothing deletes
/// it.
pub struct Restored {
    pub summary: Summary,
    pub safety_copy: String,
}

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
        let conn = open_and_upgrade(db.as_ref())?;
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

    /// Every copy this shop has: the daily ones in the backup folder, and
    /// the safety copies taken on the way into a restore, which sit beside
    /// the shop file so pruning cannot reach them.
    pub fn list_backups(&self) -> Result<(Vec<Backup>, Vec<Backup>), ApiError> {
        let daily = backup::list(&self.backup_dir).map_err(ApiError::from)?;
        let safety = backup::list_safety(&self.db_path).map_err(ApiError::from)?;
        Ok((daily, safety))
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
    /// 3. `VACUUM INTO <db>.before-restore-<stamp>-<millis>.sqlite`, the
    ///    safety copy, beside the shop file and never counted among the
    ///    thirty, so pruning cannot reach it;
    /// 4. copy the chosen backup to `<db>.restoring.tmp`, in the same folder
    ///    so the rename below cannot cross a file system;
    /// 5. `PRAGMA wal_checkpoint(TRUNCATE)` on the live connection, so its
    ///    `-wal` is folded back into the file and emptied while the
    ///    connection that owns it is still open;
    /// 6. close the live connection, which is what releases the file handle,
    ///    leaving the slot under the lock empty;
    /// 7. rename `<db>.restoring.tmp` over the shop file;
    /// 8. delete the old file's `-wal` and `-shm`, after the rename and only
    ///    if it happened: until then they are what the shop file still needs
    ///    to be whole;
    /// 9. reopen through `db::open`, which runs any migration the copy is
    ///    behind on, and put that connection back into the slot.
    ///
    /// The ways it can fail, and what each answers:
    ///
    /// - anything up to step 6 leaves the shop file exactly as it was and
    ///   the connection where it was, and takes the staged copy and the
    ///   safety copy with it. The staged one is a whole database under a name
    ///   nothing reads; the safety one holds the same data as the shop file
    ///   still standing beside it, and it is listed on the backups screen and
    ///   counted by no prune, so a restore refused once a day would fill the
    ///   folder with copies of a file that never moved;
    /// - the rename at 7 failing leaves the shop file as it was, sidecars
    ///   included, and it is reopened: the restore did not happen, the till
    ///   goes on working and the safety copy goes with the staged one for the
    ///   same reason. If that reopen fails too, both halves are logged, the
    ///   safety copy is kept and the answer is `NotRestoredRestartNeeded`:
    ///   the file on disk is the one that was always there, but nothing in
    ///   this process could open it, so the copy that was taken while it
    ///   could is the one database known to open and it stays. The app has to
    ///   be relaunched to come back on the data it had before;
    /// - a sidecar at 8, or the reopen at 9, failing leaves the slot empty
    ///   on purpose and answers `RestartNeeded`. The copy is in place, and
    ///   SQLite replays whatever `-wal` it finds beside a file it opens, so
    ///   nothing here opens it again. The file names are logged, and by the
    ///   time the app is relaunched step 5 has already made anything left
    ///   over harmless.
    pub fn restore(&self, backup_path: &Path, actor_id: i32) -> Result<Restored, ApiError> {
        let summary = backup::verify(backup_path).map_err(ApiError::Request)?;
        let mut guard = self.conn.lock().map_err(|_| ApiError::Unavailable)?;
        let db = self.db_path.as_path();

        // The shop's clock, the same one the copies are named for.
        let safety_copy = backup::safety_name(db, crate::routes::backups::now());
        let safety = db.with_file_name(&safety_copy);
        let live = guard.as_mut().ok_or(ApiError::Unavailable)?;
        if let Err(e) = backup::copy_to(live, &safety) {
            // `VACUUM INTO` that stopped part way through still wrote a file,
            // and it is a torn one under the name the backups screen lists
            // safety copies by.
            let _ = remove_if_present(&safety);
            return Err(ApiError::from(e));
        }

        let staged = sibling(db, ".restoring.tmp");
        // A leftover from a run that died mid-restore would make the copy
        // below fail; it describes nothing that is still wanted.
        if let Err(e) = remove_if_present(&staged) {
            let _ = std::fs::remove_file(&safety);
            return Err(core_io(e));
        }
        if let Err(e) = std::fs::copy(backup_path, &staged) {
            // A copy that stopped part way through still wrote a file, and
            // it is a torn one under a name that means "ready to be renamed
            // over the shop file".
            let _ = std::fs::remove_file(&staged);
            let _ = std::fs::remove_file(&safety);
            return Err(core_io(e));
        }

        // The last thing done through the live connection: its `-wal` is
        // folded back into the file and emptied while the connection that
        // owns it is still open. After this, a sidecar that survives the
        // swap describes nothing, which is what turns the failure below from
        // data loss into a restart.
        //
        // Every way out between the copy above and the rename below goes
        // through here, so the staged copy is removed in one place. It is a
        // whole database sitting beside the shop file under a name no screen
        // lists and no prune counts; the next restore would delete it
        // anyway, and until then it is a copy of the shop nobody knows is
        // there. The safety copy goes with it: the shop file was never
        // touched and this connection is still serving it, so the copy holds
        // the same data as the file next to it and nothing would ever prune
        // it away.
        if let Err(e) = fold_log_back(&mut guard) {
            let _ = std::fs::remove_file(&staged);
            let _ = std::fs::remove_file(&safety);
            return Err(e);
        }

        // From here the shop file is being replaced. Closing the connection
        // is what releases the handle, and the slot stays empty until a
        // reopen fills it.
        drop(guard.take());

        if let Err(rename_failed) = std::fs::rename(&staged, db) {
            let _ = std::fs::remove_file(&staged);
            let answer = reopen_original(&mut guard, db, rename_failed);
            // A filled slot means the shop file is there and opens, so the
            // safety copy is a duplicate of it and goes the way the staged
            // copy just did. An empty one means nothing here could open that
            // file: the copy taken while it still could is then the one
            // database known to open, and it stays for the owner to find.
            if guard.is_some() {
                let _ = std::fs::remove_file(&safety);
            }
            return Err(answer);
        }

        // The copy is in place. The sidecars beside it belong to the file it
        // replaced, and SQLite replays a `-wal` into whatever file it finds
        // under that name, so one that will not go means nothing may open
        // this file again in this process.
        if let Err((sidecar, e)) = clear_sidecars(db) {
            eprintln!(
                "dz-pos: {} holds the restored copy, but {} could not be removed, so it is not being reopened: {e}",
                db.display(),
                sidecar.display()
            );
            return Err(ApiError::RestartNeeded);
        }

        match dzpos_core::db::open(db) {
            Ok(conn) => {
                *guard = Some(conn);
                // Written to the file that was just opened, after the swap
                // and never before: `backup::record_restore`'s own doc says
                // why a row against the connection being replaced would not
                // survive being the thing overwritten. A failure here does
                // not fail the restore the caller already has: the data is
                // already replaced, and answering with an error would claim
                // a restore that succeeded had not.
                if let Some(live) = guard.as_mut() {
                    let restored_from = backup_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    if let Err(e) = dzpos_core::services::backup::record_restore(
                        live,
                        self.shop_id,
                        actor_id,
                        restored_from,
                        &safety_copy,
                        &summary,
                    ) {
                        eprintln!(
                            "dz-pos: the restore of {} finished but its audit row could not be written: {e}",
                            db.display()
                        );
                    }
                }
                Ok(Restored {
                    summary,
                    safety_copy,
                })
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
                Err(ApiError::RestartNeeded)
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
        // or a query from running against nothing, and the code says the one
        // thing that will help: relaunch.
        let conn = guard.as_mut().ok_or(ApiError::RestartNeeded)?;
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

/// Opens the shop's file and brings its schema up to this build, taking a
/// copy of the file first when there is a schema for a migration to change.
///
/// The copy is what an owner gets back if the new version turns out to read
/// the shop's books wrong. It sits beside the shop file under its own name
/// (`backup::before_upgrade`), is never pruned, and is taken only when both
/// halves are true: the file already holds a schema some version of this app
/// wrote, and this build has migrations it has not had. A brand new file gets
/// none, because a copy of an empty file is worth nothing and would sit there
/// for ever; an unchanged file gets none either, since the ordinary daily
/// copy already describes it.
///
/// A copy that cannot be written stops the app from starting. That is the
/// point of it: the alternative is a migration running with nothing to go
/// back to, on a disk that just said it was full.
fn open_and_upgrade(db: &Path) -> Result<Conn, CoreError> {
    let mut conn = dzpos_core::db::open_unmigrated(db)?;
    let pending = dzpos_core::db::pending_migrations(&mut conn)?;
    if !pending.is_empty() && dzpos_core::db::has_a_schema(&mut conn)? {
        backup::before_upgrade(&mut conn, db, dzpos_core::services::clock::now())?;
    }
    dzpos_core::db::migrate(&mut conn)?;
    Ok(conn)
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

/// Step 5 of a restore, as one fallible piece so its caller has a single
/// place to undo step 4 from. An empty slot here is not a state a restore
/// can be in: it holds the lock, and the safety copy a moment ago went
/// through the connection that would be missing.
fn fold_log_back(slot: &mut Option<Conn>) -> Result<(), ApiError> {
    let live = slot.as_mut().ok_or(ApiError::Unavailable)?;
    dzpos_core::db::checkpoint(live).map_err(|e| ApiError::from(CoreError::from(e)))
}

fn remove_if_present(path: &Path) -> Result<(), std::io::Error> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// The recovery after a rename that did not happen. The shop file was never
/// written over, so it is the one that was always there, sidecars and all,
/// and reopening it is the whole recovery: the till goes on working and the
/// caller hears about the rename.
fn reopen_original(slot: &mut Option<Conn>, db: &Path, rename_failed: std::io::Error) -> ApiError {
    match dzpos_core::db::open(db) {
        Ok(conn) => {
            *slot = Some(conn);
            core_io(rename_failed)
        }
        // Nothing fills the slot, so every route 500s from here. Answering
        // with the rename alone would describe half of that and send the
        // screen looking for a disk that is full; the code carries both
        // facts instead, and the relaunch is the one thing that helps.
        Err(reopen_failed) => {
            eprintln!(
                "dz-pos: the restore did not happen ({rename_failed}) and {} could not be reopened either: {reopen_failed}",
                db.display()
            );
            ApiError::NotRestoredRestartNeeded
        }
    }
}

/// Removes the `-wal` and `-shm` of the file that used to sit at `db`, and
/// names the one that would not go, because that is the file a person has to
/// deal with. There is no `-journal`: the shop file is opened in WAL mode, so
/// a rollback journal never exists beside it.
fn clear_sidecars(db: &Path) -> Result<(), (PathBuf, CoreError)> {
    for suffix in ["-wal", "-shm"] {
        let sidecar = sibling(db, suffix);
        remove_if_present(&sidecar).map_err(|e| (sidecar, CoreError::from(e)))?;
    }
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
    // The health route owes the same JSON 405 as every guarded one; the
    // method fallback below is the one on the router inside the guard.
    let open = Router::new().route(
        "/health",
        get(routes::health).fallback(routes::method_not_allowed),
    );
    // Which person is calling: the session (M4 T2, crate::session). Inside
    // the launch token and outside these three, because they are how a
    // session comes to exist; a sign-in behind a session guard would be a
    // lock with its key inside. They still show the launch token like
    // everything else.
    let auth = Router::new()
        .route("/auth/login", post(routes::auth::login))
        .route("/auth/logout", post(routes::auth::logout))
        .route("/auth/me", get(routes::auth::me))
        .route("/auth/first-pin", post(routes::auth::claim_first_pin));
    let guarded = Router::new()
        .route("/audit-log", get(routes::audit::list))
        .route("/auth/idle", get(routes::auth::idle))
        .route("/backups", get(routes::backups::list))
        .route("/backups", post(routes::backups::create))
        .route("/backups/{name}/restore", post(routes::backups::restore))
        .route("/cash", get(routes::expenses::cash))
        .route("/categories", get(routes::categories::list))
        .route("/clock", get(routes::clock))
        .route("/customers", get(routes::customers::list))
        .route("/customers", post(routes::customers::create))
        .route(
            "/customers/{id}",
            get(routes::customers::get_one).put(routes::customers::update),
        )
        .route("/customers/{id}/ledger", get(routes::customers::ledger))
        .route("/customers/{id}/payments", get(routes::customers::payments))
        .route("/customers/{id}/payments", post(routes::customers::pay))
        .route(
            "/customers/{id}/statement",
            get(routes::customers::statement),
        )
        .route(
            "/customers/{id}/debt-slip",
            get(routes::customers::debt_slip),
        )
        .route(
            "/customers/{id}/adjustments",
            post(routes::customers::adjust),
        )
        .route("/dashboard", get(routes::dashboard::read))
        .route("/dashboard/series", get(routes::dashboard::series))
        .route("/expense-categories", get(routes::expenses::categories))
        .route("/expenses", get(routes::expenses::list))
        .route("/expenses", post(routes::expenses::create))
        .route("/export/products", get(routes::export::products))
        .route("/export/sales", get(routes::export::sales))
        .route("/export/customers", get(routes::export::customers))
        .route("/export/suppliers", get(routes::export::suppliers))
        .route("/import/products/template", get(routes::import::template))
        .route(
            "/import/products/dry-run",
            post(routes::import::dry_run)
                .layer(DefaultBodyLimit::max(routes::import::IMPORT_BODY_LIMIT)),
        )
        .route(
            "/import/products",
            post(routes::import::apply)
                .layer(DefaultBodyLimit::max(routes::import::IMPORT_BODY_LIMIT)),
        )
        .route("/labels/sheet", post(routes::products::label_sheet))
        .route("/products", get(routes::products::list))
        .route("/products", post(routes::products::create))
        .route("/products/{id}/label", get(routes::products::label))
        .route(
            "/products/{id}",
            get(routes::products::get_one).put(routes::products::update),
        )
        .route("/purchases", get(routes::purchases::list))
        .route("/purchases", post(routes::purchases::create))
        .route("/purchases/{id}", get(routes::purchases::get_one))
        .route("/purchases/{id}/receipts", post(routes::purchases::receive))
        .route("/purchases/{id}/returns", post(routes::purchases::returns))
        .route("/purchases/{id}/cancel", post(routes::purchases::cancel))
        .route(
            "/purchases/{id}/close-short",
            post(routes::purchases::close_short),
        )
        .route("/sales", get(routes::sales::list))
        .route("/sales", post(routes::sales::create))
        .route("/sales/{id}", get(routes::sales::get_one))
        .route("/sales/{id}/avoir", post(routes::sales::avoir))
        .route("/sales/{id}/avoirs", get(routes::sales::avoirs))
        .route("/sales/{id}/cancel", post(routes::sales::cancel))
        .route("/sales/{id}/ticket", get(routes::sales::ticket))
        .route("/sales/{id}/facture", get(routes::sales::facture))
        .route("/settings", get(routes::settings::read))
        .route("/settings/store", put(routes::settings::update_store))
        .route("/settings/regime", post(routes::settings::change_regime))
        .route(
            "/settings/discount-threshold",
            post(routes::settings::set_discount_threshold),
        )
        .route("/settings/theme", put(routes::settings::set_theme))
        .route(
            "/stock/recount",
            get(routes::stock::last).post(routes::stock::recount),
        )
        .route("/suppliers", get(routes::suppliers::list))
        .route("/suppliers", post(routes::suppliers::create))
        .route(
            "/suppliers/{id}",
            get(routes::suppliers::get_one).put(routes::suppliers::update),
        )
        .route("/suppliers/{id}/close", post(routes::suppliers::close))
        .route("/suppliers/{id}/ledger", get(routes::suppliers::ledger))
        .route("/suppliers/{id}/payments", post(routes::suppliers::pay))
        .route(
            "/suppliers/{id}/adjustments",
            post(routes::suppliers::adjust),
        )
        .route(
            "/suppliers/{id}/statement",
            get(routes::suppliers::statement),
        )
        .route(
            "/users",
            get(routes::users::list).post(routes::users::create),
        )
        .route("/users/{id}/pin", post(routes::users::set_pin))
        .route("/users/{id}/deactivate", post(routes::users::deactivate))
        .route("/users/{id}/reactivate", post(routes::users::reactivate))
        // Every route above takes its actor from the session; nothing reads a
        // seeded owner id any more.
        .layer(from_fn_with_state(state.clone(), session::require));

    let inside_the_launch_token = auth
        .merge(guarded)
        .fallback(routes::not_found)
        .method_not_allowed_fallback(routes::method_not_allowed)
        .layer(from_fn_with_state(token.clone(), token::require));

    open.merge(inside_the_launch_token)
        // Every browser on the machine can reach 127.0.0.1, so
        // allow_origin(Any) let any page a user happened to open read and
        // write the till. The list is what keeps a stranger's page from
        // being handed the answer, and the preflight is answered here,
        // before the token check, because a browser sends it bare.
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods([Method::GET, Method::POST, Method::PUT])
                .allow_headers([
                    header::CONTENT_TYPE,
                    header::AUTHORIZATION,
                    // The desktop's session token travels in its own header
                    // (crate::session says why it is not Authorization); a
                    // header missing from this list is one the preflight
                    // silently kills.
                    header::HeaderName::from_static(crate::session::SESSION_HEADER),
                ])
                // The browser half of the same session sends an httpOnly
                // cookie, and a browser only attaches one cross-origin when
                // the server says it may. The origin list is a fixed list and
                // never `Any`, which is what makes this safe to turn on.
                .allow_credentials(true)
                // A browser hands a page only the few headers it is told to.
                // Without this the desktop can read the workbook's bytes and
                // not the name the server gave it, and every export would be
                // saved as whatever the anchor invented.
                .expose_headers([header::CONTENT_DISPOSITION]),
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

#[cfg(test)]
mod sidecar_tests {
    // Tests may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{clear_sidecars, sibling};

    #[test]
    fn the_sidecars_of_the_file_that_was_replaced_go_and_the_file_is_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("t.db");
        std::fs::write(&db, b"the restored copy").unwrap();
        std::fs::write(sibling(&db, "-wal"), b"wal").unwrap();
        std::fs::write(sibling(&db, "-shm"), b"shm").unwrap();

        clear_sidecars(&db).unwrap();
        assert!(!sibling(&db, "-wal").exists());
        assert!(!sibling(&db, "-shm").exists());
        assert_eq!(std::fs::read(&db).unwrap(), b"the restored copy");
    }

    #[test]
    fn a_folder_with_no_sidecars_in_it_is_not_a_failure() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("t.db");
        std::fs::write(&db, b"the restored copy").unwrap();
        clear_sidecars(&db).unwrap();
    }

    /// The name comes back because the caller has to log it: it is the file
    /// someone will have to remove before the app will open the shop again.
    #[cfg(unix)]
    #[test]
    fn one_that_will_not_go_is_named_back_to_the_caller() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("t.db");
        std::fs::write(&db, b"the restored copy").unwrap();
        let shm = sibling(&db, "-shm");
        std::fs::create_dir(&shm).unwrap();
        std::fs::write(shm.join("in the way"), b"x").unwrap();

        let (named, _) = clear_sidecars(&db).unwrap_err();
        assert_eq!(named, shm);
    }
}

#[cfg(test)]
mod reopen_original_tests {
    // Tests may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{reopen_original, ApiError};
    use dzpos_core::error::CoreError;

    fn rename_failed() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "rename refused")
    }

    /// The ordinary half: the shop file is still there and still opens, so
    /// the till goes on working and the only thing that went wrong is the
    /// one the caller asked about.
    #[test]
    fn a_shop_file_that_opens_again_leaves_the_till_working_and_names_the_rename() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("t.db");
        drop(dzpos_core::db::open(&db).unwrap());

        let mut slot = None;
        let err = reopen_original(&mut slot, &db, rename_failed());

        assert!(slot.is_some(), "the shop file was not put back in the slot");
        assert!(
            matches!(err, ApiError::Core(CoreError::Io(_))),
            "the rename is what failed and the answer should say so"
        );
    }

    /// Both halves gone: the rename did not happen and the file it would
    /// have replaced cannot be opened either. Answering with the rename
    /// alone would send a screen looking for a disk problem while every
    /// other route 500s on an empty slot, so the answer carries both facts
    /// and the one instruction that helps.
    #[test]
    fn a_shop_file_that_will_not_open_either_says_both_and_asks_for_a_relaunch() {
        let dir = tempfile::tempdir().unwrap();
        // A folder standing where the shop file was: SQLite will not open it
        // on any system, as any user.
        let db = dir.path().join("t.db");
        std::fs::create_dir(&db).unwrap();

        let mut slot = None;
        let err = reopen_original(&mut slot, &db, rename_failed());

        assert!(
            slot.is_none(),
            "a slot filled here would answer queries from a file nobody opened"
        );
        assert!(
            matches!(err, ApiError::NotRestoredRestartNeeded),
            "the caller was told about the rename only: {err}"
        );
    }
}
