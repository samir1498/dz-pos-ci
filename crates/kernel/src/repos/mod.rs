//! The only modules that run diesel queries. Every query takes a `shop_id`.
//! The seven domain-free repos; the seventeen retail repos moved to
//! `dzpos-retail` in the kernel crate split (S3 of
//! `a-kernel-crate-and-retail-as-the-first-module`), and `contains_pattern`
//! with them (its only two callers, `customers` and `suppliers`, are both
//! retail).
pub mod audit;
pub mod pairing;
pub mod preferences;
pub mod sessions;
pub mod settings;
pub mod shops;
pub mod users;

/// A migrated file of a test's own. The repos above are crate-internal, so
/// their round trips are checked from inside the crate rather than from
/// `tests/`, and this is the one place that opens the file for them.
#[cfg(test)]
pub(crate) mod testdb {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::expect_used)]

    use diesel::sqlite::SqliteConnection;

    /// The seeded shop and the user who owns it, both written by the first
    /// migration.
    pub(crate) const SHOP: i32 = 1;
    pub(crate) const OWNER: i32 = 1;

    /// The handle comes back first because a test that drops it keeps the
    /// connection open on a file that is no longer there.
    pub(crate) fn open() -> (tempfile::TempDir, SqliteConnection) {
        let dir = tempfile::tempdir().expect("a temp directory");
        let path = dir.path().join("t.db");
        let conn = crate::db::open(&path).expect("a migrated file");
        (dir, conn)
    }
}
