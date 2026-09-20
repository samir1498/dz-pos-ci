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
use axum::response::Html;
use axum::Json;
use dzpos_core::db::Conn;
use dzpos_core::error::CoreError;
use dzpos_core::lang::Lang;
use dzpos_core::models::document::SellerBlock;
use dzpos_core::print::debt_slip::MOVEMENTS;
use dzpos_core::print::{render_debt_slip, render_statement, Paper};
use dzpos_core::services::customers::NewCustomer;
use dzpos_core::services::{clock, customers as service, debt, preferences, shops};
use serde::Deserialize;

use crate::dto::{
    money_field, parse_day, AdjustmentDto, CustomerDto, CustomerLedgerDto, CustomerPaymentsDto,
    CustomerWriteDto, NewCustomerDto, NewPaymentDto, PaymentDto,
};
use crate::error::ApiError;
use crate::session::CurrentUser;
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
    who: CurrentUser,
    body: Result<Json<NewCustomerDto>, JsonRejection>,
) -> Result<(StatusCode, Json<CustomerDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    // An opening debt that was not sent stays `None`: the core reads that as
    // no movement at all, and a zero it does not write either.
    let opening = money_field("opening_debt_centimes", dto.opening_debt_centimes)?;
    let new = NewCustomer::try_from(dto)?;
    let shop = state.shop_id;
    let user = who.id;
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
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<CustomerWriteDto>, JsonRejection>,
) -> Result<Json<CustomerDto>, ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    // Lifted off the body before the fiche is built: it is a note about the
    // decision, not a field of the customer, and nothing stores it but the
    // audit row.
    let close_reason = dto.close_reason.clone();
    let fields = NewCustomer::try_from(dto)?;
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| {
            service::update(c, shop, user, id, fields, close_reason)?;
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
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<AdjustmentDto>, JsonRejection>,
) -> Result<(StatusCode, Json<CustomerLedgerDto>), ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let amount = dto.amount()?;
    let note = dto.note;
    let shop = state.shop_id;
    let user = who.id;
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
    who: CurrentUser,
    id: Result<Path<i32>, PathRejection>,
    body: Result<Json<NewPaymentDto>, JsonRejection>,
) -> Result<(StatusCode, Json<CustomerPaymentsDto>), ApiError> {
    let id = path_id(id)?;
    let Json(dto) = body.map_err(ApiError::from)?;
    let amount = dto.amount()?;
    let mode = dto.payment_mode.into();
    let note = dto.note;
    let shop = state.shop_id;
    let user = who.id;
    // The moment is the server's, not the till's: a machine whose clock is
    // wrong must not decide which side of a statement's date range a payment
    // falls on. The shop's calendar, which is the one clock the ledger and a
    // document's `issued_at` are both on.
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

/// The days a statement covers, named by the caller on every call because
/// only the screen knows the range it is showing. `lang` is the app's own
/// language, named the same way; `print_lang`, left out of most calls,
/// overrides it for the one call, and between the two sits the shop's stored
/// preference, resolved by `preferences::print_lang_for`
/// (`context/plans/20260920-a-print-language-the-shop-keeps.md`).
#[derive(Deserialize)]
pub struct StatementQuery {
    from: String,
    to: String,
    lang: Lang,
    print_lang: Option<Lang>,
}

/// The statement of account for a range of days, as the HTML page the core
/// rendered (features.md §2 and §4). The core renders it, so the desktop and
/// a server with no screen hand over the same bytes.
pub async fn statement(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    range: Result<Query<StatementQuery>, QueryRejection>,
) -> Result<Html<String>, ApiError> {
    let id = path_id(id)?;
    let Query(StatementQuery {
        from,
        to,
        lang: caller,
        print_lang: named,
    }) = range.map_err(|_| {
        ApiError::BadRequest(
            "from and to are days written YYYY-MM-DD and lang is fr, en or ar".into(),
        )
    })?;
    let from = parse_day("from", &from)?;
    let to = parse_day("to", &to)?;
    let shop = state.shop_id;
    let page = state
        .blocking(move |c| {
            // The fiche as it stands, not as it stood: a statement is a page
            // about the account, and the address it is posted to is the one
            // on the fiche today. The buyer block a facture snapshotted is
            // the other question, and the facture answers it.
            let customer = service::get(c, shop, id)?;
            let statement = debt::statement_between(c, shop, id, from, to)?;
            // The language the page is drawn in comes out of the same call:
            // the resolved language and the account it prints have to be
            // read together.
            let lang = preferences::print_lang_for(c, shop, named, caller)?;
            render_statement(&customer, &statement, lang, Paper::A4)
        })
        .await?;
    Ok(Html(page))
}

/// The language a counter paper prints in, named by the caller like the
/// statement's. `print_lang`, left out of most calls, overrides it for the
/// one call the same way, and between the two sits the shop's stored
/// preference, resolved by `preferences::print_lang_for`
/// (`context/plans/20260920-a-print-language-the-shop-keeps.md`).
#[derive(Deserialize)]
pub struct PrintQuery {
    lang: Lang,
    print_lang: Option<Lang>,
}

/// The 80 mm debt slip, as the HTML page the core rendered (features.md §2
/// and §4). What a credit customer is handed at the counter when they ask
/// what they owe: the balance, the newest movements behind it, and a line
/// saying the paper has no fiscal value.
///
/// Three reads inside one call and none of them a calculation: the shop as it
/// stands, the fiche as it stands, and the ledger. The slip carries no number
/// from a series, so unlike a facture there is nothing here that was
/// snapshotted on a day and has to be printed back as it was.
pub async fn debt_slip(
    State(state): State<AppState>,
    id: Result<Path<i32>, PathRejection>,
    print: Result<Query<PrintQuery>, QueryRejection>,
) -> Result<Html<String>, ApiError> {
    let id = path_id(id)?;
    let Query(PrintQuery {
        lang: caller,
        print_lang: named,
    }) = print.map_err(|_| ApiError::BadRequest("lang is fr, en or ar".into()))?;
    let shop = state.shop_id;
    // The moment is the server's, the same clock the ledger's rows are
    // stamped by: a till whose clock is wrong must not date the paper it
    // hands over.
    let at = clock::now();
    let page = state
        .blocking(move |c| {
            let seller = SellerBlock::from(shops::get(c, shop)?);
            let customer = service::get(c, shop, id)?;
            let slip = debt::recent(c, shop, id, MOVEMENTS)?;
            let lang = preferences::print_lang_for(c, shop, named, caller)?;
            render_debt_slip(&seller, &customer, &slip, at, lang)
        })
        .await?;
    Ok(Html(page))
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
