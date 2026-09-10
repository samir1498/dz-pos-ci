//! The expenses screen's routes, and the cash position beside them. They
//! translate: the rules live in `dzpos_core::services::expenses` and
//! `dzpos_core::services::cash`.
//!
//! Nothing here edits or deletes an expense. An expense is written once
//! (features.md §1) and the table carries no cancellation block, so there is
//! no honest route to offer yet.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use dzpos_core::services::clock::Period;
use dzpos_core::services::expenses::NewExpense;
use dzpos_core::services::{cash as cash_service, expenses as service};
use serde::Deserialize;

use crate::dto::{
    parse_day, parse_month, CashPositionDto, ExpenseCategoryDto, ExpenseDto, ExpensesDto,
    NewExpenseDto,
};
use crate::error::ApiError;
use crate::AppState;

/// The month a list is asked for, `YYYY-MM`.
#[derive(Deserialize)]
pub struct MonthQuery {
    month: String,
}

/// The shop's expenses for one month, with what the month came to.
pub async fn list(
    State(state): State<AppState>,
    month: Result<Query<MonthQuery>, QueryRejection>,
) -> Result<Json<ExpensesDto>, ApiError> {
    let Query(MonthQuery { month }) =
        month.map_err(|_| ApiError::BadRequest("month is a month written YYYY-MM".into()))?;
    let month = parse_month("month", &month)?;
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| {
            let expenses = service::list(c, shop, month)?;
            let total = service::total(c, shop, month)?;
            Ok::<_, dzpos_core::error::CoreError>(ExpensesDto {
                month: month.as_text(),
                total_centimes: total.as_centimes(),
                expenses: expenses.into_iter().map(ExpenseDto::from).collect(),
            })
        })
        .await?;
    Ok(Json(found))
}

/// The seven seeded categories, in the order the screen lists them. A shop
/// adds none of its own in this version, so this is a read and there is no
/// write beside it.
pub async fn categories(
    State(state): State<AppState>,
) -> Result<Json<Vec<ExpenseCategoryDto>>, ApiError> {
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| service::categories(c, shop))
        .await?;
    Ok(Json(
        found.into_iter().map(ExpenseCategoryDto::from).collect(),
    ))
}

pub async fn create(
    State(state): State<AppState>,
    body: Result<Json<NewExpenseDto>, JsonRejection>,
) -> Result<(StatusCode, Json<ExpenseDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let fields = NewExpense::try_from(dto)?;
    let shop = state.shop_id;
    // TODO(M4): the user comes from the request identity, not from the state.
    let user = state.user_id;
    let made = state
        .blocking(move |c| service::create(c, shop, user, fields))
        .await?;
    Ok((StatusCode::CREATED, Json(ExpenseDto::from(made))))
}

/// A day or a month, never both and never neither. One route because it is
/// one figure over one range, and two query parameters because a screen asks
/// the question one way or the other rather than passing a shape.
#[derive(Deserialize)]
pub struct PeriodQuery {
    #[serde(default)]
    day: Option<String>,
    #[serde(default)]
    month: Option<String>,
}

/// The cash position (features.md §1, Dashboard). Summed from the ledgers on
/// every call; nothing stores it.
pub async fn cash(
    State(state): State<AppState>,
    period: Result<Query<PeriodQuery>, QueryRejection>,
) -> Result<Json<CashPositionDto>, ApiError> {
    let Query(PeriodQuery { day, month }) = period.map_err(|_| {
        ApiError::BadRequest("day is a day written YYYY-MM-DD and month a month YYYY-MM".into())
    })?;
    let period = match (day.as_deref(), month.as_deref()) {
        (Some(day), None) => Period::Day(parse_day("day", day)?),
        (None, Some(month)) => Period::Month(parse_month("month", month)?),
        // Both is two questions and neither is none: either way the caller
        // has not said which range it wants, and answering one of them would
        // be this route choosing.
        _ => {
            return Err(ApiError::Request(dzpos_core::error::CoreError::validation(
                "period",
                "ask for a day or for a month, one of the two",
            )))
        }
    };
    let shop = state.shop_id;
    let position = state
        .blocking(move |c| cash_service::position(c, shop, period))
        .await?;
    Ok(Json(CashPositionDto::try_from(position)?))
}
