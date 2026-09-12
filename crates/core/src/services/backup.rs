//! Backups of the shop file (features.md §1: automatic daily copy, keep 30,
//! restore from the settings screen).
//!
//! A backup is a plain SQLite file, unencrypted: encryption at rest is the
//! operating system's job (architecture.md, Data). It is made with
//! `VACUUM INTO`, never with a file copy: the live database runs in WAL
//! mode, so copying the `.db` alone captures a file whose last committed
//! pages are still in the `-wal` sidecar. `VACUUM INTO` writes a
//! transactionally consistent copy while the connection stays open.
//!
//! Nothing here decides *when* a backup happens; the caller passes the time
//! it reads. That keeps the wall clock out of every test.

use std::path::{Path, PathBuf};

use chrono::{Duration, NaiveDateTime};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::{Conn, MIGRATIONS};
use crate::error::CoreError;
use crate::services::audit;

/// How many copies the folder keeps (features.md §1). The oldest beyond it
/// goes when a new one is made.
pub const KEEP: usize = 30;

/// How stale the newest copy may be before one is due.
const EVERY_HOURS: i64 = 24;

const PREFIX: &str = "dzpos-";
const SUFFIX: &str = ".sqlite";
/// The stamp inside the name. Sorting the names sorts the backups, which is
/// why the order is never taken from the file system's mtime: a copy moved
/// between folders keeps its name and loses its mtime.
const STAMP: &str = "%Y%m%d-%H%M%S";
/// What sits between the shop file's own name and the stamp on a copy taken
/// on the way into a restore. Those live beside the shop file, not in the
/// backup folder, so the thirty daily copies can never prune one away: a
/// safety copy is the only record of what the shop looked like before the
/// owner replaced it.
const SAFETY_INFIX: &str = ".before-restore-";
/// What sits between the shop file's own name and the stamp on a copy taken
/// on the way into an upgrade. Beside the shop file rather than in the
/// backup folder, for the same reason a safety copy is: the thirty daily
/// copies prune, and the one copy of what the shop looked like before a new
/// version rewrote its schema must not be prunable. It is also the copy an
/// owner needs when the upgrade itself is the thing that went wrong, which
/// is the case no daily copy covers, because the daily one taken after the
/// upgrade is already in the new shape.
const UPGRADE_INFIX: &str = ".before-upgrade-";

/// The first sixteen bytes of every SQLite file. Checked before the file is
/// opened, because SQLite opens lazily: without it a text file and a torn
/// database both fail at the first query, and the copy the owner picked
/// would be refused with the wrong reason.
const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// What a copy is called while SQLite is still writing it. One extension
/// past what [`taken_at`] accepts, so a copy that never finished is invisible
/// to `list`, to `is_due` and to the restore route.
const STAGING_SUFFIX: &str = ".tmp";

/// One copy in the backup folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backup {
    /// The file name, which is also the id the API takes back on a restore.
    pub name: String,
    pub path: PathBuf,
    pub taken_at: NaiveDateTime,
    pub bytes: u64,
}

/// What a copy holds, read while checking it. The counts are what the
/// settings screen shows after a restore, so the owner sees the file
/// answered for itself rather than a bare "done".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub products: i64,
    /// `None` only for a copy taken before the documents table existed
    /// (migration 2); every copy since carries the count.
    pub documents: Option<i64>,
}

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    n: i64,
}

#[derive(QueryableByName)]
struct IntegrityRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    integrity_check: String,
}

#[derive(QueryableByName)]
struct VersionRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    version: String,
}

/// The name a copy taken at `at` gets.
pub fn file_name(at: NaiveDateTime) -> String {
    format!("{PREFIX}{}{SUFFIX}", at.format(STAMP))
}

