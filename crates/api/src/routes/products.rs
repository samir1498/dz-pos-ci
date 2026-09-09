use axum::extract::rejection::{JsonRejection, PathRejection};
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
    id: Result<Path<i32>, PathRejection>,
) -> Result<Json<ProductDto>, ApiError> {
    // `/products/abc` used to leave as axum's own text/plain 400, which the
    // client could only read as "unreachable". Same envelope as every
    // other refusal.
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
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
    let user = state.user_id;
    let made = state
        .blocking(move |c| service::create(c, shop, user, new))
        .await?;
    Ok((StatusCode::CREATED, Json(ProductDto::from(made))))
}

/// The whole product again, not a patch: the screen sends every field it
/// shows, so a field left out is a bug at the edge, not a value to keep.
/// `barcode` null keeps the number the product has (core, `update`).
pub async fn update(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<NewProductDto>, JsonRejection>,
) -> Result<Json<ProductDto>, ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let new = NewProduct::try_from(dto)?;
    let shop = state.shop_id;
    let user = state.user_id;
    let after = state
        .blocking(move |c| service::update(c, shop, user, id, new))
        .await?;
    Ok(Json(ProductDto::from(after)))
}
