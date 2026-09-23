//! The product's only entry point. The desktop webview, the browser
//! preview and, later, the phone are equal callers (architecture.md rule 2),
//! so these routes and their tests are the product's contract.

pub mod daily;
pub mod device;
pub mod dto;
pub mod error;
pub mod gates;
pub mod mdns;
pub mod router;
pub mod routes;
pub mod session;
pub mod token;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::NaiveDateTime;
use dzpos_core::db::Conn;
use dzpos_core::error::CoreError;
use dzpos_core::services::backup::{self, Backup};
use dzpos_core::services::support_bundle;
use dzpos_core::shop_counts::{for_bundle, for_verify, BackupCounts};

use crate::error::ApiError;
pub use crate::router::{origin_from_flag, router, router_with_origin};
pub use crate::session::CurrentUser;
pub use crate::token::LaunchToken;

/// Every copy of a shop file that exists, by the rule that keeps it.
pub struct Copies {
    /// The daily copies, in the backup folder, pruned back to thirty.
    pub daily: Vec<Backup>,
    /// Taken on the way into a restore, holding what the restore replaced.
    /// Beside the shop file rather than in the folder, so no prune reaches
    /// them.
    pub safety: Vec<Backup>,
    /// Taken on the way into an upgrade that has migrations to run, holding
    /// the file in the shape the older version left it. Beside the shop
    /// file for the same reason.
    pub upgrade: Vec<Backup>,
}

/// What a restore leaves behind: what the shop file now holds, and the name
/// of the copy of the old one taken on the way. The owner is told that name
/// because it is the only record of what was replaced, and nothing deletes
/// it.
pub struct Restored {
    pub summary: BackupCounts,
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
        // Every process that opens a shop file is a session (the standalone
        // binary and the desktop both call through here), so this is the one
        // place that heads the log rather than a line duplicated at each of
        // their two `main`/`run` functions. Best effort: a log line that
        // could not be written is not a reason to refuse the till.
        //
        // Written before the file is opened, not after: a build that refuses
        // to start because it could not copy the shop file before migrating
        // it is exactly the session somebody will want named in this file,
        // and a line written afterwards would never appear.
        let log_path = default_log_path(db.as_ref());
        if let Err(e) = dzpos_core::log::head_session(&log_path) {
            eprintln!("dz-pos: could not write to {}: {e}", log_path.display());
        }
        // Startup takes the copy and says nothing about it: there is no
        // audit row to write, because nobody is signed in when a new version
        // opens an old file. The restore below does name the one it takes.
        let (conn, _upgrade_copy) = open_and_upgrade(db.as_ref())?;
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

    /// Every copy this shop has. Three kinds, because three rules: the
    /// daily ones in the backup folder are pruned to thirty, and the two
    /// kinds beside the shop file are never pruned at all, which is why
    /// they sit outside the folder the prune walks.
    pub fn list_backups(&self) -> Result<Copies, ApiError> {
        Ok(Copies {
            daily: backup::list(&self.backup_dir).map_err(ApiError::from)?,
            safety: backup::list_safety(&self.db_path).map_err(ApiError::from)?,
            upgrade: backup::list_upgrade(&self.db_path).map_err(ApiError::from)?,
        })
    }

    /// A copy of the shop file, taken at `at`, and the folder pruned back to
    /// thirty.
    pub fn create_backup(&self, at: NaiveDateTime) -> Result<Backup, ApiError> {
        let dir = Arc::clone(&self.backup_dir);
        self.with_conn(|conn| backup::create(conn, &dir, at))
    }

