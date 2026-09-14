//! The till's routes. They translate: every rule lives in
//! `dzpos_core::services::sales`.

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::Json;
use dzpos_core::error::CoreError;
use dzpos_core::lang::Lang;
use dzpos_core::print::{
    render_facture_with, render_ticket, render_ticket_escpos, Cancellation, FactureInput, Paper,
};
use dzpos_core::services::documents::DocumentKind;
use dzpos_core::services::sales::{NewSale, SaleKind};
use dzpos_core::services::{avoir, documents, sales};
use serde::Deserialize;

use crate::dto::{CancelDocumentDto, NewAvoirDto, NewSaleDto, SaleDto, SaleKindDto};
use crate::error::ApiError;
use crate::session::CurrentUser;
use crate::AppState;

/// Which paper the caller wants listed, if only one of them.
#[derive(Deserialize)]
pub struct ListQuery {
    kind: Option<SaleKindDto>,
}

/// Newest first, every kind the till issues unless the caller narrows it.
///
/// A facture is reachable here the moment its print panel is closed, which
/// is the whole reason the filter is optional rather than fixed: the day's
/// till roll asks for `ticket`, the documents screen asks for
/// `facture`, and a screen that wants both asks for neither. A kind this
/// route does not know is refused rather than read as no filter: a caller
/// asking for `avoir` and being handed everything would be showing the
/// wrong list with nothing to tell it apart from the right one.
pub async fn list(
    State(state): State<AppState>,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Vec<SaleDto>>, ApiError> {
    let Query(ListQuery { kind }) = query
        .map_err(|_| ApiError::BadRequest("kind must be ticket, facture or proforma".into()))?;
    let kind = kind.map(|k| SaleKind::from(k).document_kind());
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| documents::list(c, shop, kind))
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
    // The document and what cancelling it would do, in one call on the pool:
    // the screen showing the document is the screen that asks, and the two
    // reads have to be answers about the same file.
    let (found, effect) = state
        .blocking(move |c| {
            let found = documents::get(c, shop, id)?;
            let effect = documents::cancel_effect(c, shop, id)?;
            Ok((found, effect))
        })
        .await?;
    Ok(Json(SaleDto {
        cancel_effect: Some(effect.into()),
        ..SaleDto::from(found)
    }))
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
/// unreadable request. The id has to name a ticket: a facture squeezed onto
/// a till slip is a facture nobody would take for one, so its id answers 404
/// here, the same way a ticket's does on the facture route.
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
    let found = state
        .blocking(move |c| documents::get_of_kind(c, shop, id, DocumentKind::Ticket))
        .await?;
    Ok(Html(render_ticket(&found, lang)?))
}

/// The same ticket as ESC/POS bytes for a thermal printer.
///
/// Same document, same `lang`, same refusal as the HTML route when the
/// row is not a ticket or carries an IFU TVA recap. Returns the raw bytes
/// the head eats, so the desktop can write them to a file, a USB-serial
/// device or a TCP printer on port 9100. The transport stays out of the
/// API: the caller here decides whether those bytes go to a spool file or
/// over the wire, and `crates/core/src/print/escpos.rs` is what that caller
/// calls next.
pub async fn ticket_escpos(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    lang: Result<Query<TicketQuery>, QueryRejection>,
) -> Result<axum::response::Response, ApiError> {
    use axum::http::{header, HeaderValue};
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let Query(TicketQuery { lang }) =
        lang.map_err(|_| ApiError::BadRequest("lang must be fr, en or ar".into()))?;
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| documents::get_of_kind(c, shop, id, DocumentKind::Ticket))
        .await?;
    let bytes = render_ticket_escpos(&found, lang)?;
    let mut res = axum::response::Response::new(axum::body::Body::from(bytes));
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    res.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"ticket.bin\""),
    );
    Ok(res)
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

