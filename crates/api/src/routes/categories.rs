use axum::extract::State;
use axum::Json;
use dzpos_core::services::categories as service;

use crate::dto::CategoryDto;
use crate::error::ApiError;
use crate::AppState;

pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<CategoryDto>>, ApiError> {
    let shop = state.shop_id;
    let found = state.blocking(move |c| service::list(c, shop)).await?;
    Ok(Json(found.into_iter().map(CategoryDto::from).collect()))
}
