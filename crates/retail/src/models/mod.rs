//! Plain structs and their diesel rows. No business rules live here.
pub mod cash_refund;
pub mod category;
pub mod contenance;
pub mod customer;
pub mod debt;
pub mod document;
pub mod expense;
pub mod job;
pub mod product;
pub mod purchase;
pub mod sale_idempotency;
pub mod shift;
pub mod sql_types;
pub mod stock;
pub mod supplier;
pub mod supplier_debt;

// The five domain-free models stay in the kernel; re-exported here so a
// retail file's `crate::models::audit` (and the other four) resolves the
// way it did before the crate split. `sql_types` is this crate's own module
// since S4 of `a-kernel-crate-and-retail-as-the-first-module`: nine of its
// ten `text_enum!` invocations name a shop concept, and it re-exports
// `Role` itself rather than being re-exported whole.
pub use dzpos_kernel::models::{audit, pairing, session, shop, user};
