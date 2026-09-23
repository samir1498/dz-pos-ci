//! The first module
//! (`context/plans/20260922-a-kernel-crate-and-retail-as-the-first-module.md`):
//! sales, purchases, avoir, proforma, documents, cancellation, cash,
//! cash_refunds, shifts, stock, pricing, products, categories, customers,
//! debt, suppliers, supplier_debt, expenses, dashboard, export, import,
//! seed, and the print engine. Depends on `dzpos-kernel`.
// The domain-free half, re-exported by name so `crate::money::Money`,
// `crate::lang::Lang`, `crate::db::Conn` and `crate::schema::products` keep
// resolving unchanged: every retail file wrote these paths as a sibling
// module before the kernel crate split (S3 of
// `a-kernel-crate-and-retail-as-the-first-module`), and moving each one to
// `dzpos_kernel::x` would be a rewrite of every call site rather than a
// move of the source. `error` is this crate's own module since S4: eight of
// `CoreError`'s variants named a shop concept or a retail-only dependency,
// and `error.rs` re-exports `CoreError` itself so `crate::error::CoreError`
// still resolves.
pub use dzpos_kernel::{db, lang, money, schema};

// A plain constant list, not a service: see its own module doc for why it
// sits outside `services/` rather than inside as `services::audit_actions`.
pub mod audit_actions;
pub mod error;
pub mod models;
pub mod print;
// Crate-internal on purpose, same as `dzpos-core` before the split.
pub(crate) mod repos;
pub mod services;
// A plain read with no repo of its own, outside `services/` for the same
// reason `audit_actions` is: see its own module doc.
pub mod shop_counts;
