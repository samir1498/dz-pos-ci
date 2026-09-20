use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::Json;
use dzpos_core::error::CoreError;
use dzpos_core::lang::Lang;
use dzpos_core::models::product::NewProduct;
use dzpos_core::print::{render_label, render_label_sheet};
use dzpos_core::services::permissions::{can, Permission};
use dzpos_core::services::products as service;
use serde::Deserialize;

use crate::dto::{LabelSheetDto, NewProductDto, ProductDto, LABEL_SHEET_MAX};
use crate::error::ApiError;
use crate::session::CurrentUser;
use crate::AppState;

/// Every answer in this file that carries a product goes through here,
/// writes included. A create or an update is behind `EditFiches`, which only
/// a manager or an owner holds today and both of those hold
/// `SeeCostAndMargin` as well, so redacting the write answers changes
/// nothing now. It is here so that the day somebody adds a role that may
/// edit a fiche without seeing what the shop paid, the fiche it hands back
/// does not quietly tell them (M4 closing review, 2026-09-11).
///
/// `GET /products` and `GET /products/{id}` carry the whole catalogue to
/// every signed-in role, cashier included, because the till needs it to
/// ring a sale up (`gates/` names no row for either read). What the till
/// does not need is what a product cost the shop, so this is the one field
/// a route in this file redacts itself rather than leaving to a row in the
/// gate table, which can only say yes or no to a whole route (M4 T5 review,
/// 2026-09-11).
fn redact_cost(mut dto: ProductDto, role: dzpos_core::models::sql_types::Role) -> ProductDto {
    if !can(role, Permission::SeeCostAndMargin) {
        dto.cost_centimes = None;
        dto.wholesale_centimes = None;
    }
    dto
}

pub async fn list(
    State(state): State<AppState>,
    who: CurrentUser,
) -> Result<Json<Vec<ProductDto>>, ApiError> {
    let shop = state.shop_id;
    let found = state.blocking(move |c| service::list(c, shop)).await?;
    Ok(Json(
        found
            .into_iter()
            .map(|p| redact_cost(ProductDto::from(p), who.role))
            .collect(),
    ))
}

pub async fn get_one(
    State(state): State<AppState>,
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
) -> Result<Json<ProductDto>, ApiError> {
    // `/products/abc` used to leave as axum's own text/plain 400, which the
    // client could only read as "unreachable". Same envelope as every
    // other refusal.
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let shop = state.shop_id;
    let found = state.blocking(move |c| service::get(c, shop, id)).await?;
    Ok(Json(redact_cost(ProductDto::from(found), who.role)))
}

pub async fn create(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<NewProductDto>, JsonRejection>,
) -> Result<(StatusCode, Json<ProductDto>), ApiError> {
    // A body that does not parse is the caller's mistake, not a server
    // fault, and it leaves in the same error shape as everything else.
    let Json(dto) = body.map_err(ApiError::from)?;
    let new = NewProduct::try_from(dto)?;
    let shop = state.shop_id;
    let user = who.id;
    let made = state
        .blocking(move |c| service::create(c, shop, user, new))
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(redact_cost(ProductDto::from(made), who.role)),
    ))
}

/// The whole product again, not a patch: the screen sends every field it
/// shows, so a field left out is a bug at the edge, not a value to keep.
/// `barcode` null keeps the number the product has (core, `update`).
pub async fn update(
    State(state): State<AppState>,
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<NewProductDto>, JsonRejection>,
) -> Result<Json<ProductDto>, ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let new = NewProduct::try_from(dto)?;
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| service::update(c, shop, user, id, new))
        .await?;
    Ok(Json(redact_cost(ProductDto::from(after), who.role)))
}

/// The language on the label, named by the caller on every call the way the
/// ticket's is.
#[derive(Deserialize)]
pub struct LabelQuery {
    lang: Lang,
}

/// The 58 x 40 mm shelf label for one product, as an HTML page.
///
/// The core renders it (features.md §4), so this handler reads the fiche and
/// hands the string over. A product with no EAN-13 leaves as the core's own
/// validation refusal: there is no honest picture of a code a scanner cannot
/// read, and a label with the digits and no bars gets stuck on a shelf and
/// scans as nothing.
pub async fn label(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    query: Result<Query<LabelQuery>, QueryRejection>,
) -> Result<Html<String>, ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let Query(LabelQuery { lang }) =
        query.map_err(|_| ApiError::BadRequest("lang must be fr, en or ar".into()))?;
    let shop = state.shop_id;
    let found = state.blocking(move |c| service::get(c, shop, id)).await?;
    Ok(Html(render_label(&found, lang)?))
}

/// A sheet of labels on A4 for the products the caller names, in the order
/// it named them.
///
/// Every id is read in one call and every one has to be this shop's: a
/// stranger's id answers 404 rather than being skipped, because a sheet
/// missing one label looks complete and the product left off it is exactly
/// the one somebody was looking for. Same reason the core refuses the whole
/// sheet when one product has no EAN-13.
pub async fn label_sheet(
    State(state): State<AppState>,
    query: Result<Query<LabelQuery>, QueryRejection>,
    body: Result<Json<LabelSheetDto>, JsonRejection>,
) -> Result<Html<String>, ApiError> {
    let Query(LabelQuery { lang }) =
        query.map_err(|_| ApiError::BadRequest("lang must be fr, en or ar".into()))?;
    let Json(LabelSheetDto { ids }) = body.map_err(ApiError::from)?;
    // Refused here rather than after the reads: the core would render the
    // page, but only after this handler had run one query per id, and a
    // body naming fifty thousand of them is a request nobody typed.
    if ids.len() > LABEL_SHEET_MAX {
        return Err(ApiError::Request(CoreError::validation(
            "ids",
            "more labels than one sheet is printed at a time",
        )));
    }
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| ids.iter().map(|id| service::get(c, shop, *id)).collect())
        .await?;
    let found: Vec<_> = found;
    Ok(Html(render_label_sheet(&found, lang)?))
}