    /// The support bundle (M5 T3): the log file beside the shop file, the build's own version and
    /// migration history, the shape of the schema, a handful of counts and sizes, and the machine's
    /// OS, language and time zone, zipped into one file a shop can send. Every rule about what goes
    /// in lives in `support_bundle`; this reads the three paths its `gather` wants off `self`, the
    /// retail counts through `shop_counts::for_bundle`, and hands the connection over the same way
    /// every other write here does. The log is read before the connection is taken, not after: it
    /// is a plain file with its own lock story, and reading it holds the connection's mutex for no
    /// longer than building the zip needs to.
    ///
    /// `actor_id` is who asked, for the audit row `support_bundle::record` writes once the zip is
    /// built and before it is handed back, the same order an export's row is written in.
    pub fn support_bundle(&self, actor_id: i32) -> Result<Vec<u8>, ApiError> {
        let log =
            support_bundle::read_log(&default_log_path(&self.db_path)).map_err(ApiError::from)?;
        let db_path = Arc::clone(&self.db_path);
        let backup_dir = Arc::clone(&self.backup_dir);
        let shop_id = self.shop_id;
        self.with_conn(|conn| -> Result<Vec<u8>, CoreError> {
            let retail_counts = for_bundle(conn)?;
            let facts = support_bundle::gather(conn, &db_path, &backup_dir, &retail_counts)?;
            let bytes = support_bundle::build_zip(&facts, &log)?;
            support_bundle::record(conn, shop_id, actor_id)?;
            Ok(bytes)
        })
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
    /// 9. reopen through `open_and_upgrade`, the same door startup uses, so
    ///    a copy that is behind on migrations gets a pre-upgrade copy of
    ///    itself taken before any migration changes it, and put that
    ///    connection back into the slot.
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
    ///   over harmless. A copy of the restored file that cannot be written
    ///   is one of the ways step 9 fails now, and that is the point: it is
    ///   the same refusal the app makes at startup rather than migrating a
    ///   shop's books with nothing to go back to. The restore itself stands;
    ///   the file on disk is the copy the owner asked for, unmigrated.
    pub fn restore(&self, backup_path: &Path, actor_id: i32) -> Result<Restored, ApiError> {
        let summary = backup::verify(backup_path, for_verify).map_err(ApiError::Request)?;
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

        // The same door the app starts through, and not `db::open`, which
        // would migrate the copy where it stands. A copy being restored is
        // usually behind: that is what an owner reaches for one when the new
        // version reads the books wrong. The copy itself is still in the
        // backups folder, but it is a daily one and the prune counts it, so
        // thirty days later nothing would hold the shape it had before this
        // migration. A pre-upgrade copy is never pruned. The safety copy
        // taken at step 3 is not that copy: it holds the file this restore
        // replaced, not the one it is about to migrate.
        match open_and_upgrade(db) {
            Ok((conn, upgrade_copy)) => {
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
                        upgrade_copy.as_ref().map(|c| c.name.as_str()),
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
                //
                // Both file names go in the line, because this is where a
                // full disk lands and the audit row that would have named
                // them is in the arm above: the copy this restore put in
                // place is not the shop's own file any more, and the one
                // holding what the shop had an hour ago is the safety copy.
                // Whoever is helping the shop reads this line and nothing
                // else, since the relaunch that follows hits the same
                // refusal and never gets far enough to write a row.
                eprintln!(
                    "dz-pos: the shop file at {} could not be reopened after the restore of {}: {e}. \
                     The file it replaced is in {} beside it.",
                    db.display(),
                    backup_path.display(),
                    safety_copy
                );
                Err(ApiError::RestartNeeded)
            }
        }
    }

    /// Runs one closure against the connection; a `CoreError` or a shop's
    /// `RetailError` maps to `ApiError`, and a poisoned lock to a 500.
    pub fn with_conn<T, E>(&self, f: impl FnOnce(&mut Conn) -> Result<T, E>) -> Result<T, ApiError>
    where
        ApiError: From<E>,
    {
        let mut guard = self.conn.lock().map_err(|_| ApiError::Unavailable)?;
        // Empty means a restore closed the file and could not reopen it.
        // Every caller is refused from here on, which is what keeps a backup
        // or a query from running against nothing, and the code says the one
        // thing that will help: relaunch.
        let conn = guard.as_mut().ok_or(ApiError::RestartNeeded)?;
        f(conn).map_err(ApiError::from)
    }

    /// Same, off the async executor: a synchronous diesel query must not hold a runtime thread.
    pub async fn blocking<T, E, F>(&self, f: F) -> Result<T, ApiError>
    where
        T: Send + 'static,
        ApiError: From<E>,
        F: FnOnce(&mut Conn) -> Result<T, E> + Send + 'static,
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
/// Both doors into the shop file come through here: the app's startup, and
/// the reopen at the end of `AppState::restore`, where the copy being put in
/// place is usually a version behind. The copy comes back with the
/// connection, because the restore writes its name into the row it records
/// and nothing else would know it: startup has nobody to write a row for.
///
/// A copy that cannot be written stops the app from starting, and stops a
/// restore from reopening the file it just put in place. That is the point of
/// it: the alternative is a migration running with nothing to go back to, on
/// a disk that just said it was full.
fn open_and_upgrade(db: &Path) -> Result<(Conn, Option<backup::Backup>), CoreError> {
    let mut conn = dzpos_core::db::open_unmigrated(db)?;
    let pending = dzpos_core::db::pending_migrations(&mut conn)?;
    let mut copy = None;
    if !pending.is_empty() && dzpos_core::db::has_a_schema(&mut conn)? {
        copy = Some(backup::before_upgrade(
            &mut conn,
            db,
            dzpos_core::services::clock::now(),
        )?);
    }
    dzpos_core::db::migrate(&mut conn)?;
    Ok((conn, copy))
}

/// `<db folder>/backups`. A shop file with no parent (a bare `t.db` on a
/// relative path) keeps the folder in the working directory.
pub fn default_backup_dir(db: &Path) -> PathBuf {
    match db.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(BACKUP_DIR_NAME),
        _ => PathBuf::from(BACKUP_DIR_NAME),
    }
}

/// `dzpos.log` beside the shop file, the same convention `default_backup_dir`
/// uses for its own folder: a shop file with no parent keeps the log in the
/// working directory instead of losing it to a path nothing else reads.
pub const LOG_FILE_NAME: &str = "dzpos.log";

pub fn default_log_path(db: &Path) -> PathBuf {
    match db.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(LOG_FILE_NAME),
        _ => PathBuf::from(LOG_FILE_NAME),
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

/// Binds loopback only, on `port`, and reports the port actually bound so
/// a caller passing 0 can tell the UI where to look. A till's database
/// must never be reachable from the shop's wifi; LAN mode in M6 is a
/// deliberate, separate decision.
pub async fn bind(port: u16) -> std::io::Result<(tokio::net::TcpListener, u16)> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    let bound = listener.local_addr()?.port();
    Ok((listener, bound))
}

/// Binds on all interfaces for LAN mode (M6 T1). The phone on the same
/// Wi-Fi finds the desktop via mDNS (`mdns::register`), then talks to this
/// port. The caller must still show the launch token and a session; the
/// network being reachable is not the permission (docs/architecture.md,
/// "Transport and auth" + M6 TLS decision). `0` still lets the OS pick.
pub async fn bind_lan(port: u16) -> std::io::Result<(tokio::net::TcpListener, u16)> {
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
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
mod log_path_tests {
    use super::default_log_path;
    use std::path::Path;

    #[test]
    fn the_log_sits_beside_the_shop_file() {
        assert_eq!(
            default_log_path(Path::new("/data/dzpos/shop.db")),
            Path::new("/data/dzpos/dzpos.log")
        );
    }

    #[test]
    fn a_shop_file_with_no_parent_keeps_the_log_in_the_working_directory() {
        assert_eq!(
            default_log_path(Path::new("shop.db")),
            Path::new("dzpos.log")
        );
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
