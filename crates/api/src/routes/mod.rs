//! HTTP handlers. They translate, they do not decide: every rule lives in
//! `dzpos_core::services`.

pub mod audit;
pub mod auth;
pub mod backups;
pub mod categories;
pub mod customers;
pub mod dashboard;
pub mod expenses;
pub mod export;
pub mod import;
pub mod pairing;
pub mod products;
pub mod purchases;
pub mod sales;
pub mod settings;
pub mod stock;
pub mod suppliers;
pub mod support;
pub mod users;

use axum::extract::State;
use axum::Json;

use crate::dto::{BuildInfoDto, ClockDto, HealthDto};
use crate::error::ApiError;
use crate::AppState;

pub async fn health(State(state): State<AppState>) -> Result<Json<HealthDto>, ApiError> {
    let shop = state.shop_id;
    let needs_first_setup = state
        .blocking(move |c| dzpos_core::services::users::shop_needs_first_setup(c, shop))
        .await?;
    Ok(Json(HealthDto {
        status: "ok".to_string(),
        shop_id: shop,
        needs_first_setup,
    }))
}

/// The About screen's one source (M5 T1): the version, the git short hash
/// and the build date `crates/core::build_info` baked in at compile time,
/// read back here rather than a second copy kept in `apps/desktop`.
pub async fn build_info() -> Json<BuildInfoDto> {
    Json(BuildInfoDto::from(dzpos_core::build_info::BUILD_INFO))
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
