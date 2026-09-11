//! The users screen's routes (M4 T8). They translate: every rule (the shape
//! of a PIN, the last owner, nobody switching their own fiche off) lives in
//! `dzpos_core::services::users` and is enforced there, on the row, not
//! restated here.
//!
//! `ManageUsers` gates every route in this file (`crate::gates`), so a
//! cashier or a manager is refused before a handler runs: a hidden button on
//! the screen is not the defence, the gate in `crate::session::require` is.
//!
//! Nothing here deletes a fiche. A user who has left the shop is switched
//! off, never removed: every document, ledger row and audit entry they wrote
//! still names them.

use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use dzpos_core::services::users::{self as service, NewUser};

use crate::dto::{NewUserDto, SetPinDto, UserDto};
use crate::error::ApiError;
use crate::session::CurrentUser;
use crate::AppState;

/// The shop's staff, active first then alphabetical: the same order the
/// service reads them in, and the owner's own read (`crate::gates`).
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<UserDto>>, ApiError> {
    let shop = state.shop_id;
    let found = state.blocking(move |c| service::list(c, shop)).await?;
    Ok(Json(found.into_iter().map(UserDto::from).collect()))
}

/// A fiche, name and role. No credential: `POST /users/{id}/pin` is what
/// gives it a PIN, the first one or a reset alike
/// (`services::users::create`'s own doc, and `crate::dto::NewUserDto`'s).
pub async fn create(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<NewUserDto>, JsonRejection>,
) -> Result<(StatusCode, Json<UserDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let shop = state.shop_id;
    let actor = who.id;
    let made = state
        .blocking(move |c| {
            service::create(
                c,
                shop,
                actor,
                NewUser {
                    name: dto.name,
                    role: dto.role.into(),
                },
            )
        })
        .await?;
    Ok((StatusCode::CREATED, Json(UserDto::from(made))))
}

/// Gives a fiche its first PIN or resets a forgotten one; the service does
/// not tell the two apart and neither does this route. Never answers the
/// PIN it replaces, because there is not one to show
/// (`services::users::set_pin`'s own doc).
pub async fn set_pin(
    State(state): State<AppState>,
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<SetPinDto>, JsonRejection>,
) -> Result<Json<UserDto>, ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let shop = state.shop_id;
    let actor = who.id;
    let acting_session_id = who.session_id;
    let after = state
        .blocking(move |c| service::set_pin(c, shop, actor, id, &dto.pin, Some(acting_session_id)))
        .await?;
    Ok(Json(UserDto::from(after)))
}

/// Switches a fiche off. The last-owner and self refusals are
/// `services::users::deactivate`'s, enforced on the row: this route names no
/// rule of its own.
pub async fn deactivate(
    State(state): State<AppState>,
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
) -> Result<Json<UserDto>, ApiError> {
    let id = path_id(id)?;
    let shop = state.shop_id;
    let actor = who.id;
    let after = state
        .blocking(move |c| service::deactivate(c, shop, actor, id))
        .await?;
    Ok(Json(UserDto::from(after)))
}

/// Switches a fiche back on. Nothing was destroyed, so nothing is rebuilt.
pub async fn reactivate(
    State(state): State<AppState>,
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
) -> Result<Json<UserDto>, ApiError> {
    let id = path_id(id)?;
    let shop = state.shop_id;
    let actor = who.id;
    let after = state
        .blocking(move |c| service::reactivate(c, shop, actor, id))
        .await?;
    Ok(Json(UserDto::from(after)))
}

/// `/users/abc` leaves in the same envelope as every other refusal rather
/// than as axum's own text/plain 400.
fn path_id(id: Result<Path<i32>, PathRejection>) -> Result<i32, ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    Ok(id)
}
