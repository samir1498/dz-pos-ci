//! The only modules that run diesel queries. Every query takes a `shop_id`.
pub mod audit;
pub mod categories;
pub mod counters;
pub mod documents;
pub mod products;
pub mod settings;
pub mod shops;
pub mod stock;
