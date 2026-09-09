//! HTTP handlers. They translate, they do not decide: every rule lives in
//! `dzpos_core::services`.

pub mod categories;
pub mod products;

use axum::extract::State;
use axum::Json;

use crate::dto::HealthDto;
use crate::error::ApiError;
use crate::AppState;

pub async fn health(State(state): State<AppState>) -> Json<HealthDto> {
    Json(HealthDto {
        status: "ok".to_string(),
        shop_id: state.shop_id,
    })
}

pub async fn not_found() -> ApiError {
    ApiError::NoRoute
}
