//! The purchases screen's routes. They translate: every rule lives in
//! `dzpos_core::services::purchases`, and what a delivery does to the stock
//! and to the supplier's account is decided there.
//!
//! Nothing here deletes an order. An order nothing arrived against is
//! cancelled and one the rest of which will never come is closed short, and
//! both carry the reason into the audit log.
//!
//! Every route that changes an order answers the whole order again, lines and
//! deliveries included: the screen shows the new state without a second call,
//! and what it shows is what the core stored rather than what the form sent.

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use dzpos_core::services::purchases as service;
use serde::Deserialize;

use crate::dto::{
    CloseOrderDto, NewPurchaseDto, NewReceiptDto, PurchaseDetailDto, PurchaseDto, PurchaseStatusDto,
};
use crate::error::ApiError;
use crate::session::CurrentUser;
use crate::AppState;

/// The list's two filters. Both absent is the whole list, which is what a
/// screen opened with nothing chosen shows.
#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    status: Option<PurchaseStatusDto>,
    #[serde(default)]
    supplier_id: Option<i32>,
}

pub async fn list(
    State(state): State<AppState>,
    filters: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Vec<PurchaseDto>>, ApiError> {
    let Query(ListQuery {
        status,
        supplier_id,
    }) = filters.map_err(|_| {
        ApiError::BadRequest("status is one of the five states and supplier_id is a number".into())
    })?;
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| service::list(c, shop, status.map(Into::into), supplier_id))
        .await?;
    Ok(Json(found.into_iter().map(PurchaseDto::from).collect()))
}

/// One order with its lines and its deliveries.
pub async fn get_one(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
) -> Result<Json<PurchaseDetailDto>, ApiError> {
    let id = path_id(id)?;
    let shop = state.shop_id;
    let found = state.blocking(move |c| service::get(c, shop, id)).await?;
    Ok(Json(PurchaseDetailDto::from(found)))
}

/// An order, its lines, and when the goods came with the paper the whole
/// delivery too. One transaction in the core.
pub async fn create(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<NewPurchaseDto>, JsonRejection>,
) -> Result<(StatusCode, Json<PurchaseDetailDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let new = dto.into_core()?;
    let shop = state.shop_id;
    // Writing an order commits the shop's money, so this route needs a
    // permission of its own once roles land.
    let user = who.id;
    let made = state
        .blocking(move |c| service::save(c, shop, user, new))
        .await?;
    Ok((StatusCode::CREATED, Json(PurchaseDetailDto::from(made))))
}

/// A delivery against an order. The stock rises and the supplier's account
/// with it, at the cost the goods landed at (features.md §1).
pub async fn receive(
    State(state): State<AppState>,
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<NewReceiptDto>, JsonRejection>,
) -> Result<(StatusCode, Json<PurchaseDetailDto>), ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let lines = dto.lines.into_iter().map(Into::into).collect();
    let note = dto.note;
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| service::receive(c, shop, user, id, lines, note))
        .await?;
    Ok((StatusCode::CREATED, Json(PurchaseDetailDto::from(after))))
}

/// Goods handed back to the supplier. It writes no document: the stock
/// movement out and the credit on the ledger are the record.
pub async fn returns(
    State(state): State<AppState>,
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<NewReceiptDto>, JsonRejection>,
) -> Result<(StatusCode, Json<PurchaseDetailDto>), ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let lines = dto.lines.into_iter().map(Into::into).collect();
    let note = dto.note;
    let shop = state.shop_id;
    // A return lowers what the shop owes, so it needs the same permission a
    // correction to the ledger does.
    let user = who.id;
    let after = state
        .blocking(move |c| service::return_to_supplier(c, shop, user, id, lines, note))
        .await?;
    Ok((StatusCode::CREATED, Json(PurchaseDetailDto::from(after))))
}

/// An order that never happened. Only while nothing has arrived against it.
pub async fn cancel(
    State(state): State<AppState>,
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<CloseOrderDto>, JsonRejection>,
) -> Result<Json<PurchaseDetailDto>, ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| service::cancel(c, shop, user, id, dto.reason))
        .await?;
    Ok(Json(PurchaseDetailDto::from(after)))
}

/// An order the rest of which will never come. What arrived stays; the rest
/// is written off, with the reason in the log.
pub async fn close_short(
    State(state): State<AppState>,
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<CloseOrderDto>, JsonRejection>,
) -> Result<Json<PurchaseDetailDto>, ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let shop = state.shop_id;
    // Writing off goods that never came is a decision, and the log is what
    // carries the accountability until a permission does.
    let user = who.id;
    let after = state
        .blocking(move |c| service::close_short(c, shop, user, id, dto.reason))
        .await?;
    Ok(Json(PurchaseDetailDto::from(after)))
}

/// `/purchases/abc` leaves in the same envelope as every other refusal rather
/// than as axum's own text/plain 400.
fn path_id(id: Result<Path<i32>, PathRejection>) -> Result<i32, ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    Ok(id)
}
