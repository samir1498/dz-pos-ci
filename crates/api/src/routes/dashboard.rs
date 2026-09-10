//! The dashboard's one route. It translates: every figure on it is decided by
//! `dzpos_core::services::dashboard`.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::Json;
use dzpos_core::services::{clock, dashboard as service};
use serde::Deserialize;

use crate::dto::{parse_day, DashboardDto};
use crate::error::ApiError;
use crate::AppState;

/// The day the screen is asked about. Left out, it is the shop's today: the
/// desktop has one clock and this route reads the same one the services date
/// documents with, so a screen that sent nothing and a screen that sent the
/// day the `/clock` route gave it get the same answer.
#[derive(Deserialize)]
pub struct DayQuery {
    #[serde(default)]
    day: Option<String>,
}

pub async fn read(
    State(state): State<AppState>,
    query: Result<Query<DayQuery>, QueryRejection>,
) -> Result<Json<DashboardDto>, ApiError> {
    let Query(DayQuery { day }) =
        query.map_err(|_| ApiError::BadRequest("day is a day written YYYY-MM-DD".into()))?;
    let day = match day.as_deref() {
        Some(text) => parse_day("day", text)?,
        None => clock::now().date(),
    };
    let shop = state.shop_id;
    let read = state.blocking(move |c| service::read(c, shop, day)).await?;
    Ok(Json(DashboardDto::try_from(read)?))
}
