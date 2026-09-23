// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// Every test binary compiles this whole file and calls part of it.
#![allow(dead_code)]

//! What a clinic test needs before it can say anything: a migrated file of
//! its own, and a patient as empty as a file is allowed to be. Nothing here
//! decides anything; a test that needs a phone or a note sets it itself.

use diesel::{RunQueryDsl, SqliteConnection};
use dzpos_clinic::services::patients::NewPatient;

/// The shop and its owner the first migration seeds.
pub const SHOP: i32 = 1;
pub const OWNER: i32 = 1;

/// A database of this test's own, migrated and seeded, in a directory that
/// is deleted when the returned handle drops. Opened through the kernel, the
/// same door the app uses, so every migration in the one shared folder runs.
pub fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = dzpos_kernel::db::open(dir.path().join("t.db")).unwrap();
    (dir, conn)
}

/// A second shop in the same file, for the cases about rule 3.
pub fn second_shop(conn: &mut SqliteConnection) -> i32 {
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième cabinet')")
        .execute(conn)
        .unwrap();
    diesel::sql_query("INSERT INTO users (id, shop_id, name, role) VALUES (2, 2, 'Dr B', 'owner')")
        .execute(conn)
        .unwrap();
    2
}

/// A file with the two names and nothing else.
pub fn named(first: &str, last: &str) -> NewPatient {
    NewPatient {
        first_name: first.to_string(),
        last_name: last.to_string(),
        ..NewPatient::default()
    }
}
