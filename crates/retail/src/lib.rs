//! The first module
//! (`context/plans/20260922-a-kernel-crate-and-retail-as-the-first-module.md`):
//! sales, purchases, avoir, proforma, documents, cancellation, cash,
//! cash_refunds, shifts, stock, pricing, products, categories, customers,
//! debt, suppliers, supplier_debt, expenses, dashboard, export, import,
//! seed, and the print engine. Depends on `dzpos-kernel`.
// The domain-free half, re-exported by name so `crate::error::CoreError`,
// `crate::money::Money`, `crate::lang::Lang`, `crate::db::Conn` and
// `crate::schema::products` keep resolving unchanged: every retail file
// wrote these paths as a sibling module before the kernel crate split
// (S3 of `a-kernel-crate-and-retail-as-the-first-module`), and moving each
// one to `dzpos_kernel::x` would be a rewrite of every call site rather
// than a move of the source.
pub use dzpos_kernel::{db, error, lang, money, schema};

pub mod models;
pub mod print;
// Crate-internal on purpose, same as `dzpos-core` before the split.
pub(crate) mod repos;
pub mod services;
