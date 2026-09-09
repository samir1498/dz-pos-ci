//! The till's routes. They translate: every rule lives in
//! `dzpos_core::services::sales`.

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::Json;
use dzpos_core::lang::Lang;
use dzpos_core::print::{render_facture, render_ticket, Paper};
use dzpos_core::services::documents::DocumentKind;
use dzpos_core::services::sales::NewSale;
use dzpos_core::services::{documents, sales};
use serde::Deserialize;

use crate::dto::{NewSaleDto, SaleDto};
use crate::error::ApiError;
use crate::AppState;

/// Newest first, the ticket series only. The kind is named rather than left
/// open on purpose: the till's receipt view is about the day's till roll,
/// and a facture is filed and reprinted from the documents screen T9 adds.
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

/// The language and the sheet, both named by the caller on every call. The
/// sheet is not a setting either: the same facture goes on A4 in the office
/// and on A5 at the counter, and the till is the only place that knows which
/// the cashier reached for (features.md §4).
#[derive(Deserialize)]
pub struct FactureQuery {
    lang: Lang,
    paper: Paper,
}

/// The A4 or A5 facture for a stored sale, as an HTML page.
///
/// The id has to name a facture. A ticket is its own paper and its own
/// series, so the kind is part of what is being asked for and a ticket's id
/// answers 404 rather than a page titled FACTURE (services::documents).
pub async fn facture(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    query: Result<Query<FactureQuery>, QueryRejection>,
) -> Result<Html<String>, ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let Query(FactureQuery { lang, paper }) = query
        .map_err(|_| ApiError::BadRequest("lang must be fr, en or ar and paper a4 or a5".into()))?;
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| documents::get_of_kind(c, shop, id, DocumentKind::Facture))
        .await?;
    Ok(Html(render_facture(&found, lang, paper)?))
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
    // The warning the core answered with travels on this one answer only:
    // it is about the moment the sale was rung up, and a later read of the
    // same document carries none.
    let made = state
        .blocking(move |c| sales::issue(c, shop, user, new))
        .await?;
    Ok((StatusCode::CREATED, Json(SaleDto::from(made))))
}
