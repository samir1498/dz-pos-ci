// A test may panic; the deny is for shipped code.
#![allow(clippy::expect_used)]

use diesel::sqlite::SqliteConnection;

/// The seeded shop and the user who owns it, both written by the first
/// migration.
pub(crate) const SHOP: i32 = 1;
pub(crate) const OWNER: i32 = 1;

/// The handle comes back first because a test that drops it keeps the
/// connection open on a file that is no longer there. `dzpos_kernel::db`
/// since the kernel crate split: `db.rs` and the migrations it embeds
/// stayed in the kernel.
pub(crate) fn open() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().expect("a temp directory");
    let path = dir.path().join("t.db");
    let conn = dzpos_kernel::db::open(&path).expect("a migrated file");
    (dir, conn)
}
