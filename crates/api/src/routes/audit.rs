//! The owner's audit log screen (M4 T7, features.md §5). One read route:
//! the log records what every other service already wrote, and this
//! translates it, the way every other file in this folder does.
//!
//! No write route exists here and none is coming. Nothing on the screen
//! this feeds edits or deletes a row: `services::audit::record` is the only
//! way one is ever written, and it is called from inside the transaction of
//! the change it is recording.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::Json;
use dzpos_core::services::audit::{self, Filter};
use serde::Deserialize;

use crate::dto::{parse_day, AuditLogDto};
use crate::error::ApiError;
use crate::AppState;

/// The three filters and the page, all optional: a screen opened with
/// nothing chosen reads the whole log's first page, newest first.
#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    user_id: Option<i32>,
    #[serde(default)]
    action: Option<String>,
    /// `YYYY-MM-DD` on the shop's calendar, not the UTC the column stores
    /// (`services::audit::day_range_utc`).
    #[serde(default)]
    day: Option<String>,
    /// 1-based; left out or under 1 is the first page.
    #[serde(default)]
    page: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<AuditLogDto>, ApiError> {
    let Query(ListQuery {
        user_id,
        action,
        day,
        page,
    }) = query.map_err(|_| {
        ApiError::BadRequest("user_id and page are numbers, day is YYYY-MM-DD".into())
    })?;
    let day = day.as_deref().map(|d| parse_day("day", d)).transpose()?;
    let filter = Filter {
        user_id,
        action,
        day,
    };
    let shop = state.shop_id;
    let page_number = page.unwrap_or(1);
    let (found, facets) = state
        .blocking(move |c| audit::read(c, shop, &filter, page_number))
        .await?;
    Ok(Json(AuditLogDto::from_core(found, facets)))
}
