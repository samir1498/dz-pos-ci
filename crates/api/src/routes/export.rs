//! The four workbooks leaving over HTTP. Every rule lives in
//! `dzpos_core::services::export`; this file names the file and sets the
//! two headers a browser needs to save it.
//!
//! The name is built here rather than in the core because it carries the
//! shop's day, and the day is the API's clock (`routes::settings::now`), not
//! the service's. A workbook downloaded at half past midnight is filed under
//! the shop's date the way every other paper is.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use chrono::NaiveDate;
use dzpos_core::lang::Lang;
use dzpos_core::services::export::{self, DayRange};
use serde::Deserialize;

use crate::error::ApiError;
use crate::routes::settings::now;
use crate::session::CurrentUser;
use crate::AppState;

/// The one media type every spreadsheet reads an .xlsx as. Long, and the
/// exact string: a workbook served as `application/octet-stream` opens in a
/// text editor on some machines.
const XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

/// The language the sheet is named in. Asked on every call for the reason
/// the ticket's is: the caller knows who is looking at the screen, and the
/// server is not going to guess (features.md §4).
#[derive(Deserialize)]
pub struct ExportQuery {
    lang: Lang,
    /// Only the sales workbook reads these. Left on the shared struct so
    /// `?from=&to=` on a products export is an ignored extra rather than a
    /// 422 a caller cannot act on.
    #[serde(default)]
    from: Option<NaiveDate>,
    #[serde(default)]
    to: Option<NaiveDate>,
}

/// The workbook with its two headers. `Content-Disposition` is what makes a
/// browser save rather than render, and the CORS layer exposes it so the
/// desktop can read the name off the answer instead of inventing one.
pub(crate) fn workbook(stem: &str, day: NaiveDate, bytes: Vec<u8>) -> Response {
    let filename = format!("{stem}-{}.xlsx", day.format("%Y-%m-%d"));
    (
        [
            (CONTENT_TYPE, XLSX.to_owned()),
            (
                CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response()
}

fn query(q: Result<Query<ExportQuery>, QueryRejection>) -> Result<ExportQuery, ApiError> {
    let Query(q) = q.map_err(|_| {
        ApiError::BadRequest("lang must be fr, en or ar, and from and to dates".into())
    })?;
    Ok(q)
}

pub async fn products(
    State(state): State<AppState>,
    who: CurrentUser,
    q: Result<Query<ExportQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let lang = query(q)?.lang;
    let shop = state.shop_id;
    let actor = who.id;
    let bytes = state
        .blocking(move |c| export::products(c, shop, actor, lang))
        .await?;
    Ok(workbook("produits", now().date(), bytes))
}

/// The documents over a range, one row per line. An open end is open: no
/// `from` is everything the shop has ever written, and a comptable asking
/// for the year says so with two dates.
pub async fn sales(
    State(state): State<AppState>,
    who: CurrentUser,
    q: Result<Query<ExportQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let ExportQuery { lang, from, to } = query(q)?;
    let range = DayRange { from, to };
    let shop = state.shop_id;
    let actor = who.id;
    let bytes = state
        .blocking(move |c| export::sales(c, shop, actor, lang, range))
        .await?;
    Ok(workbook("ventes", now().date(), bytes))
}

pub async fn customers(
    State(state): State<AppState>,
    who: CurrentUser,
    q: Result<Query<ExportQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let lang = query(q)?.lang;
    let shop = state.shop_id;
    let actor = who.id;
    let bytes = state
        .blocking(move |c| export::customers(c, shop, actor, lang))
        .await?;
    Ok(workbook("clients", now().date(), bytes))
}

pub async fn suppliers(
    State(state): State<AppState>,
    who: CurrentUser,
    q: Result<Query<ExportQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let lang = query(q)?.lang;
    let shop = state.shop_id;
    let actor = who.id;
    let bytes = state
        .blocking(move |c| export::suppliers(c, shop, actor, lang))
        .await?;
    Ok(workbook("fournisseurs", now().date(), bytes))
}
