//! The dashboard's one route. It translates: every figure on it is decided by
//! `dzpos_core::services::dashboard`.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::Json;
use dzpos_core::services::{clock, dashboard as service};
use serde::Deserialize;

use crate::dto::{parse_day, DashboardDto, DashboardSeriesDto};
use crate::error::ApiError;
use crate::AppState;

/// How many days a chart covers when the screen names no number. Thirty is
/// what the dashboard draws (features.md §1); the core is what refuses a
/// window of nothing or of more than a year.
const DEFAULT_DAYS: u32 = 30;

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

/// The chart's window: the day it ends on and how many days it covers. Both
/// optional, so `/dashboard/series` alone is the shop's last thirty days.
///
/// `days` is a string and not a number on purpose: serde's own refusal of a
/// bad integer is a rejection this route would answer as "day is a day",
/// which is the wrong sentence. Parsed below, it is refused for what it is.
#[derive(Deserialize)]
pub struct SeriesQuery {
    #[serde(default)]
    day: Option<String>,
    #[serde(default)]
    days: Option<String>,
}

/// The last `days` days ending on `day`, each on its own and folded into
/// weeks. Every figure is `dashboard::series`, which reads the same rules the
/// two columns above are read with.
pub async fn series(
    State(state): State<AppState>,
    query: Result<Query<SeriesQuery>, QueryRejection>,
) -> Result<Json<DashboardSeriesDto>, ApiError> {
    let Query(SeriesQuery { day, days }) = query.map_err(|_| {
        ApiError::BadRequest("day is a day written YYYY-MM-DD and days is a whole number".into())
    })?;
    let day = match day.as_deref() {
        Some(text) => parse_day("day", text)?,
        None => clock::now().date(),
    };
    let days = match days.as_deref() {
        Some(text) => text
            .parse::<u32>()
            .map_err(|_| ApiError::BadRequest("days is a whole number of days".into()))?,
        None => DEFAULT_DAYS,
    };
    let shop = state.shop_id;
    let read = state
        .blocking(move |c| service::series(c, shop, day, days))
        .await?;
    Ok(Json(DashboardSeriesDto::from(read)))
}
