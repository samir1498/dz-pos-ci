//! The clinic's patient file routes (C3 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`).
//! They translate: every rule lives in `dzpos_core::services::patients`, and
//! the two permissions are rows in `gates/table.rs`, not checks here.
//!
//! Nothing here deletes a patient. The queue and the book will point at the
//! file, so a patient who no longer comes is archived: out of the search,
//! still readable by id.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use dzpos_core::services::patients::{self as service, NewPatient};
use serde::Deserialize;

use crate::dto::{PatientDto, PatientWriteDto};
use crate::error::ApiError;
use crate::session::CurrentUser;
use crate::AppState;

/// The search box, as it reaches the API. Blank is no filter. `archived`
/// brings the archived files back into the answer beside the live ones.
#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    archived: bool,
}

pub async fn list(
    State(state): State<AppState>,
    search: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Vec<PatientDto>>, ApiError> {
    let Query(ListQuery { q, archived }) = search.map_err(|_| {
        ApiError::BadRequest("q must be a piece of text and archived true or false".into())
    })?;
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| service::search(c, shop, q.as_deref(), archived))
        .await?;
    Ok(Json(found.into_iter().map(PatientDto::from).collect()))
}

/// A patient's id is the UUID's text, so the path segment is taken as it
/// is: an id nobody made, in this shop or at all, is the service's 404.
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PatientDto>, ApiError> {
    let shop = state.shop_id;
    let found = state.blocking(move |c| service::get(c, shop, &id)).await?;
    Ok(Json(PatientDto::from(found)))
}

pub async fn create(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<PatientWriteDto>, JsonRejection>,
) -> Result<(StatusCode, Json<PatientDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let fields = NewPatient::try_from(dto)?;
    let shop = state.shop_id;
    let user = who.id;
    let made = state
        .blocking(move |c| service::create(c, shop, user, fields))
        .await?;
    Ok((StatusCode::CREATED, Json(PatientDto::from(made))))
}

/// The whole file again, not a patch: a field left out is a bug at the
/// edge and a null clears the column.
pub async fn update(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<String>,
    body: Result<Json<PatientWriteDto>, JsonRejection>,
) -> Result<Json<PatientDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let fields = NewPatient::try_from(dto)?;
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| service::update(c, shop, user, &id, fields))
        .await?;
    Ok(Json(PatientDto::from(after)))
}

/// Takes the file out of the search. A second archive is a 409.
pub async fn archive(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<PatientDto>, ApiError> {
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| service::archive(c, shop, user, &id))
        .await?;
    Ok(Json(PatientDto::from(after)))
}
