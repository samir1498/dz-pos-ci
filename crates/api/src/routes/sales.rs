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
    render_facture_escpos, render_facture_with, render_ticket, render_ticket_escpos_in,
    Cancellation, FactureInput, FactureLayout, Page, Paper,
};
use dzpos_core::services::documents::DocumentKind;
use dzpos_core::services::sales::{NewSale, SaleKind};
use dzpos_core::services::{avoir, cancellation, documents, preferences, sales};
use serde::Deserialize;

use crate::dto::{CancelDocumentDto, NewAvoirDto, NewSaleDto, RefundDto, SaleDto, SaleKindDto};
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
        .blocking(move |c| -> Result<_, CoreError> {
            let found = documents::get(c, shop, id)?;
            let effect = cancellation::cancel_effect(c, shop, id)?;
            Ok((found, effect))
        })
        .await?;
    Ok(Json(SaleDto {
        cancel_effect: Some(effect.into()),
        ..SaleDto::from(found)
    }))
}

/// `lang` is the language the till is being used in, named by the caller on
/// every call because the till is the only place that knows which that is.
/// It is not what decides the paper on its own: `print_lang`, left out of
/// most calls, overrides for the one call the way `?layout=` overrides the
/// stored facture layout, and between the two sits the shop's own stored
/// preference. `preferences::print_lang_for` (crates/core) is the one place
/// that resolves the three steps
/// (`context/plans/20260920-a-print-language-the-shop-keeps.md`).
#[derive(Deserialize)]
pub struct TicketQuery {
    lang: Lang,
    print_lang: Option<Lang>,
}

/// The 80 mm ticket for a stored sale, as an HTML page.
///
/// The core renders it (features.md §4: the same bytes from the desktop and
/// from a server with no screen), so this handler reads the document, the
/// language it prints in comes out of the same look at the file, and the
/// string is handed over. A lang the app does not print, or none at all, is
/// the caller's mistake and answers 422 in the envelope like every other
/// unreadable request. The id has to name a ticket: a facture squeezed onto
/// a till slip is a facture nobody would take for one, so its id answers 404
/// here, the same way a ticket's does on the facture route.
pub async fn ticket(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    query: Result<Query<TicketQuery>, QueryRejection>,
) -> Result<Html<String>, ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let Query(TicketQuery {
        lang: caller,
        print_lang: named,
    }) = query.map_err(|_| ApiError::BadRequest("lang must be fr, en or ar".into()))?;
    let shop = state.shop_id;
    let (found, lang, show_fiscal_ids) = state
        .blocking(move |c| -> Result<_, CoreError> {
            let found = documents::get_of_kind(c, shop, id, DocumentKind::Ticket)?;
            let lang = preferences::print_lang_for(c, shop, named, caller)?;
            let show_fiscal_ids = preferences::ticket_fiscal_ids(c, shop)?;
            Ok((found, lang, show_fiscal_ids))
        })
        .await?;
    Ok(Html(render_ticket(&found, lang, show_fiscal_ids)?))
}

