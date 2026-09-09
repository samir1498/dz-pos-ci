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
    /// `None` until the documents table exists (it arrives with the sale).
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
/// The copy is named for `at`, so two copies in the same second are one
/// copy: the existing file is returned rather than overwritten, since a
/// double click on "back up now" must not destroy the copy it just made.
pub fn create(conn: &mut Conn, dir: &Path, at: NaiveDateTime) -> Result<Backup, CoreError> {
    std::fs::create_dir_all(dir)?;
    let name = file_name(at);
    let path = dir.join(&name);
    if !path.exists() {
        copy_to(conn, &path)?;
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
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(taken_at) = taken_at(&name) else {
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
    // (there cannot be, but the order must not depend on the file system)
    // still come out in a fixed order.
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
    let url = path.to_string_lossy();
    let mut conn =
        SqliteConnection::establish(&url).map_err(|_| refused("does not open as a database"))?;
    // Read-only from here on: a copy the owner may yet decide against must
    // come out of this check byte for byte as it went in.
    conn.batch_execute("PRAGMA query_only = ON;")
        .map_err(|_| refused("does not open as a database"))?;

    let checked: Vec<IntegrityRow> = diesel::sql_query("PRAGMA integrity_check")
        .load(&mut conn)
        .map_err(|_| refused("is not a database"))?;
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
/// older, and `db::open` migrates it forward on the way in); it may not have
/// more.
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

fn refused(why: &str) -> CoreError {
    CoreError::validation("backup", why)
}
