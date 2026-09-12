use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::path::Path;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

/// The connection every service takes. Aliased so callers above this crate
/// name a core type and never depend on diesel themselves.
pub type Conn = SqliteConnection;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("connect: {0}")]
    Connect(#[from] diesel::ConnectionError),
    #[error("pragma: {0}")]
    Pragma(#[from] diesel::result::Error),
    #[error("migrate: {0}")]
    Migrate(String),
    /// SQLite would not let the write-ahead log be folded back: another
    /// connection is reading the file. Not a `Pragma` error, because the
    /// pragma answered perfectly well; what it answered was no.
    #[error("the write-ahead log could not be folded back ({log} pages in it, {checkpointed} of them moved): the shop file is in use by another connection")]
    CheckpointBusy { log: i32, checkpointed: i32 },
}

/// The row `PRAGMA wal_checkpoint` answers: whether it was blocked, how many
/// pages the log holds, and how many of those were moved into the main file
/// before it gave up. SQLite reports a checkpoint it could not take here
/// rather than as an error, so this row is the only place a caller learns it
/// did not happen, and the two counts are what say how far it got.
#[derive(QueryableByName)]
struct CheckpointRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    busy: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    log: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    checkpointed: i32,
}

/// Folds the write-ahead log back into the main file and truncates it to
/// nothing.
///
/// Called before the file is closed and replaced: a `-wal` holding committed
/// pages belongs to the database it was written for, and SQLite replays
/// whatever `-wal` it finds beside a file it opens. Emptying it first means
/// the sidecars left over a swap describe nothing, whether or not they can
/// be deleted afterwards.
pub fn checkpoint(conn: &mut SqliteConnection) -> Result<(), DbError> {
    let row: CheckpointRow =
        diesel::sql_query("PRAGMA wal_checkpoint(TRUNCATE)").get_result(conn)?;
    // `busy` is the whole point of reading the row. A blocked checkpoint
    // answers 1 and leaves the log where it was; executing the pragma and
    // dropping its answer reports that as a success. Only `busy` is treated
    // as the failure: the other columns are context for the log line, not a
    // second condition to trip over on a build that fills them differently.
    if row.busy != 0 {
        return Err(DbError::CheckpointBusy {
            log: row.log,
            checkpointed: row.checkpointed,
        });
    }
    Ok(())
}

/// Opens (creating if needed) the SQLite file and applies pending migrations.
pub fn open(path: impl AsRef<Path>) -> Result<SqliteConnection, DbError> {
    let mut conn = open_unmigrated(path)?;
    migrate(&mut conn)?;
    Ok(conn)
}

/// Opens (creating if needed) the SQLite file and stops there, with the
/// pragmas set and nothing migrated.
///
/// [`open`] is this plus [`migrate`], and is what almost everything wants.
/// This exists for the caller that has to do something between the two:
/// `open_and_upgrade` copies the shop's file before an upgrade touches it,
/// and it cannot know whether there is anything to copy until the file is
/// open (`crates/api/src/lib.rs`, reached from both the app's startup and
/// the reopen at the end of a restore).
pub fn open_unmigrated(path: impl AsRef<Path>) -> Result<SqliteConnection, DbError> {
    let url = path.as_ref().to_string_lossy();
    let mut conn = SqliteConnection::establish(&url)?;
    conn.batch_execute("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    Ok(conn)
}

/// Applies every migration the file has not had, and answers with their
/// names in the order they ran. An empty answer means the file was already
/// current.
pub fn migrate(conn: &mut SqliteConnection) -> Result<Vec<String>, DbError> {
    conn.run_pending_migrations(MIGRATIONS)
        .map(|versions| versions.iter().map(ToString::to_string).collect())
        .map_err(|e| DbError::Migrate(e.to_string()))
}

/// Whether this file has anything a migration could damage.
///
/// True once the file carries diesel's own bookkeeping table with at least
/// one row in it, which is the same as saying some version of this app has
/// opened it before. A file that answers false is either brand new or was
/// created by something else entirely, and copying it before the first
/// migration would leave a shop with a backup of nothing, taken on the day
/// they installed, sitting beside their file for ever.
pub fn has_a_schema(conn: &mut SqliteConnection) -> Result<bool, DbError> {
    #[derive(QueryableByName)]
    struct Count {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        n: i64,
    }
    let table: Count = diesel::sql_query(
        "SELECT count(*) AS n FROM sqlite_master \
         WHERE type = 'table' AND name = '__diesel_schema_migrations'",
    )
    .get_result(conn)?;
    if table.n == 0 {
        return Ok(false);
    }
    let applied: Count = diesel::sql_query("SELECT count(*) AS n FROM __diesel_schema_migrations")
        .get_result(conn)?;
    Ok(applied.n > 0)
}

/// The migrations this file has not had yet, by version, oldest first.
///
/// Asked before anything is written, so the app can say whether an upgrade
/// is about to happen and take a copy of the file first.
pub fn pending_migrations(conn: &mut SqliteConnection) -> Result<Vec<String>, DbError> {
    conn.pending_migrations(MIGRATIONS)
        .map(|pending| {
            pending
                .iter()
                .map(|m| m.name().version().as_owned().to_string())
                .collect()
        })
        .map_err(|e| DbError::Migrate(e.to_string()))
}
