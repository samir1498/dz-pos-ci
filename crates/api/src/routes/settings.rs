//! The settings screen's routes. The store block is replaced whole; the
//! régime is appended with the day it applies from, so a document issued
//! under the old one keeps reading it (core, services::settings).

use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::Json;
use chrono::NaiveDateTime;
use dzpos_core::models::shop::StoreBlock;
use dzpos_core::services::{settings, shops};

use crate::dto::{parse_day, RegimeChangeDto, SettingsDto, StoreDto};
use crate::error::ApiError;
use crate::AppState;

/// "Now" for the régime in force. UTC, the same clock the product rows are
/// stamped with; a régime changes on a calendar day, so the hour Algeria
/// sits ahead of UTC cannot move a change across a document.
fn now() -> NaiveDateTime {
    chrono::Utc::now().naive_utc()
}

fn read_all(
    conn: &mut dzpos_core::db::Conn,
    shop: i32,
) -> Result<SettingsDto, dzpos_core::error::CoreError> {
    let at = now();
    Ok(SettingsDto {
        store: StoreDto::from(shops::get(conn, shop)?),
        regime: settings::regime_current(conn, shop, at)?.into(),
        regime_planned: settings::regime_planned(conn, shop, at)?.map(Into::into),
    })
}

pub async fn read(State(state): State<AppState>) -> Result<Json<SettingsDto>, ApiError> {
    let shop = state.shop_id;
    let all = state.blocking(move |c| read_all(c, shop)).await?;
    Ok(Json(all))
}

pub async fn update_store(
    State(state): State<AppState>,
    body: Result<Json<StoreDto>, JsonRejection>,
) -> Result<Json<StoreDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let block = StoreBlock::from(dto);
    let shop = state.shop_id;
    let after = state
        .blocking(move |c| shops::update_store(c, shop, block))
        .await?;
    Ok(Json(StoreDto::from(after)))
}

/// Answers the whole settings page again: the change may be current or
/// planned depending on its date, and the screen should not have to guess
/// which.
pub async fn change_regime(
    State(state): State<AppState>,
    body: Result<Json<RegimeChangeDto>, JsonRejection>,
) -> Result<Json<SettingsDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let day = parse_day("valid_from", &dto.valid_from)?;
    let from = day
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| ApiError::BadRequest("valid_from is not a day".into()))?;
    let regime = dto.regime.into();
    let shop = state.shop_id;
    let all = state
        .blocking(move |c| {
            settings::set_regime(c, shop, regime, from)?;
            read_all(c, shop)
        })
        .await?;
    Ok(Json(all))
}
