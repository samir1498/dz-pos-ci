//! Plain structs and their diesel rows. No business rules live here.
//! The domain-free six: the rest moved to `dzpos-retail` in the kernel
//! crate split (S3 of `a-kernel-crate-and-retail-as-the-first-module`).
pub mod audit;
pub mod pairing;
pub mod session;
pub mod shop;
pub mod sql_types;
pub mod user;
