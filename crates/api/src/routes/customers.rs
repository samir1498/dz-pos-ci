//! The customers screen's routes. They translate: the fiche's rules live in
//! `dzpos_core::services::customers` and the ledger's in
//! `dzpos_core::services::debt`.
//!
//! Nothing here deletes a customer. The ledger holds the fiche (the foreign
//! key is RESTRICT), and a shop that has stopped dealing with somebody
//! deactivates them, which is the update with `active` false.

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use dzpos_core::db::Conn;
use dzpos_core::error::CoreError;
use dzpos_core::services::customers::NewCustomer;
use dzpos_core::services::{clock, customers as service, debt};
use serde::Deserialize;

use crate::dto::{
    money_field, AdjustmentDto, CustomerDto, CustomerLedgerDto, CustomerPaymentsDto,
    CustomerWriteDto, NewCustomerDto, NewPaymentDto, PaymentDto,
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
) -> Result<Json<Vec<CustomerDto>>, ApiError> {
    let Query(ListQuery { q }) =
        search.map_err(|_| ApiError::BadRequest("q must be a piece of text".into()))?;
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| service::list_with_balance(c, shop, q.as_deref()))
        .await?;
    Ok(Json(found.into_iter().map(CustomerDto::from).collect()))
}

pub async fn get_one(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
) -> Result<Json<CustomerDto>, ApiError> {
    let id = path_id(id)?;
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| service::get_with_balance(c, shop, id))
        .await?;
    Ok(Json(CustomerDto::from(found)))
}

/// A fiche, and the debt the shop was already carrying for this customer,
/// when there is one. The two are written together (core, `create`).
pub async fn create(
    State(state): State<AppState>,
    body: Result<Json<NewCustomerDto>, JsonRejection>,
) -> Result<(StatusCode, Json<CustomerDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    // An opening debt that was not sent stays `None`: the core reads that as
    // no movement at all, and a zero it does not write either.
    let opening = money_field("opening_debt_centimes", dto.opening_debt_centimes)?;
    let new = NewCustomer::try_from(dto)?;
    let shop = state.shop_id;
    // TODO(M4): the user comes from the request identity, not from the state.
    let user = state.user_id;
    let made = state
        .blocking(move |c| {
            let created = service::create(c, shop, user, new, opening)?;
            service::get_with_balance(c, shop, created.id)
        })
        .await?;
    Ok((StatusCode::CREATED, Json(CustomerDto::from(made))))
}

/// The whole fiche again, not a patch: the screen sends every field it
/// shows, so a field left out is a bug at the edge and a null clears the
/// column. The opening debt is not among them; a wrong one is corrected by
/// an adjustment.
pub async fn update(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<CustomerWriteDto>, JsonRejection>,
) -> Result<Json<CustomerDto>, ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let fields = NewCustomer::try_from(dto)?;
    let shop = state.shop_id;
    let user = state.user_id;
    let after = state
        .blocking(move |c| {
            service::update(c, shop, user, id, fields)?;
            service::get_with_balance(c, shop, id)
        })
        .await?;
    Ok(Json(CustomerDto::from(after)))
}

pub async fn ledger(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
) -> Result<Json<CustomerLedgerDto>, ApiError> {
    let id = path_id(id)?;
    let shop = state.shop_id;
    let statement = state
        .blocking(move |c| debt::statement(c, shop, id))
        .await?;
    Ok(Json(envelope(id, statement)))
}

/// Corrects a balance by writing a movement (features.md §2). The answer is
/// the whole ledger again: the screen shows the new balance and the new row
/// without a second call, and the row it just wrote is the one the core
/// stored rather than the one the form sent.
///
/// The ledger is the one the write's own transaction read (core, `adjust`),
/// not a second read afterwards: the balance answered here is the figure the
/// audit entry carries, so the log and the screen cannot disagree.
pub async fn adjust(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<AdjustmentDto>, JsonRejection>,
) -> Result<(StatusCode, Json<CustomerLedgerDto>), ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let amount = dto.amount()?;
    let note = dto.note;
    let shop = state.shop_id;
    let user = state.user_id;
    let written = state
        .blocking(move |c| debt::adjust(c, shop, user, id, amount, note))
        .await?;
    Ok((StatusCode::CREATED, Json(envelope(id, written.statement))))
}

/// Money against the debt (features.md §2). One transaction in the core: the
/// movement, the documents it settled and the remaining debt on each of them.
///
/// The answer is the whole list of payments again, the way an adjustment
/// answers the whole ledger: the screen shows the new payment with its
/// allocations and the new balance without a second call, and what it shows
/// is what the core stored rather than what the form sent.
pub async fn pay(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<NewPaymentDto>, JsonRejection>,
) -> Result<(StatusCode, Json<CustomerPaymentsDto>), ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let amount = dto.amount()?;
    let mode = dto.payment_mode.into();
    let note = dto.note;
    let shop = state.shop_id;
    // TODO(M4): the user comes from the request identity, not from the state.
    let user = state.user_id;
    // The moment is the server's, like a document's `issued_at`: a till whose
    // clock is wrong must not decide which side of a statement's date range a
    // payment falls on.
    let at = clock::now();
    let written = state
        .blocking(move |c| {
            debt::pay(c, shop, user, id, amount, mode, note, at)?;
            payments_envelope(c, shop, id)
        })
        .await?;
    Ok((StatusCode::CREATED, Json(written)))
}

/// The customer's payments, newest first, each with what it settled.
pub async fn payments(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
) -> Result<Json<CustomerPaymentsDto>, ApiError> {
    let id = path_id(id)?;
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| payments_envelope(c, shop, id))
        .await?;
    Ok(Json(found))
}

fn payments_envelope(
    conn: &mut Conn,
    shop: i32,
    customer_id: i32,
) -> Result<CustomerPaymentsDto, CoreError> {
    let payments = debt::payments(conn, shop, customer_id)?;
    // The balance is the ledger's whole sum, not the newest payment's: a sale
    // written after the last payment moved it, and the fiche beside this list
    // shows the same figure.
    let balance = debt::balance(conn, shop, customer_id)?;
    Ok(CustomerPaymentsDto {
        customer_id,
        balance_centimes: balance.as_centimes(),
        payments: payments.into_iter().map(PaymentDto::from).collect(),
    })
}

fn envelope(customer_id: i32, statement: debt::Statement) -> CustomerLedgerDto {
    CustomerLedgerDto {
        customer_id,
        balance_centimes: statement.balance.as_centimes(),
        entries: statement.lines.into_iter().map(Into::into).collect(),
    }
}

/// `/customers/abc` leaves in the same envelope as every other refusal
/// rather than as axum's own text/plain 400.
fn path_id(id: Result<Path<i32>, PathRejection>) -> Result<i32, ApiError> {
    let Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    Ok(id)
}