/// The same ticket as ESC/POS bytes for a thermal printer.
///
/// Same document, same `lang` and `print_lang`, same refusal as the HTML
/// route when the row is not a ticket or carries an IFU TVA recap. Returns
/// the raw bytes the head eats, so the desktop can write them to a file, a
/// USB-serial device or a TCP printer on port 9100. The transport stays out
/// of the API: the caller here decides whether those bytes go to a spool
/// file or over the wire, and `crates/core/src/print/escpos.rs` is what that
/// caller calls next.
/// Which path the head is sent is read here in the same look at the file as
/// the document and the language, the shape `facture` reads its layout in:
/// the bytes and the mode they were chosen under have to be answers about
/// one document. `lang=ar` comes back as raster bands whatever the shop
/// stored, because there is no single-byte table with Arabic in it; `fr`
/// and `en` come back as text until the shop's preference says otherwise
/// (`preferences::thermal_mode_for`, which is the only place that rule
/// lives).
pub async fn ticket_escpos(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    query: Result<Query<TicketQuery>, QueryRejection>,
) -> Result<axum::response::Response, ApiError> {
    use axum::http::{header, HeaderValue};
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let Query(TicketQuery {
        lang: caller,
        print_lang: named,
    }) = query.map_err(|_| ApiError::BadRequest("lang must be fr, en or ar".into()))?;
    let shop = state.shop_id;
    let (found, lang, mode, show_fiscal_ids) = state
        .blocking(move |c| -> Result<_, CoreError> {
            let found = documents::get_of_kind(c, shop, id, DocumentKind::Ticket)?;
            let lang = preferences::print_lang_for(c, shop, named, caller)?;
            let mode = preferences::thermal_mode_for(c, shop, lang)?;
            let show_fiscal_ids = preferences::ticket_fiscal_ids(c, shop)?;
            Ok((found, lang, mode, show_fiscal_ids))
        })
        .await?;
    let bytes = render_ticket_escpos_in(&found, lang, mode, show_fiscal_ids)?;
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

/// Print through desktop (M6 T6): the phone POSTs, the desktop spools the
/// same bytes `ticket_escpos` would return. Spool is
/// `spool/ticket-<id>-<lang>-<mode>.bin` beside the shop file (never
/// pruned), and if `DZPOS_PRINTER_ADDR` (e.g. `192.168.1.50:9100`) is set
/// the bytes are also pushed to that TCP printer. Idempotent: re-printing
/// overwrites the same spool file.
///
/// `<lang>` and `<mode>` in the spool name, and the two this answers with,
/// are both the resolved answers and never the caller's own: the spool file
/// and the paper it stands for have to agree. The mode is in the name
/// because the two paths produce bytes nothing about the file says apart —
/// one is text and one is a bitmap — and a phone that inherits the desktop's
/// spool should be able to see which it is holding without decoding it.
pub async fn print_ticket(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    query: Result<Query<TicketQuery>, QueryRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use serde_json::json;
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let Query(TicketQuery {
        lang: caller,
        print_lang: named,
    }) = query.map_err(|_| ApiError::BadRequest("lang must be fr, en or ar".into()))?;
    let shop = state.shop_id;
    let (found, lang, mode, show_fiscal_ids) = state
        .blocking(move |c| -> Result<_, CoreError> {
            let found = documents::get_of_kind(c, shop, id, DocumentKind::Ticket)?;
            let lang = preferences::print_lang_for(c, shop, named, caller)?;
            let mode = preferences::thermal_mode_for(c, shop, lang)?;
            let show_fiscal_ids = preferences::ticket_fiscal_ids(c, shop)?;
            Ok((found, lang, mode, show_fiscal_ids))
        })
        .await?;
    let bytes = render_ticket_escpos_in(&found, lang, mode, show_fiscal_ids)?;
    let spool_dir = state
        .db_path()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("spool");
    std::fs::create_dir_all(&spool_dir).map_err(|e| ApiError::from(CoreError::from(e)))?;
    let spool_path = spool_dir.join(format!(
        "ticket-{}-{}-{}.bin",
        id,
        lang.tag(),
        mode.as_str()
    ));
    std::fs::write(&spool_path, &bytes).map_err(|e| ApiError::from(CoreError::from(e)))?;
    if let Ok(addr) = std::env::var("DZPOS_PRINTER_ADDR") {
        if !addr.is_empty() {
            let _ = tokio::task::spawn_blocking({
                let found = found.clone();
                let addr = addr.clone();
                move || {
                    // The same mode the spool file was written under. The
                    // sender resolves it again through the same rule, so
                    // the file on disk and the bytes on the wire are the
                    // same bytes for the same document.
                    let _ = dzpos_core::print::send_ticket_escpos_tcp(
                        &found,
                        lang,
                        mode,
                        show_fiscal_ids,
                        &addr,
                    );
                }
            })
            .await;
        }
    }
    Ok(Json(json!({
        "spooled": spool_path.display().to_string(),
        "bytes": bytes.len(),
        "lang": lang.tag(),
        "thermal_mode": mode.as_str(),
    })))
}

/// The sheet, named by the caller on every call and not a setting: the same
/// facture goes on A4 in the office and on A5 at the counter, and the till
/// is the only place that knows which the cashier reached for (features.md
/// §4). `lang` is named the same way, for the same reason, but does not
/// decide the paper on its own: `print_lang` overrides it for one call and
/// the shop's stored preference sits between the two, resolved by
/// `preferences::print_lang_for`
/// (`context/plans/20260920-a-print-language-the-shop-keeps.md`).
///
/// The layout is the opposite and is left out of most calls. It is how the
/// page is drawn rather than what it is drawn on, the shop chooses it once in
/// settings, and every facture follows that choice. Naming it here overrides
/// the choice for one page, which is what a settings screen showing a preview
/// of each layout needs and nothing else does.
/// The sheet a query string may name.
///
/// `Paper` has a third value and this type deliberately does not. The roll is
/// not a sheet a print dialog offers, and the one layout drawn for it names
/// it itself (`FactureLayout::fixed_paper`), so there is nothing for a caller
/// to say here. Without the narrowing,
/// `?paper=roll_80mm&layout=standard` answers 200 with the six column A4
/// body under `@page { size: 80mm auto; margin: 12mm }`: a table measured for
/// 190 mm, on 80 mm of paper, with 24 mm of that given to the margins.
///
/// The TypeScript client already spells this type `"a4" | "a5"`
/// (`packages/shared/src/client-response.ts`), so this is the two sides
/// agreeing rather than a new restriction.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sheet {
    A4,
    A5,
}

