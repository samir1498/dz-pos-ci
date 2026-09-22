//! The only modules that run diesel queries. Every query takes a `shop_id`.
//! The seven domain-free repos are in `dzpos_kernel::repos` since the
//! kernel crate split (S3 of `a-kernel-crate-and-retail-as-the-first-module`).
pub mod cash;
pub mod cash_refunds;
pub mod categories;
pub mod counters;
pub mod customers;
pub mod dashboard;
pub mod debt;
pub mod documents;
pub mod expenses;
pub mod jobs;
pub mod products;
pub mod purchases;
pub mod sale_idempotency;
pub mod shifts;
pub mod stock;
pub mod supplier_debt;
pub mod suppliers;

/// What was typed into a search box, as a LIKE pattern that matches it
/// anywhere: the escape character first, so escaping it does not escape the
/// escapes. A `%` typed by accident is a character to match and not the whole
/// list, and the customer list and the supplier list read a box the same way
/// rather than each escaping on its own.
pub(crate) fn contains_pattern(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

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
    /// connection open on a file that is no longer there. `dzpos_kernel::db`
    /// since the kernel crate split: `db.rs` and the migrations it embeds
    /// stayed in the kernel.
    pub(crate) fn open() -> (tempfile::TempDir, SqliteConnection) {
        let dir = tempfile::tempdir().expect("a temp directory");
        let path = dir.path().join("t.db");
        let conn = dzpos_kernel::db::open(&path).expect("a migrated file");
        (dir, conn)
    }
}
