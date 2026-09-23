//! The shop's own counts a restored backup's summary and the support
//! bundle's `counts.txt` both carry: how many products, documents and
//! customers a shop file holds.
//!
//! Kept here rather than in `dzpos_kernel::services::backup` or
//! `dzpos_kernel::services::support_bundle`: S4 of
//! `a-kernel-crate-and-retail-as-the-first-module` moved the counting itself
//! out of both, so `backup::verify` takes a caller-supplied closure and
//! `support_bundle::gather` takes a plain slice of already-read counts
//! instead of asking either module to name a table. Both kernel functions
//! keep their own generic `count`/`count_rows` helper (a table name is not a
//! shop concept); this module is the one caller that hands them
//! `"products"`, `"documents"` and `"customers"` by name.
//!
//! Outside `services/` for the same reason `audit_actions.rs` is: a file
//! inside that folder is read as a second service by the ring walk in
//! `crates/core/tests/services_go_through_services.rs`, and this is a plain
//! read with no repo of its own, not a service with siblings to reach past.

use diesel::sqlite::SqliteConnection;
use dzpos_kernel::db::Conn;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::{backup, support_bundle};
use serde::Serialize;

/// What a restored copy's summary carries. `documents` is `None` only for a
/// copy taken before the documents table existed (migration 2); every copy
/// since carries the count. `Serialize` is what lets
/// `backup::record_restore` write these two figures into its audit row
/// without that module ever naming either field itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BackupCounts {
    pub products: i64,
    pub documents: Option<i64>,
}

/// The closure `backup::verify` calls once a copy has opened, passed
/// SQLite's integrity check and carries no migration this build does not
/// know, to read what it holds.
pub fn for_verify(conn: &mut SqliteConnection) -> Result<BackupCounts, CoreError> {
    Ok(BackupCounts {
        products: backup::count(conn, "products")?.unwrap_or(0),
        documents: backup::count(conn, "documents")?,
    })
}

/// The counts `support_bundle`'s `counts.txt` carries, in the order it
/// prints them. Read by the API before it calls `support_bundle::gather`,
/// which takes them as a plain slice rather than reading a table by name
/// itself.
pub fn for_bundle(conn: &mut Conn) -> Result<[(&'static str, i64); 3], CoreError> {
    Ok([
        ("products", support_bundle::count_rows(conn, "products")?),
        ("documents", support_bundle::count_rows(conn, "documents")?),
        ("customers", support_bundle::count_rows(conn, "customers")?),
    ])
}