/// The time a name carries, or `None` when the name is not one this app
/// wrote. This is the only gate on a name the API takes from a caller: no
/// separator, no dot and no `..` survives it, so a path can never be built
/// out of one.
pub fn taken_at(name: &str) -> Option<NaiveDateTime> {
    let stamp = name.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    let at = NaiveDateTime::parse_from_str(stamp, STAMP).ok()?;
    // "20260931-070000" parses as nothing, but a stamp that round-trips to
    // different text (a two-digit year, a stray sign) is refused here.
    (at.format(STAMP).to_string() == stamp).then_some(at)
}

/// The name a safety copy of `db` taken at `at` gets. The millisecond is
/// part of it: two restores inside one second must not write to one name,
/// and `copy_to` refuses a target that exists rather than overwrite it.
pub fn safety_name(db: &Path, at: NaiveDateTime) -> String {
    let stem = db
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    format!(
        "{stem}{SAFETY_INFIX}{}-{}{SUFFIX}",
        at.format(STAMP),
        at.format("%3f")
    )
}

/// The time a safety copy of `db` carries, or `None` when the name is not
/// one this app wrote beside that file.
pub fn safety_taken_at(db: &Path, name: &str) -> Option<NaiveDateTime> {
    let stem = db.file_name()?.to_str()?;
    let rest = name
        .strip_prefix(stem)?
        .strip_prefix(SAFETY_INFIX)?
        .strip_suffix(SUFFIX)?;
    let (stamp, millis) = rest.rsplit_once('-')?;
    if millis.len() != 3 || !millis.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let at = NaiveDateTime::parse_from_str(stamp, STAMP).ok()?;
    (at.format(STAMP).to_string() == stamp).then_some(at)
}

/// The name a copy taken before an upgrade gets. Same shape as
/// [`safety_name`], including the millisecond, and a different word in the
/// middle so the two kinds never sort into one list.
pub fn upgrade_name(db: &Path, at: NaiveDateTime) -> String {
    let stem = db
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    format!(
        "{stem}{UPGRADE_INFIX}{}-{}{SUFFIX}",
        at.format(STAMP),
        at.format("%3f")
    )
}

/// The time a pre-upgrade copy of `db` carries, or `None` when the name is
/// not one this app wrote beside that file.
pub fn upgrade_taken_at(db: &Path, name: &str) -> Option<NaiveDateTime> {
    let stem = db.file_name()?.to_str()?;
    let rest = name
        .strip_prefix(stem)?
        .strip_prefix(UPGRADE_INFIX)?
        .strip_suffix(SUFFIX)?;
    let (stamp, millis) = rest.rsplit_once('-')?;
    if millis.len() != 3 || !millis.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let at = NaiveDateTime::parse_from_str(stamp, STAMP).ok()?;
    (at.format(STAMP).to_string() == stamp).then_some(at)
}

/// Every pre-upgrade copy beside the shop file, newest first.
pub fn list_upgrade(db: &Path) -> Result<Vec<Backup>, CoreError> {
    let Some(dir) = folder_of(db) else {
        return Ok(Vec::new());
    };
    collect(&dir, |name| upgrade_taken_at(db, name))
}

/// The folder a file sits in, with a bare name reading as the working
/// folder rather than as nothing. `Path::parent` answers `Some("")` for
/// `"shop.sqlite"`, and an empty path opens nothing.
fn folder_of(file: &Path) -> Option<PathBuf> {
    let dir = file.parent()?;
    Some(if dir.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        dir.to_path_buf()
    })
}

