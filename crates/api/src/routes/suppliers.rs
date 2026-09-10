//! The suppliers screen's routes. They translate: the fiche's rules live in
//! `dzpos_core::services::suppliers` and the ledger's in
//! `dzpos_core::services::supplier_debt`.
//!
//! Nothing here deletes a supplier. The ledger and the orders hold the fiche
//! (both foreign keys are RESTRICT), and a shop that has stopped buying from
//! somebody closes the fiche, which is what `POST /suppliers/{id}/close`
//! does.
//!
//! The statement answers JSON and not a rendered page: the printed statement
//! is a paper a customer is handed, and nobody hands a supplier the shop's
//! own copy of what it owes.

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use dzpos_core::db::Conn;
use dzpos_core::error::CoreError;
use dzpos_core::services::suppliers::NewSupplier;
use dzpos_core::services::{clock, supplier_debt as debt, suppliers as service};
use serde::Deserialize;

use crate::dto::{
    money_field, parse_day, AdjustmentDto, CloseSupplierDto, NewPaymentDto, NewSupplierDto,
    SupplierDto, SupplierEntryDto, SupplierLedgerDto, SupplierStatementDto, SupplierWriteDto,
    DATE_FORMAT, DATE_TIME_FORMAT,
};
use crate::error::ApiError;
use crate::AppState;

/// The search box, as it reaches the API. Blank is no filter, so a box that
/// has been emptied reads the whole list rather than nothing.
#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    q: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    search: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Vec<SupplierDto>>, ApiError> {
    let Query(ListQuery { q }) =
        search.map_err(|_| ApiError::BadRequest("q must be a piece of text".into()))?;
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| service::list_with_balance(c, shop, q.as_deref()))
        .await?;
    Ok(Json(found.into_iter().map(SupplierDto::from).collect()))
}

pub async fn get_one(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
) -> Result<Json<SupplierDto>, ApiError> {
    let id = path_id(id)?;
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| service::get_with_balance(c, shop, id))
        .await?;
    Ok(Json(SupplierDto::from(found)))
}

/// A fiche, and the debt the shop was already carrying to this supplier when
/// there is one. The two are written together (core, `create`).
pub async fn create(
    State(state): State<AppState>,
    body: Result<Json<NewSupplierDto>, JsonRejection>,
) -> Result<(StatusCode, Json<SupplierDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    // An opening debt that was not sent stays `None`: the core reads that as
    // no movement at all, and a zero it does not write either.
    let opening = money_field("opening_debt_centimes", dto.opening_debt_centimes)?;
    let new = NewSupplier::from(dto);
    let shop = state.shop_id;
    // TODO(M4): the user comes from the request identity, not from the state.
    let user = state.user_id;
    let made = state
        .blocking(move |c| {
            let created = service::create(c, shop, user, new, opening)?;
            service::get_with_balance(c, shop, created.id)
        })
        .await?;
    Ok((StatusCode::CREATED, Json(SupplierDto::from(made))))
}

/// The whole fiche again, not a patch: the screen sends every field it shows,
/// so a field left out is a bug at the edge and a null clears the column. The
/// opening debt is not among them; a wrong one is corrected by an adjustment.
pub async fn update(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<SupplierWriteDto>, JsonRejection>,
) -> Result<Json<SupplierDto>, ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    // Lifted off the body before the fiche is built: it is a note about the
    // decision, not a field of the supplier, and nothing stores it but the
    // audit row.
    let close_reason = dto.close_reason.clone();
    let fields = NewSupplier::from(dto);
    let shop = state.shop_id;
    let user = state.user_id;
    let after = state
        .blocking(move |c| {
            service::update(c, shop, user, id, fields, close_reason)?;
            service::get_with_balance(c, shop, id)
        })
        .await?;
    Ok(Json(SupplierDto::from(after)))
}

/// Stops the shop buying from this supplier. A fiche whose account is still
/// open needs the reason, and the audit entry carries it beside the balance.
pub async fn close(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<CloseSupplierDto>, JsonRejection>,
) -> Result<Json<SupplierDto>, ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let shop = state.shop_id;
    let user = state.user_id;
    let after = state
        .blocking(move |c| {
            service::close(c, shop, user, id, dto.reason)?;
            service::get_with_balance(c, shop, id)
        })
        .await?;
    Ok(Json(SupplierDto::from(after)))
}

pub async fn ledger(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
) -> Result<Json<SupplierLedgerDto>, ApiError> {
    let id = path_id(id)?;
    let shop = state.shop_id;
    let answer = state.blocking(move |c| envelope(c, shop, id)).await?;
    Ok(Json(answer))
}

