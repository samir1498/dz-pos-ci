//! Domain core: models, repos, services. Every caller (Tauri, HTTP, tests)
//! goes through `services`; nothing outside this crate touches diesel.
pub mod build_info;
pub mod db;
pub mod error;
pub mod lang;
pub mod log;
pub mod models;
pub mod money;
pub mod print;
// Crate-internal on purpose: architecture.md says HTTP, Tauri and mobile
// call services, never diesel. The visibility is the enforcement.
pub(crate) mod repos;
pub mod schema;
pub mod services;
