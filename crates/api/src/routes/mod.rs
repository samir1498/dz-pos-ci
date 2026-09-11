//! HTTP handlers. They translate, they do not decide: every rule lives in
//! `dzpos_core::services`.

pub mod auth;
pub mod backups;
pub mod categories;
pub mod customers;
pub mod dashboard;
pub mod expenses;
pub mod export;
pub mod import;
pub mod products;
pub mod purchases;
pub mod sales;
pub mod settings;
pub mod stock;
pub mod suppliers;

use axum::extract::State;
use axum::Json;

use crate::dto::{ClockDto, HealthDto};
use crate::error::ApiError;
use crate::AppState;

pub async fn health(State(state): State<AppState>) -> Json<HealthDto> {
    Json(HealthDto {
        status: "ok".to_string(),
        shop_id: state.shop_id,
    })
}

/// The shop's calendar. Read from the same `clock` the services date
/// documents with, so a screen and a stored row never disagree about which
/// day it is.
pub async fn clock() -> Json<ClockDto> {
    Json(ClockDto {
        today: dzpos_core::services::clock::now()
            .date()
            .format("%Y-%m-%d")
            .to_string(),
    })
}

pub async fn not_found() -> ApiError {
    ApiError::NoRoute
}

/// A known path with a method it does not take (`DELETE /products`).
/// axum's default answer is an empty 405 the client reads as unreachable.
pub async fn method_not_allowed() -> ApiError {
    ApiError::MethodNotAllowed
}