/// Money to a supplier (features.md §1). One transaction in the core: the
/// movement and the orders it settled.
///
/// The answer is the whole ledger again, the way an adjustment answers it:
/// the screen shows the new movement, what it settled and the new balance
/// without a second call, and what it shows is what the core stored rather
/// than what the form sent.
pub async fn pay(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<NewPaymentDto>, JsonRejection>,
) -> Result<(StatusCode, Json<SupplierLedgerDto>), ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let amount = dto.amount()?;
    let mode = dto.payment_mode.into();
    let note = dto.note;
    let shop = state.shop_id;
    // TODO(M4): the user comes from the request identity, not from the state.
    let user = state.user_id;
    // The moment is the server's, not the till's: a machine whose clock is
    // wrong must not decide which side of a statement's date range a payment
    // falls on.
    let at = clock::now();
    let written = state
        .blocking(move |c| {
            debt::pay(c, shop, user, id, amount, mode, note, at)?;
            envelope(c, shop, id)
        })
        .await?;
    Ok((StatusCode::CREATED, Json(written)))
}

/// Corrects a balance by writing a movement. The answer is the whole ledger
/// again, read inside the write's own transaction in the core, so the balance
/// answered here is the figure the audit entry carries.
pub async fn adjust(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<AdjustmentDto>, JsonRejection>,
) -> Result<(StatusCode, Json<SupplierLedgerDto>), ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let amount = dto.amount()?;
    let note = dto.note;
    let shop = state.shop_id;
    let user = state.user_id;
    let written = state
        .blocking(move |c| {
            debt::adjust(c, shop, user, id, amount, note)?;
            envelope(c, shop, id)
        })
        .await?;
    Ok((StatusCode::CREATED, Json(written)))
}

/// The days a statement covers. No language: this one is JSON for a screen,
/// not a page for a printer.
#[derive(Deserialize)]
pub struct StatementQuery {
    from: String,
    to: String,
}

/// The supplier's account between two days, both included.
pub async fn statement(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    range: Result<Query<StatementQuery>, QueryRejection>,
) -> Result<Json<SupplierStatementDto>, ApiError> {
    let id = path_id(id)?;
    let Query(StatementQuery { from, to }) = range
        .map_err(|_| ApiError::BadRequest("from and to are days written YYYY-MM-DD".into()))?;
    let from = parse_day("from", &from)?;
    let to = parse_day("to", &to)?;
    let answer = state.shop_id;
    let statement = state
        .blocking(move |c| {
            let ranged = debt::statement_between(c, answer, id, from, to)?;
            let mut entries = Vec::with_capacity(ranged.entries.len());
            for line in ranged.entries {
                entries.push(entry(c, answer, line)?);
            }
            Ok::<_, CoreError>(SupplierStatementDto {
                supplier_id: id,
                from: ranged.from.format(DATE_FORMAT).to_string(),
                to: ranged.to.format(DATE_FORMAT).to_string(),
                opening_centimes: ranged.opening.as_centimes(),
                entries,
                closing_centimes: ranged.closing.as_centimes(),
            })
        })
        .await?;
    Ok(Json(statement))
}

/// The ledger with its balance, read once. Every route that writes a movement
/// answers this, so the screen never has a movement without the balance it
/// left behind.
fn envelope(conn: &mut Conn, shop: i32, supplier_id: i32) -> Result<SupplierLedgerDto, CoreError> {
    let statement = debt::statement(conn, shop, supplier_id)?;
    let mut entries = Vec::with_capacity(statement.lines.len());
    for line in statement.lines {
        entries.push(entry(conn, shop, line)?);
    }
    Ok(SupplierLedgerDto {
        supplier_id,
        balance_centimes: statement.balance.as_centimes(),
        entries,
    })
}

/// One movement, with what it settled when it settled anything. The
/// allocations are read for the payment rows only: every other kind has none
/// by construction, and asking would be a query per row of the ledger.
fn entry(
    conn: &mut Conn,
    shop: i32,
    line: debt::LedgerLine,
) -> Result<SupplierEntryDto, CoreError> {
    let allocations = if line.entry.kind == debt::SupplierDebtKind::Payment {
        debt::allocations_of_payment(conn, shop, line.entry.id)?
    } else {
        Vec::new()
    };
    Ok(SupplierEntryDto {
        id: line.entry.id,
        supplier_id: line.entry.supplier_id,
        purchase_id: line.entry.purchase_id,
        kind: line.entry.kind.into(),
        debit_centimes: line.entry.debit.as_centimes(),
        credit_centimes: line.entry.credit.as_centimes(),
        balance_after_centimes: line.balance_after.as_centimes(),
        payment_mode: line.entry.payment_mode.map(Into::into),
        user_id: line.entry.user_id,
        note: line.entry.note,
        allocations: allocations.into_iter().map(Into::into).collect(),
        created_at: line.entry.created_at.format(DATE_TIME_FORMAT).to_string(),
    })
}

/// `/suppliers/abc` leaves in the same envelope as every other refusal rather
/// than as axum's own text/plain 400.
fn path_id(id: Result<Path<i32>, PathRejection>) -> Result<i32, ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    Ok(id)
}