/// Writes a copy of the shop's file beside itself, before a migration runs.
///
/// One copy per upgrade that has migrations to run, kept for good. A shop
/// that takes ten releases with schema changes in them ends up with ten of
/// these beside its file, and nothing deletes them: the whole point of the
/// copy is that it is still there months later, when somebody works out that
/// the trouble started with an update.
///
/// Called on the way into the shop file, from both doors: the app's startup,
/// and the reopen at the end of a restore, where the copy being restored is
/// usually the one behind. It takes no audit row and could not: nobody is
/// signed in when a new version opens an old file, and on the restore path
/// the file is not open yet. What records it is the copy itself, whose name
/// carries the moment.
///
/// A shop that restores an older copy twice gets two of these, because the
/// two copies hold different books. Nothing deletes them, which is the point
/// above, and nothing lists them yet either: putting them on the backups
/// screen beside the daily ones is its own task (M5 T11).
///
/// The copy is written under a staging name and renamed once SQLite has
/// finished with it, the same way [`create`] does, so a copy interrupted by
/// a full disk is never mistaken for a complete one. Two copies in the same
/// millisecond are one copy: an existing target is returned rather than
/// overwritten.
pub fn before_upgrade(conn: &mut Conn, db: &Path, at: NaiveDateTime) -> Result<Backup, CoreError> {
    let Some(dir) = folder_of(db) else {
        return Err(refused(
            "the shop file has no folder to write a pre-upgrade copy into",
        ));
    };
    // A copy killed mid `VACUUM INTO` leaves a staging file holding up to a
    // whole database, and the retry that follows it is at a later moment and
    // so under a different name, which would leave the first one there for
    // good. Nothing else reads these names, so this is the only thing that
    // would ever clear them.
    sweep_staging(&dir, |stem| upgrade_taken_at(db, stem));
    let name = upgrade_name(db, at);
    let path = dir.join(&name);
    if path.is_file() {
        let bytes = std::fs::metadata(&path)?.len();
        return Ok(Backup {
            name,
            path,
            taken_at: at,
            bytes,
        });
    }
    let staging = dir.join(format!("{name}{STAGING_SUFFIX}"));
    remove_if_present(&staging)?;
    let written = copy_to(conn, &staging).and_then(|()| Ok(std::fs::rename(&staging, &path)?));
    if let Err(e) = written {
        let _ = std::fs::remove_file(&staging);
        return Err(e);
    }
    let bytes = std::fs::metadata(&path)?.len();
    Ok(Backup {
        name,
        path,
        taken_at: at,
        bytes,
    })
}

/// Every safety copy beside the shop file, newest first.
pub fn list_safety(db: &Path) -> Result<Vec<Backup>, CoreError> {
    let Some(dir) = db.parent() else {
        return Ok(Vec::new());
    };
    let dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    collect(dir, |name| safety_taken_at(db, name))
}

/// Is a copy due? True when there is none, or when the newest is a full day
/// behind `now`. A newest dated ahead of `now` (the till's clock was
/// corrected backwards) is not due: it would otherwise be due on every tick.
pub fn is_due(newest: Option<NaiveDateTime>, now: NaiveDateTime) -> bool {
    match newest {
        None => true,
        Some(at) => now.signed_duration_since(at) >= Duration::hours(EVERY_HOURS),
    }
}

