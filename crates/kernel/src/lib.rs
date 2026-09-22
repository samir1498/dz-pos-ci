//! The domain-free half of the split
//! (`context/plans/20260922-a-kernel-crate-and-retail-as-the-first-module.md`):
//! users, sessions, permissions, audit, settings, preferences, pairing,
//! backup, support_bundle, clock, shops, the money module, the error enum,
//! `Role` (and the rest of `models::sql_types`), and the two print files
//! every trade shares. Depends on nothing else in the workspace.
pub mod build_info;
pub mod db;
pub mod error;
pub mod lang;
pub mod log;
pub mod models;
pub mod money;
pub mod print;
// Crate-internal on purpose, same as `dzpos-core` before the split: a
// caller goes through `services`, never diesel. `dzpos-retail` cannot name
// a kernel repo either way, because this line makes it a compile error.
pub(crate) mod repos;
pub mod schema;
pub mod services;