impl From<Sheet> for Paper {
    fn from(sheet: Sheet) -> Paper {
        match sheet {
            Sheet::A4 => Paper::A4,
            Sheet::A5 => Paper::A5,
        }
    }
}

#[derive(Deserialize)]
pub struct FactureQuery {
    lang: Lang,
    paper: Sheet,
    layout: Option<FactureLayout>,
    print_lang: Option<Lang>,
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
    let Query(FactureQuery {
        lang: caller,
        paper,
        layout,
        print_lang: named,
    }) = query.map_err(|_| {
        ApiError::BadRequest(
            "lang must be fr, en or ar, paper a4 or a5, and layout one the shop has".into(),
        )
    })?;
    let shop = state.shop_id;
    let (found, referenced, chosen, lang) = state
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
            // The shop's layout is read in the same call as the document, so
            // the page and the layout it is drawn in come out of one look at
            // the file. A layout named in the query wins, for the preview.
            let chosen = match layout {
                Some(layout) => layout,
                None => preferences::facture_layout(c, shop)?,
            };
            // Same shape as the layout above, and the same read: the language
            // the page is drawn in comes out of the same look at the file.
            let lang = preferences::print_lang_for(c, shop, named, caller)?;
            Ok((found, referenced, chosen, lang))
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
        Page {
            paper: paper.into(),
            layout: chosen,
        },
    )?))
}

/// The same facture as ESC/POS bytes for a thermal head, on the 80 mm roll.
///
/// Same document, same `lang` and `print_lang`, same refusals as the HTML
/// route: an IFU document carrying a TVA recap, a document with no buyer
/// block, an avoir carrying a droit de timbre, a proforma carrying a debt.
/// They are one list in `print::refusals` and this route reaches it through
/// the same `built_view` the four HTML layouts do, so a page drawn on a roll
/// refuses exactly what a page drawn on A4 refuses.
///
/// No `paper` and no `layout`: a thermal head has one width and no tray, and
/// the roll is the only layout drawn for it. The second document an avoir
/// names and the day a cancellation was taken are read here for the reason
/// the HTML route reads them — this is the layer that can open a second row.
pub async fn facture_escpos(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    query: Result<Query<TicketQuery>, QueryRejection>,
) -> Result<axum::response::Response, ApiError> {
    use axum::http::{header, HeaderValue};
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let Query(TicketQuery {
        lang: caller,
        print_lang: named,
    }) = query.map_err(|_| ApiError::BadRequest("lang must be fr, en or ar".into()))?;
    let shop = state.shop_id;
    let (found, referenced, lang, mode) = state
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
            let referenced = match found.ref_document_id {
                Some(ref_id) => Some(documents::get(c, shop, ref_id)?),
                None => None,
            };
            let lang = preferences::print_lang_for(c, shop, named, caller)?;
            let mode = preferences::thermal_mode_for(c, shop, lang)?;
            Ok((found, referenced, lang, mode))
        })
        .await?;
    let cancellation = found.cancellation.as_ref().map(|c| Cancellation {
        at: c.at,
        reason: &c.reason,
    });
    let bytes = render_facture_escpos(
        &found,
        &FactureInput {
            referenced: referenced.as_ref(),
            cancellation,
        },
        lang,
        mode,
    )?;
    let mut res = axum::response::Response::new(axum::body::Body::from(bytes));
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    res.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"facture.bin\""),
    );
    Ok(res)
}

pub async fn create(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<NewSaleDto>, JsonRejection>,
) -> Result<(StatusCode, Json<SaleDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let key = dto.idempotency_key.clone();
    let new = NewSale::try_from(dto)?;
    let shop = state.shop_id;
    let user = who.id;
    // The warning the core answered with travels on this one answer only:
    // it is about the moment the sale was rung up, and a later read of the
    // same document carries none.
    let made = state
        .blocking(move |c| match key {
            Some(key) => sales::issue_idempotent(c, shop, user, new, key),
            None => sales::issue(c, shop, user, new),
        })
        .await?;
    // A replay answers the stored paper with 200: the ring happened on an
    // earlier call, and 201 would claim this one rang it. Only a fresh ring
    // is 201.
    let status = if made.replayed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((status, Json(SaleDto::from(made))))
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
    let refund = RefundDto::refund(dto.refund);
    let reason = dto.reason;
    let shop = state.shop_id;
    let user = who.id;
    let made = state
        .blocking(move |c| avoir::issue_settling(c, shop, user, id, lines, reason, None, refund))
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
    let Json(CancelDocumentDto { reason, refund }) = body.map_err(ApiError::from)?;
    let refund = RefundDto::refund(refund);
    let shop = state.shop_id;
    let user = who.id;
    let done = state
        .blocking(move |c| cancellation::cancel_settling(c, shop, user, id, reason, None, refund))
        .await?;
    Ok(Json(SaleDto::from(done)))
}
