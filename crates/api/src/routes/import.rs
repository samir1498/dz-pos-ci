//! The product import over HTTP: the template on the way out, the filled
//! file on the way in. Every rule lives in `dzpos_core::services::import`;
//! this file carries bytes.
//!
//! The file arrives as a raw body rather than as a multipart form. One file
//! and no fields beside it is not a form, and multipart would add a parser,
//! a boundary and a second way for a request to be malformed for no answer
//! the screen could give differently. The desktop posts the `File` object it
//! read from the picker; `content-type` is not read, because a shop's
//! machine names an .xlsx four different ways and the workbook says what it
//! is in its own first four bytes.

use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::response::Response;
use axum::Json;
use dzpos_core::lang::Lang;
use dzpos_core::services::import;
use serde::Deserialize;

use crate::dto::{ImportAppliedDto, ImportDryRunDto};
use crate::error::ApiError;
use crate::routes::export::workbook;
use crate::routes::settings::now;
use crate::session::CurrentUser;
use crate::AppState;

/// A filled catalogue is a few hundred kilobytes; axum's default cap is two
/// megabytes and the router raises these two routes to eight. A shop with
/// twenty thousand products has a file around one, and the headroom costs
/// nothing because the body is read once and handed to the parser.
pub const IMPORT_BODY_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Deserialize)]
pub struct TemplateQuery {
    lang: Lang,
}

/// The empty workbook a shop fills in: the columns, one example row, and a
/// second sheet naming the units and the rates a row may hold.
pub async fn template(
    q: Result<Query<TemplateQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let Query(TemplateQuery { lang }) =
        q.map_err(|_| ApiError::BadRequest("lang must be fr, en or ar".into()))?;
    let bytes = tokio::task::spawn_blocking(move || import::template(lang))
        .await
        .map_err(|_| ApiError::Unavailable)??;
    Ok(workbook("modele-produits", now().date(), bytes))
}

/// What the file would do, row by row, with nothing written.
///
/// A file with refusals answers 200 and not an error: the refusals are the
/// answer the screen asked for, and a table of them is what the shop reads
/// before it edits the spreadsheet. Only a file that is not a workbook at
/// all leaves as a 422.
pub async fn dry_run(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<ImportDryRunDto>, ApiError> {
    let shop = state.shop_id;
    let report = state
        .blocking(move |c| import::dry_run(c, shop, &body))
        .await?;
    Ok(Json(ImportDryRunDto::from(report)))
}

/// The file, written, or nothing at all.
///
/// The core runs the dry run again inside its transaction, so a file the
/// screen saw as clean and that has since collided with a product somebody
/// else created is refused here rather than half applied.
pub async fn apply(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Bytes,
) -> Result<Json<ImportAppliedDto>, ApiError> {
    let shop = state.shop_id;
    let user = who.id;
    let done = state
        .blocking(move |c| import::apply(c, shop, user, &body))
        .await?;
    Ok(Json(ImportAppliedDto::from(done)))
}