/// Writes a consistent copy of the database `conn` is open on into `dir`,
/// then prunes the folder back to [`KEEP`].
///
/// The copy is written under `<name>.tmp` and renamed only once SQLite has
/// finished with it. A `VACUUM INTO` that stops halfway (the disk filled)
/// can leave its target behind, and under the final name that half a file
/// would be the newest copy the list reports, the reason no copy is due for
/// a day, and something a restore would be offered. The staging name
/// carries one extension more than the pattern allows, so nothing that
/// reads this folder ever trusts it.
///
/// The copy is named for `at`, so two copies in the same second are one
/// copy: an existing file is returned rather than overwritten, since a
/// double click on "back up now" must not destroy the copy it just made.
///
/// Writes no audit row, on purpose (a review asked the question
/// separately from the restore it sits beside). A restore rewrites every row
/// the shop has and is the act this milestone's control is about; a backup
/// changes nothing a screen reads and nothing walks out of the shop, it only
/// exists in a folder beside the file it copied. It is also, unlike a
/// restore, routinely taken with nobody signed in at all: `crates/api/src/
/// daily.rs` calls this once a day on a timer, and a row that named an actor
/// for that call would be naming whoever happened to be signed in when the
/// hourly check fired, which is not who backed the shop up, and a row with
/// no actor at all is not something this log has a shape for. A row that
/// fired once a day forever regardless would also be exactly the noise
/// `permissions::record_refusal`'s own doc warns against: present for every
/// reader, informative to none.
pub fn create(conn: &mut Conn, dir: &Path, at: NaiveDateTime) -> Result<Backup, CoreError> {
    std::fs::create_dir_all(dir)?;
    sweep_staging(dir, taken_at);
    let name = file_name(at);
    let path = dir.join(&name);
    if path.is_file() {
        let bytes = std::fs::metadata(&path)?.len();
        prune(dir, KEEP)?;
        return Ok(Backup {
            name,
            path,
            taken_at: at,
            bytes,
        });
    }

    let staging = dir.join(format!("{name}{STAGING_SUFFIX}"));
    remove_if_present(&staging)?;
    let written = copy_to(conn, &staging).and_then(|()| Ok(std::fs::rename(&staging, &path)?));
    if let Err(e) = written {
        // Whatever is under the staging name describes nothing anyone wants,
        // and leaving it would make the next copy of the same second fail.
        let _ = std::fs::remove_file(&staging);
        return Err(e);
    }

    let bytes = std::fs::metadata(&path)?.len();
    prune(dir, KEEP)?;
    Ok(Backup {
        name,
        path,
        taken_at: at,
        bytes,
    })
}

/// Deletes every staging file left in `dir` by a copy that never finished.
///
/// A process killed mid `VACUUM INTO` leaves one holding up to a whole
/// database, and nothing that reads this folder can see it: it is not in
/// `list`, so `prune` never counts it and the thirty copies the shop is
/// allowed to keep quietly become twenty nine plus a corpse. Only names this
/// module writes are swept, so a `.tmp` someone else put here is left alone.
///
/// `reads` says which names in this folder are the caller's own, because the
/// two kinds of copy live under different names and a sweep must not delete
/// a staging file the other kind is in the middle of writing.
///
/// Safe to run at the top of `create` and of [`before_upgrade`] because
/// copies are taken one at a time: they go through the one connection, which
/// is behind one lock.
///
/// Nothing here can fail the copy it precedes. Housekeeping that refuses to
/// finish is not a reason to stop backing the shop up, so a folder that will
/// not list, an entry that will not stat and a file that will not delete are
/// all left for the next run.
fn sweep_staging(dir: &Path, reads: impl Fn(&str) -> Option<NaiveDateTime>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_ours = name
            .strip_suffix(STAGING_SUFFIX)
            .is_some_and(|stem| reads(stem).is_some());
        if !is_ours {
            continue;
        }
        // The entry itself, never what it points at. The check used to be
        // `is_file()`, and a symlink is not a file, so one standing on a
        // staging name survived every sweep however long it sat there. It is
        // in the way of the copy that wants that name whatever it points at,
        // and `remove_file` unlinks the link rather than its target.
        //
        // A folder is the one thing left alone: this module never writes
        // one, so it belongs to whoever did.
        if entry.path().symlink_metadata().is_ok_and(|m| m.is_dir()) {
            continue;
        }
        let _ = std::fs::remove_file(entry.path());
    }
}

fn remove_if_present(path: &Path) -> Result<(), CoreError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// `VACUUM INTO target`. The target must not exist and its folder must, both
/// of which SQLite enforces itself.
pub fn copy_to(conn: &mut Conn, target: &Path) -> Result<(), CoreError> {
    // The path goes into SQL as a literal because `VACUUM INTO` takes no
    // bind parameter. A single quote in a Windows user's folder name would
    // otherwise end the literal, so it is doubled, the one escape SQLite
    // defines for a string literal.
    let literal = target.to_string_lossy().replace('\'', "''");
    conn.batch_execute(&format!("VACUUM INTO '{literal}'"))?;
    Ok(())
}

