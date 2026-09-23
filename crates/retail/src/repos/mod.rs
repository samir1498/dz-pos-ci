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
#[path = "../../tests/unit/repos_mod.rs"]
pub(crate) mod testdb;
