use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::path::Path;

pub fn open(path: impl AsRef<Path>) -> ConnectionResult<SqliteConnection> {
    let mut conn = SqliteConnection::establish(path.as_ref().to_str().unwrap())?;
    conn.batch_execute("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .expect("Failed to set pragmas");
    run_migrations(&mut conn);
    Ok(conn)
}

pub fn run_migrations(conn: &mut SqliteConnection) {
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run database migrations");
}