/// Deletes the copies past the newest `keep`, oldest first. Returns what
/// went, so a caller can log it.
pub fn prune(dir: &Path, keep: usize) -> Result<Vec<PathBuf>, CoreError> {
    let mut gone = Vec::new();
    for old in list(dir)?.into_iter().skip(keep) {
        std::fs::remove_file(&old.path)?;
        gone.push(old.path);
    }
    Ok(gone)
}

/// Every copy in `dir`, newest first. A folder that was never written to is
/// empty, not a failure; anything whose name this app did not write is not a
/// backup and is left alone.
pub fn list(dir: &Path) -> Result<Vec<Backup>, CoreError> {
    collect(dir, taken_at)
}

/// Every entry of `dir` whose name `reads` gives a time to, newest first. A
/// folder that was never written to is empty, not a failure, and anything
/// that is not a file is not a copy.
fn collect(
    dir: &Path,
    reads: impl Fn(&str) -> Option<NaiveDateTime>,
) -> Result<Vec<Backup>, CoreError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(taken_at) = reads(&name) else {
            continue;
        };
        let meta = entry.metadata()?;
        if !meta.is_file() {
            continue;
        }
        found.push(Backup {
            name,
            path: entry.path(),
            taken_at,
            bytes: meta.len(),
        });
    }
    // Newest first. The name breaks a tie so two copies of the same second
    // still come out in a fixed order rather than the file system's.
    found.sort_by(|a, b| b.taken_at.cmp(&a.taken_at).then(b.name.cmp(&a.name)));
    Ok(found)
}

/// Opens the copy, checks it, and reports what it holds. Every failure is a
/// validation error: the caller picked this file, and the answer it needs is
/// "this copy will not do", not "the server broke".
///
/// Three things are checked, in order: the file is there and opens as a
/// database, `PRAGMA integrity_check` passes, and every migration the copy
/// has applied is one this build knows. That last one is what stops a copy
/// taken by a newer version from being restored into an older app, where the
/// schema would be ahead of the code reading it and no forward-only
/// migration could put it back.
pub fn verify(path: &Path) -> Result<Summary, CoreError> {
    if !std::fs::metadata(path).is_ok_and(|m| m.is_file()) {
        return Err(refused("is not a file this shop can read"));
    }
    if !starts_like_sqlite(path) {
        return Err(refused("is not a database"));
    }
    let url = path.to_string_lossy();
    let mut conn =
        SqliteConnection::establish(&url).map_err(|_| refused("does not open as a database"))?;
    // Read-only from here on: a copy the owner may yet decide against must
    // come out of this check byte for byte as it went in.
    conn.batch_execute("PRAGMA query_only = ON;")
        .map_err(|_| refused("does not open as a database"))?;

    // Damage inside the file surfaces either as a row naming what is wrong
    // or as an error on the read itself, depending on which page is torn.
    // Both are the integrity check saying no, and the message says so: the
    // file opened, so "not a database" would be the wrong thing to tell
    // someone choosing between copies.
    let checked: Vec<IntegrityRow> = diesel::sql_query("PRAGMA integrity_check")
        .load(&mut conn)
        .map_err(|_| refused("failed SQLite's integrity check"))?;
    if !checked.iter().all(|row| row.integrity_check == "ok") {
        return Err(refused("failed SQLite's integrity check"));
    }

    let applied: Vec<VersionRow> =
        diesel::sql_query("SELECT version FROM __diesel_schema_migrations")
            .load(&mut conn)
            .map_err(|_| refused("is not a dz-pos shop file"))?;
    let known = embedded_versions()?;
    if let Some(ahead) = applied.iter().find(|row| !known.contains(&row.version)) {
        return Err(CoreError::validation(
            "backup",
            &format!(
                "was written by a newer version of the app (migration {}), which this one cannot read",
                ahead.version
            ),
        ));
    }

    Ok(Summary {
        products: count(&mut conn, "products")?.unwrap_or(0),
        documents: count(&mut conn, "documents")?,
    })
}

