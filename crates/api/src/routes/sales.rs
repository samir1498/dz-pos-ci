//! The till's routes. They translate: every rule lives in
//! `dzpos_core::services::sales`.

use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use dzpos_core::services::documents::DocumentKind;
use dzpos_core::services::sales::NewSale;
use dzpos_core::services::{documents, sales};

use crate::dto::{NewSaleDto, SaleDto};
use crate::error::ApiError;
use crate::AppState;

/// Newest first. M1 issues tickets only, so the list is the ticket series;
/// the kind is named here rather than left open so a facture added in M2
/// does not silently appear in the till's receipt view.
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<SaleDto>>, ApiError> {
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| documents::list(c, shop, Some(DocumentKind::Ticket)))
        .await?;
    Ok(Json(found.into_iter().map(SaleDto::from).collect()))
}

pub async fn get_one(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
) -> Result<Json<SaleDto>, ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let shop = state.shop_id;
    let found = state.blocking(move |c| documents::get(c, shop, id)).await?;
    Ok(Json(SaleDto::from(found)))
}

pub async fn create(
    State(state): State<AppState>,
    body: Result<Json<NewSaleDto>, JsonRejection>,
) -> Result<(StatusCode, Json<SaleDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let new = NewSale::try_from(dto)?;
    let shop = state.shop_id;
    // TODO(M4): the user comes from the request identity, not from the state.
    let user = state.user_id;
    let made = state
        .blocking(move |c| sales::issue(c, shop, user, new))
        .await?;
    Ok((StatusCode::CREATED, Json(SaleDto::from(made))))
}
