//! The till's routes. They translate: every rule lives in
//! `dzpos_core::services::sales`.

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::Json;
use dzpos_core::lang::Lang;
use dzpos_core::print::render_ticket;
use dzpos_core::services::documents::DocumentKind;
use dzpos_core::services::sales::NewSale;
use dzpos_core::services::{documents, sales};
use serde::Deserialize;

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

/// The language the ticket prints in. Named by the caller on every call
/// rather than read from a setting: the ruling in `docs/features.md` §4 is
/// that a document prints in the language the till is being used in, and
/// the till is the only place that knows which that is.
#[derive(Deserialize)]
pub struct TicketQuery {
    lang: Lang,
}

/// The 80 mm ticket for a stored sale, as an HTML page.
///
/// The core renders it (features.md §4: the same bytes from the desktop and
/// from a server with no screen), so this handler reads the document and
/// hands the string over. A lang the app does not print, or none at all, is
/// the caller's mistake and answers 422 in the envelope like every other
/// unreadable request.
pub async fn ticket(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    lang: Result<Query<TicketQuery>, QueryRejection>,
) -> Result<Html<String>, ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let Query(TicketQuery { lang }) =
        lang.map_err(|_| ApiError::BadRequest("lang must be fr, en or ar".into()))?;
    let shop = state.shop_id;
    let found = state.blocking(move |c| documents::get(c, shop, id)).await?;
    Ok(Html(render_ticket(&found, lang)?))
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
