//! The settings screen's routes. The store block is replaced whole; the
//! régime is appended with the day it applies from, so a document issued
//! under the old one keeps reading it (core, services::settings).

use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::Json;
use chrono::{DateTime, FixedOffset, NaiveDateTime, NaiveTime, Utc};
use dzpos_core::models::shop::StoreBlock;
use dzpos_core::services::{settings, shops};

use crate::dto::{parse_day, RegimeChangeDto, SettingsDto, StoreDto};
use crate::error::ApiError;
use crate::AppState;

/// The shop's clock: Algeria, UTC+1, no daylight saving. A régime changes
/// on a calendar day and the day is the shop's, so a change dated
/// 1 January is in force at 00:30 in Algiers, when UTC still reads
/// 31 December.
const SHOP_UTC_OFFSET_SECONDS: i32 = 3600;

/// `utc` read on the shop's calendar. Separate from `now()` so the wall
/// clock never enters a test.
fn shop_time(utc: DateTime<Utc>) -> NaiveDateTime {
    match FixedOffset::east_opt(SHOP_UTC_OFFSET_SECONDS) {
        Some(offset) => utc.with_timezone(&offset).naive_local(),
        // 3600 is inside the range east_opt accepts, so this arm is never
        // taken; UTC is the honest fallback rather than a panic.
        None => utc.naive_utc(),
    }
}

fn now() -> NaiveDateTime {
    shop_time(Utc::now())
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
    let from = parse_day("valid_from", &dto.valid_from)?.and_time(NaiveTime::MIN);
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

#[cfg(test)]
mod shop_time_tests {
    use super::shop_time;
    use chrono::{NaiveDate, TimeZone, Utc};

    #[test]
    fn the_first_hour_of_the_algerian_day_is_already_the_new_day() {
        let local = Utc
            .with_ymd_and_hms(2026, 12, 31, 23, 30, 0)
            .single()
            .map(shop_time);
        assert_eq!(local.map(|l| l.date()), NaiveDate::from_ymd_opt(2027, 1, 1));
        assert_eq!(
            local.map(|l| l.format("%H:%M").to_string()),
            Some("00:30".to_string())
        );
    }

    #[test]
    fn the_rest_of_the_day_reads_the_same_date_as_utc() {
        let local = Utc
            .with_ymd_and_hms(2026, 6, 15, 12, 0, 0)
            .single()
            .map(shop_time);
        assert_eq!(
            local.map(|l| l.date()),
            NaiveDate::from_ymd_opt(2026, 6, 15)
        );
    }
}