/// The migrations compiled into this binary. A copy may have fewer (it is
/// older, and the restore migrates it forward on the way in, taking a
/// pre-upgrade copy of it first); it may not have more.
fn embedded_versions() -> Result<Vec<String>, CoreError> {
    use diesel::migration::MigrationSource;
    let migrations = MigrationSource::<diesel::sqlite::Sqlite>::migrations(&MIGRATIONS)
        .map_err(|e| CoreError::Db(crate::db::DbError::Migrate(e.to_string())))?;
    Ok(migrations
        .iter()
        .map(|m| m.name().version().to_string())
        .collect())
}

/// `None` when the table is not in this copy's schema yet.
fn count(conn: &mut SqliteConnection, table: &str) -> Result<Option<i64>, CoreError> {
    let present: Vec<Count> = diesel::sql_query(
        "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = ?",
    )
    .bind::<diesel::sql_types::Text, _>(table)
    .load(conn)
    .map_err(|_| refused("is not a dz-pos shop file"))?;
    if present.first().is_none_or(|row| row.n == 0) {
        return Ok(None);
    }
    // The name comes from this function's own callers, never from a request.
    let row: Count = diesel::sql_query(format!("SELECT COUNT(*) AS n FROM {table}"))
        .get_result(conn)
        .map_err(|_| refused("is not a dz-pos shop file"))?;
    Ok(Some(row.n))
}

/// Does the file begin the way every SQLite file does?
fn starts_like_sqlite(path: &Path) -> bool {
    use std::io::Read;
    let mut head = [0_u8; SQLITE_MAGIC.len()];
    std::fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut head))
        .is_ok()
        && &head == SQLITE_MAGIC
}

fn refused(why: &str) -> CoreError {
    CoreError::validation("backup", why)
}

/// The row a restore writes: a restore used to leave nothing
/// behind. `crate::AppState::restore` (`crates/api/src/lib.rs`) is the only
/// caller, and it calls this only after the swap has already happened and
/// the connection reopened, never before: a restore is the one act that
/// replaces the whole audit log along with everything else the shop file
/// holds, so a row written against the connection being replaced would be
/// gone the instant the replacement landed, and the restore would still look
/// silent to whoever reads the log afterwards. Written after, it is the
/// first row the shop's log carries on the file that is now live, which is
/// as much as a row can promise about an act that can rewrite the very log
/// it is written to: the row is still worth writing, because everything
/// written to the *new* file from here on is genuinely durable, and an owner
/// reading the log for "was this shop ever restored" has an honest answer
/// for every restore since the one this call is about, even though an
/// attacker who controlled a restore could in principle also choose a copy
/// old enough to omit this very call's own effect on what the log looks
/// like going forward. That is a property of restoring a database, not a gap
/// this row could close by being shaped differently.
///
/// A failure to write this row does not fail the restore that already
/// happened: the data is already replaced, and answering the caller with an
/// error would claim a restore that succeeded had not. `crate::AppState::
/// restore` logs that failure to the server's own log instead.
pub fn record_restore(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    restored_from: &str,
    safety_copy: &str,
    summary: &Summary,
) -> Result<(), CoreError> {
    audit::record(
        conn,
        shop_id,
        actor_id,
        audit::Change {
            action: audit::ACTION_RESTORE_BACKUP,
            entity: "backup",
            entity_id: None,
            before: None,
            after: Some(
                serde_json::json!({
                    "restored_from": restored_from,
                    "safety_copy": safety_copy,
                    "products": summary.products,
                    "documents": summary.documents,
                })
                .to_string(),
            ),
        },
    )
}
