//! The settings screen's routes. The store block is replaced whole; the
//! régime is appended with the day it applies from, so a document issued
//! under the old one keeps reading it (core, services::settings).

use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::Json;
use chrono::{NaiveDateTime, NaiveTime};
use dzpos_core::error::CoreError;
use dzpos_core::models::shop::StoreBlock;
use dzpos_core::money::Bps;
use dzpos_core::print::FactureLayout;
use dzpos_core::services::clock;
use dzpos_core::services::{preferences, settings, shops};

use crate::dto::{
    parse_day, DiscountThresholdChangeDto, FactureLayoutChoiceDto, FactureLayoutDto,
    RegimeChangeDto, SettingsDto, StoreDto, ThemeChoiceDto,
};
use crate::error::ApiError;
use crate::session::CurrentUser;
use crate::AppState;

/// This moment on the shop's calendar. The offset itself lives in the core
/// (services::clock): a document's issued_at reads the same clock, and the
/// backups route stamps its copies with it.
pub(crate) fn now() -> NaiveDateTime {
    clock::now()
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
        theme: preferences::theme(conn, shop)?.map(Into::into),
        facture_layout: preferences::facture_layout(conn, shop)?.into(),
        facture_layouts: FactureLayout::ALL.map(FactureLayoutDto::from).to_vec(),
        discount_threshold_bps: settings::discount_threshold_as_of(conn, shop, at)?.as_u32(),
    })
}

pub async fn read(State(state): State<AppState>) -> Result<Json<SettingsDto>, ApiError> {
    let shop = state.shop_id;
    let all = state.blocking(move |c| read_all(c, shop)).await?;
    Ok(Json(all))
}

pub async fn update_store(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<StoreDto>, JsonRejection>,
) -> Result<Json<StoreDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let block = StoreBlock::from(dto);
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| shops::update_store(c, shop, user, block))
        .await?;
    Ok(Json(StoreDto::from(after)))
}

/// Records the shop's theme, or forgets it when the body carries `null`,
/// which puts the app back on Comptoir, the default (the machine's own
/// light or dark preference is not consulted).
/// Answers the whole settings page for the reason `change_regime` does: the
/// screen should read one shape back, not patch its own copy.
pub async fn set_theme(
    State(state): State<AppState>,
    body: Result<Json<ThemeChoiceDto>, JsonRejection>,
) -> Result<Json<SettingsDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let chosen = dto.theme.map(Into::into);
    let shop = state.shop_id;
    let all = state
        .blocking(move |c| {
            preferences::set_theme(c, shop, chosen, now())?;
            read_all(c, shop)
        })
        .await?;
    Ok(Json(all))
}

/// Records the layout the shop's factures print in.
///
/// Answers the whole settings page for the reason `set_theme` does: the
/// screen should read one shape back rather than patch its own copy.
///
/// No `null` arm, unlike the theme. A shop always prints in some layout, and
/// "none" would be `standard` under another name.
pub async fn set_facture_layout(
    State(state): State<AppState>,
    body: Result<Json<FactureLayoutChoiceDto>, JsonRejection>,
) -> Result<Json<SettingsDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let chosen = FactureLayout::from(dto.facture_layout);
    let shop = state.shop_id;
    let all = state
        .blocking(move |c| {
            preferences::set_facture_layout(c, shop, chosen, now())?;
            read_all(c, shop)
        })
        .await?;
    Ok(Json(all))
}

/// Answers the whole settings page again: the change may be current or
/// planned depending on its date, and the screen should not have to guess
/// which.
pub async fn change_regime(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<RegimeChangeDto>, JsonRejection>,
) -> Result<Json<SettingsDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let from = parse_day("valid_from", &dto.valid_from)?.and_time(NaiveTime::MIN);
    let regime = dto.regime.into();
    let shop = state.shop_id;
    let user = who.id;
    let all = state
        .blocking(move |c| {
            settings::set_regime(c, shop, user, regime, from)?;
            read_all(c, shop)
        })
        .await?;
    Ok(Json(all))
}

/// The discount a cashier may give without asking anyone. Answers the whole
/// settings page for the reason `change_regime` does, and is dated for the
/// same reason: a sale refused in March is read against March's threshold,
/// not against the one the shop moved to in April.
pub async fn set_discount_threshold(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<DiscountThresholdChangeDto>, JsonRejection>,
) -> Result<Json<SettingsDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let from = parse_day("valid_from", &dto.valid_from)?.and_time(NaiveTime::MIN);
    let threshold = Bps::new(dto.threshold_bps).map_err(|_| {
        CoreError::validation(
            "threshold_bps",
            "a discount threshold is a share of a basket, so at most 100 %",
        )
    })?;
    let shop = state.shop_id;
    let user = who.id;
    let all = state
        .blocking(move |c| {
            settings::set_discount_threshold(c, shop, user, threshold, from)?;
            read_all(c, shop)
        })
        .await?;
    Ok(Json(all))
}
