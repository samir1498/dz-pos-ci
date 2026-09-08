//! Domain core: models, repos, services. Every caller (Tauri, HTTP, tests)
//! goes through `services`; nothing outside this crate touches diesel.
pub mod db;
pub mod money;
