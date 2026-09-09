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

/// A known path with a method it does not take (`DELETE /products`).
/// axum's default answer is an empty 405 the client reads as unreachable.
pub async fn method_not_allowed() -> ApiError {
    ApiError::MethodNotAllowed
}
