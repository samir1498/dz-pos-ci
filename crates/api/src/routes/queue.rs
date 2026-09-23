//! The clinic's waiting queue routes (C4 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`).
//! They translate: every rule lives in `dzpos_core::services::queue`, and
//! the permissions are rows in `gates/table.rs` (the patient file's two,
//! reused), not checks here.
//!
//! Every write answers with the entry as it now stands, the patient's names
//! with it, so the screen redraws one line without reading the day again.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use dzpos_core::services::queue as service;

use crate::dto::{QueueAddDto, QueueEntryDto};
use crate::error::ApiError;
use crate::session::CurrentUser;
use crate::AppState;

/// Today's queue on the shop's clock, in arrival order.
pub async fn today(State(state): State<AppState>) -> Result<Json<Vec<QueueEntryDto>>, ApiError> {
    let shop = state.shop_id;
    let found = state.blocking(move |c| service::today(c, shop)).await?;
    Ok(Json(found.into_iter().map(QueueEntryDto::from).collect()))
}

pub async fn add(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<QueueAddDto>, JsonRejection>,
) -> Result<(StatusCode, Json<QueueEntryDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let shop = state.shop_id;
    let user = who.id;
    let made = state
        .blocking(move |c| service::add(c, shop, user, &dto.patient_id))
        .await?;
    Ok((StatusCode::CREATED, Json(QueueEntryDto::from(made))))
}

/// The earliest arrival not yet called. An empty line is a 409.
pub async fn call_next(
    State(state): State<AppState>,
    who: CurrentUser,
) -> Result<Json<QueueEntryDto>, ApiError> {
    let shop = state.shop_id;
    let user = who.id;
    let called = state
        .blocking(move |c| service::call_next(c, shop, user))
        .await?;
    Ok(Json(QueueEntryDto::from(called)))
}

/// One entry called in out of order.
pub async fn call(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<QueueEntryDto>, ApiError> {
    let shop = state.shop_id;
    let user = who.id;
    let called = state
        .blocking(move |c| service::call(c, shop, user, &id))
        .await?;
    Ok(Json(QueueEntryDto::from(called)))
}

pub async fn seen(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<QueueEntryDto>, ApiError> {
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| service::mark_seen(c, shop, user, &id))
        .await?;
    Ok(Json(QueueEntryDto::from(after)))
}

pub async fn left(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<QueueEntryDto>, ApiError> {
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| service::mark_left(c, shop, user, &id))
        .await?;
    Ok(Json(QueueEntryDto::from(after)))
}
