use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::path::Path;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("connect: {0}")]
    Connect(#[from] diesel::ConnectionError),
    #[error("pragma: {0}")]
    Pragma(#[from] diesel::result::Error),
    #[error("migrate: {0}")]
    Migrate(String),
}

/// Opens (creating if needed) the SQLite file and applies pending migrations.
pub fn open(path: impl AsRef<Path>) -> Result<SqliteConnection, DbError> {
    let url = path.as_ref().to_string_lossy();
    let mut conn = SqliteConnection::establish(&url)?;
    conn.batch_execute("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| DbError::Migrate(e.to_string()))?;
    Ok(conn)
}
