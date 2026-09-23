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
#[path = "../../tests/unit/repos_mod.rs"]
pub(crate) mod testdb;
