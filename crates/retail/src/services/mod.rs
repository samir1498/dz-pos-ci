//! Business rules for the retail trade. Every caller (HTTP, Tauri, tests)
//! enters here. The eleven domain-free services, and the helpers this
//! module used to hold (`role_of`, `optional_field`, `bounded_field`,
//! `user_names`, `end_sessions_of`), are in `dzpos_kernel::services` since
//! the kernel crate split (S3 of
//! `a-kernel-crate-and-retail-as-the-first-module`).
pub mod avoir;
pub mod avoir_remaining;
pub mod avoir_slice;
pub mod cancellation;
pub mod cash;
pub mod cash_refunds;
pub mod categories;
pub mod customers;
pub mod dashboard;
pub mod debt;
pub mod discount_threshold;
pub mod documents;
pub mod expenses;
pub mod export;
pub mod import;
pub mod pricing;
pub mod products;
pub mod proforma;
pub mod purchases;
pub mod sales;
pub mod seed;
pub mod shifts;
pub mod stock;
pub mod supplier_debt;
pub mod suppliers;
