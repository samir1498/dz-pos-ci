//! Business rules. Every caller (HTTP, Tauri, tests) enters here.
pub mod audit;
pub mod backup;
pub mod categories;
pub mod clock;
pub mod documents;
pub mod products;
pub mod sales;
pub mod settings;
pub mod shops;
pub mod stock;