/// The A4 or A5 sheet for a stored facture, avoir or proforma, as an HTML
/// page.
///
/// Three kinds and not one: an avoir and a proforma are the same sheet with
/// a different title, a different number and one line saying which. A ticket
/// is its own paper and its own series, so its id still answers 404 rather
/// than a page titled FACTURE (services::documents).
///
/// What the template needs and the row does not carry is filled here,
/// because this is the layer that can read a second document. The facture an
/// avoir names is an id on the row and a number and a day on the paper, so
/// that document is read and handed over; the day and the reason a
/// cancellation was taken come off the annulled document's own block. A
/// cancelled facture with no block still prints as cancelled, because the
/// status is on the row; it just cannot say when or why.
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
    let (found, referenced) = state
        .blocking(move |c| {
            let found = documents::get(c, shop, id)?;
            if !matches!(
                found.kind,
                DocumentKind::Facture | DocumentKind::Avoir | DocumentKind::Proforma
            ) {
                return Err(CoreError::NotFound {
                    entity: DocumentKind::Facture.as_str(),
                    id,
                });
            }
            // Read in the same call, so the page and the document it names
            // come out of one look at the file.
            let referenced = match found.ref_document_id {
                Some(ref_id) => Some(documents::get(c, shop, ref_id)?),
                None => None,
            };
            Ok((found, referenced))
        })
        .await?;
    let cancellation = found.cancellation.as_ref().map(|c| Cancellation {
        at: c.at,
        reason: &c.reason,
    });
    Ok(Html(render_facture_with(
        &found,
        &FactureInput {
            referenced: referenced.as_ref(),
            cancellation,
        },
        lang,
        paper,
    )?))
}

pub async fn create(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<NewSaleDto>, JsonRejection>,
) -> Result<(StatusCode, Json<SaleDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let new = NewSale::try_from(dto)?;
    let shop = state.shop_id;
    let user = who.id;
    // The warning the core answered with travels on this one answer only:
    // it is about the moment the sale was rung up, and a later read of the
    // same document carries none.
    let made = state
        .blocking(move |c| sales::issue(c, shop, user, new))
        .await?;
    Ok((StatusCode::CREATED, Json(SaleDto::from(made))))
}

/// Writes a credit note against the facture in the path (features.md §3).
///
/// The body names the lines coming back, or nothing at all for the whole of
/// what is left on the facture. Every rule is the core's: what a line has left
/// to credit, the running total against what the facture asked for, the stock
/// coming back and the ledger movement with what it settled.
pub async fn avoir(
    State(state): State<AppState>,
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<NewAvoirDto>, JsonRejection>,
) -> Result<(StatusCode, Json<SaleDto>), ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let lines = dto.lines();
    let reason = dto.reason;
    let shop = state.shop_id;
    let user = who.id;
    let made = state
        .blocking(move |c| avoir::issue(c, shop, user, id, lines, reason, None))
        .await?;
    Ok((StatusCode::CREATED, Json(SaleDto::from(made))))
}

/// Every avoir written against one facture, oldest first. A ticket's id or a
/// document of another shop answers 404 rather than an empty list: an empty
/// list would read as "this facture has no credit notes".
pub async fn avoirs(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
) -> Result<Json<Vec<SaleDto>>, ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| avoir::list_for(c, shop, id))
        .await?;
    Ok(Json(found.into_iter().map(SaleDto::from).collect()))
}

/// Annuls the document in the path. It keeps its number and its row; what it
/// stops doing is asking for its amount and holding the goods off the shelf.
/// A facture that put money on an account is undone through an avoir the core
/// writes in the same transaction, and the answer carries the block naming it.
pub async fn cancel(
    State(state): State<AppState>,
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<CancelDocumentDto>, JsonRejection>,
) -> Result<Json<SaleDto>, ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let Json(CancelDocumentDto { reason }) = body.map_err(ApiError::from)?;
    let shop = state.shop_id;
    let user = who.id;
    let done = state
        .blocking(move |c| documents::cancel(c, shop, user, id, reason, None))
        .await?;
    Ok(Json(SaleDto::from(done)))
}
