//! QR pairing (M6 T2). The desktop shows a QR that carries a 60s single-use
//! pairing token; the phone trades it for a long-lived device token.
//!
//! The QR creation is gated `ManageUsers` (owner|manager), the claim is open
//! inside the launch token but outside any session — the phone has no session
//! yet, only the QR it just scanned.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::Deserialize;

use crate::dto::{DeviceTokenDto, PairedDeviceDto, PairingQrDto};
use crate::error::ApiError;
use crate::session::CurrentUser;
use crate::AppState;

/// The QR the desktop shows: the pairing token the phone will scan.
pub async fn create_qr(
    State(state): State<AppState>,
    who: CurrentUser,
) -> Result<Json<PairingQrDto>, ApiError> {
    let shop = state.shop_id;
    let actor = who.id;
    let now = Utc::now().naive_utc();
    let token = state
        .blocking(move |c| dzpos_core::services::pairing::create_pairing_token(c, shop, actor, now))
        .await?;
    Ok(Json(PairingQrDto {
        pairing_token: token.expose().to_string(),
        expires_in_seconds: dzpos_core::services::pairing::PAIRING_TTL_SECONDS,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ClaimBody {
    pub pairing_token: String,
    pub device_name: String,
}

/// The phone trades the QR's pairing token for a device token that lives
/// until revoked. No session, only the launch token — the phone has not yet
/// been paired.
pub async fn claim(
    State(state): State<AppState>,
    body: Result<Json<ClaimBody>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<DeviceTokenDto>), ApiError> {
    let Json(body) = body.map_err(ApiError::from)?;
    let shop = state.shop_id;
    let now = Utc::now().naive_utc();
    let pairing_token = body.pairing_token.clone();
    let device_name = body.device_name.clone();
    let (token, _row) = state
        .blocking(move |c| {
            dzpos_core::services::pairing::claim_pairing_token(
                c,
                shop,
                &pairing_token,
                now,
                &device_name,
            )
        })
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(DeviceTokenDto {
            device_token: token.expose().to_string(),
        }),
    ))
}

/// The settings screen lists paired phones (M6 T4). Owner|manager only, same
/// gate as the QR.
pub async fn list_devices(
    State(state): State<AppState>,
    _who: CurrentUser,
) -> Result<Json<Vec<PairedDeviceDto>>, ApiError> {
    let shop = state.shop_id;
    let rows = state
        .blocking(move |c| dzpos_core::services::pairing::list_devices(c, shop))
        .await?;
    Ok(Json(rows.into_iter().map(PairedDeviceDto::from).collect()))
}

/// Revoke a paired phone from settings (M6 T4). Owner|manager only.
pub async fn revoke_device(
    State(state): State<AppState>,
    who: CurrentUser,
    id: Result<axum::extract::Path<i32>, axum::extract::rejection::PathRejection>,
) -> Result<Json<PairedDeviceDto>, ApiError> {
    let axum::extract::Path(id) =
        id.map_err(|_| ApiError::BadRequest("the id in the path is not a number".into()))?;
    let shop = state.shop_id;
    let actor = who.id;
    let now = Utc::now().naive_utc();
    let row = state
        .blocking(move |c| dzpos_core::services::pairing::revoke_device(c, shop, actor, id, now))
        .await?;
    Ok(Json(PairedDeviceDto::from(row)))
}
