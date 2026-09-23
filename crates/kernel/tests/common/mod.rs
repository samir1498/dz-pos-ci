// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! What a kernel test needs before it can say anything: a database of its
//! own. `crates/retail/tests/common/mod.rs` is the sibling of this file for
//! the retail crate's tests, which also need a customer or a supplier to
//! sell to; nothing here does, because the eleven domain-free services
//! this crate's tests exercise never sell anything.

use diesel::SqliteConnection;

/// A database of this test's own, migrated and seeded, in a directory that
/// is deleted when the returned handle drops. The handle comes back first
/// because a test that drops it keeps the connection open on a file that is
/// no longer there.
pub fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let conn = dzpos_kernel::db::open(&path).unwrap();
    (dir, conn)
}
