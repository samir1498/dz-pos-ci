//! Plain structs and their diesel rows. No business rules live here.
pub mod cash_refund;
pub mod category;
pub mod customer;
pub mod debt;
pub mod document;
pub mod expense;
pub mod job;
pub mod product;
pub mod purchase;
pub mod sale_idempotency;
pub mod shift;
pub mod stock;
pub mod supplier;
pub mod supplier_debt;

// The six domain-free models stay in the kernel; re-exported here so a
// retail file's `crate::models::sql_types::Role` (and the other five)
// resolves the way it did before the crate split.
pub use dzpos_kernel::models::{audit, pairing, session, shop, sql_types, user};
