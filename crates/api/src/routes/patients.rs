//! The clinic's patient file routes (C3 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`).
//! They translate: every rule lives in `dzpos_core::services::patients`, and
//! the two permissions are rows in `gates/table.rs`, not checks here.
//!
//! Nothing here deletes a patient. The queue and the book will point at the
//! file, so a patient who no longer comes is archived: out of the search,
//! still readable by id.
//!
//! Notes are a field-level rule on top of `ViewPatients`/`EditPatients`
//! (C3b), the way `products.rs::redact_cost` is a field-level rule on top
//! of a product being readable by every role: every handler here asks
//! `service::may_see_notes(who.role)`, a plain `bool` the service already
//! decided, never the permission table itself, so this file stays outside
//! the two the table's own walk (`one_handler_decides.rs`) allows to ask.

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
    who: CurrentUser,
    search: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Vec<PatientDto>>, ApiError> {
    let Query(ListQuery { q, archived }) = search.map_err(|_| {
        ApiError::BadRequest("q must be a piece of text and archived true or false".into())
    })?;
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| service::search(c, shop, q.as_deref(), archived))
        .await?;
    let sees_notes = service::may_see_notes(who.role);
    Ok(Json(
        found
            .into_iter()
            .map(|p| PatientDto::from_patient(p, sees_notes))
            .collect(),
    ))
}

/// A patient's id is the UUID's text, so the path segment is taken as it
/// is: an id nobody made, in this shop or at all, is the service's 404.
pub async fn get_one(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<PatientDto>, ApiError> {
    let shop = state.shop_id;
    let found = state.blocking(move |c| service::get(c, shop, &id)).await?;
    Ok(Json(PatientDto::from_patient(
        found,
        service::may_see_notes(who.role),
    )))
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
    let role = who.role;
    let made = state
        .blocking(move |c| service::create(c, shop, user, fields, role))
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(PatientDto::from_patient(made, service::may_see_notes(role))),
    ))
}

/// The whole file again, not a patch: a field left out is a bug at the
/// edge and a null clears the column — except `notes`, which a caller
/// without `ViewPatientNotes` cannot clear by leaving it out (C3b:
/// `services::patients::update` keeps it as it was for that caller).
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
    let role = who.role;
    let after = state
        .blocking(move |c| service::update(c, shop, user, &id, fields, role))
        .await?;
    Ok(Json(PatientDto::from_patient(
        after,
        service::may_see_notes(role),
    )))
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
    Ok(Json(PatientDto::from_patient(
        after,
        service::may_see_notes(who.role),
    )))
}
