//! The only modules that run diesel queries. Every query takes a `shop_id`.
pub mod audit;
pub mod cash;
pub mod categories;
pub mod counters;
pub mod customers;
pub mod debt;
pub mod documents;
pub mod expenses;
pub mod jobs;
pub mod products;
pub mod purchases;
pub mod settings;
pub mod shops;
pub mod stock;
pub mod supplier_debt;
pub mod suppliers;

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
