//! The settings screen's routes. The store block is replaced whole; the
//! régime is appended with the day it applies from, so a document issued
//! under the old one keeps reading it (core, services::settings).

use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::Json;
use chrono::{NaiveDateTime, NaiveTime};
use dzpos_core::error::CoreError;
use dzpos_core::models::shop::StoreBlock;
#[cfg(feature = "retail")]
use dzpos_core::money::Bps;
use dzpos_core::print::{FactureLayout, ThermalMode};
use dzpos_core::services::clock;
#[cfg(feature = "retail")]
use dzpos_core::services::discount_threshold;
use dzpos_core::services::{preferences, settings, shops};

#[cfg(feature = "retail")]
use crate::dto::DiscountThresholdChangeDto;
use crate::dto::{
    parse_day, FactureLayoutChoiceDto, FactureLayoutDto, PrintLangChoiceDto, RegimeChangeDto,
    SettingsDto, StoreDto, ThemeChoiceDto, ThermalModeChoiceDto,
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
        print_lang: preferences::print_lang(conn, shop)?.map(Into::into),
        thermal_mode: preferences::thermal_mode(conn, shop)?.into(),
        #[cfg(feature = "retail")]
        discount_threshold_bps: discount_threshold::discount_threshold_as_of(conn, shop, at)?
            .as_u32(),
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

/// Records the language every fiscal paper prints in, or forgets it when the
/// body carries `null`, which puts every fiscal paper back on the till's own
/// language: the `null` arm exists for the same reason the theme's does, a
/// shop that chose one needs a way back. The six fiscal-paper routes read it
/// back through `preferences::print_lang_for`.
///
/// Answers the whole settings page for the reason `set_theme` does.
pub async fn set_print_lang(
    State(state): State<AppState>,
    body: Result<Json<PrintLangChoiceDto>, JsonRejection>,
) -> Result<Json<SettingsDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    // A body with no `print_lang` at all is refused rather than read as the
    // shop forgetting its choice. See `PrintLangChoiceDto`'s own doc: serde
    // would otherwise hand both the same answer.
    let Some(answer) = dto.print_lang else {
        return Err(ApiError::Request(CoreError::validation(
            "print_lang",
            "name the language to print in, or null to follow the till",
        )));
    };
    let chosen = answer.map(Into::into);
    let shop = state.shop_id;
    let all = state
        .blocking(move |c| {
            preferences::set_print_lang(c, shop, chosen, now())?;
            read_all(c, shop)
        })
        .await?;
    Ok(Json(all))
}

/// Records which ESC/POS path the shop's thermal head is sent.
///
/// Answers the whole settings page for the reason `set_theme` does.
///
/// No `null` arm, the same as the facture layout: a head is always on one
/// of the two paths. What the choice does not reach is Arabic, which is
/// drawn whatever is stored here because no single-byte table a cheap head
/// carries has Arabic in it — the rule lives in the core
/// (`ThermalMode::for_lang`) and the printing panel says so in all three
/// languages, so an owner is not left to find it on paper.
pub async fn set_thermal_mode(
    State(state): State<AppState>,
    body: Result<Json<ThermalModeChoiceDto>, JsonRejection>,
) -> Result<Json<SettingsDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let chosen = ThermalMode::from(dto.thermal_mode);
    let shop = state.shop_id;
    let all = state
        .blocking(move |c| {
            preferences::set_thermal_mode(c, shop, chosen, now())?;
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
/// not against the one the shop moved to in April. Retail-only (S5): the
/// discount a cashier may give is a shop concept, and the kernel's own
/// `POST /settings/discount-threshold` gate row is gated the same way.
#[cfg(feature = "retail")]
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
            discount_threshold::set_discount_threshold(c, shop, user, threshold, from)?;
            read_all(c, shop)
        })
        .await?;
    Ok(Json(all))
}
