use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use dzpos_core::models::product::NewProduct;
use dzpos_core::services::products as service;

use crate::dto::{NewProductDto, ProductDto};
use crate::error::ApiError;
use crate::AppState;

pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<ProductDto>>, ApiError> {
    let shop = state.shop_id;
    let found = state.blocking(move |c| service::list(c, shop)).await?;
    Ok(Json(found.into_iter().map(ProductDto::from).collect()))
}

pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ProductDto>, ApiError> {
    let shop = state.shop_id;
    let found = state.blocking(move |c| service::get(c, shop, id)).await?;
    Ok(Json(ProductDto::from(found)))
}

pub async fn create(
    State(state): State<AppState>,
    body: Result<Json<NewProductDto>, JsonRejection>,
) -> Result<(StatusCode, Json<ProductDto>), ApiError> {
    // A body that does not parse is the caller's mistake, not a server
    // fault, and it leaves in the same error shape as everything else.
    let Json(dto) = body.map_err(ApiError::from)?;
    let new = NewProduct::try_from(dto)?;
    let shop = state.shop_id;
    let made = state
        .blocking(move |c| service::create(c, shop, new))
        .await?;
    Ok((StatusCode::CREATED, Json(ProductDto::from(made))))
}
