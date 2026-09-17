//! Pairing a phone: the QR, the token it trades for, and the devices paired.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// The QR the desktop shows for the phone to scan (M6 T2). The token is
/// 64 hex, 60s single-use; the phone trades it for a device token.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PairingQrDto.ts")]
pub struct PairingQrDto {
    pub pairing_token: String,
    pub expires_in_seconds: i64,
}

/// The long-lived device token the phone keeps after pairing (M6 T2). Like
/// `SessionDto.token`, 64 hex, stored as hash, shown once.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "DeviceTokenDto.ts")]
pub struct DeviceTokenDto {
    pub device_token: String,
}

/// A paired phone as the settings screen lists it (M6 T4). The token hash
/// never leaves the server; the row is what the screen revokes.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PairedDeviceDto.ts")]
pub struct PairedDeviceDto {
    pub id: i32,
    pub name: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
    pub created_by: i32,
}

impl From<dzpos_core::models::pairing::PairedDeviceRow> for PairedDeviceDto {
    fn from(row: dzpos_core::models::pairing::PairedDeviceRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            created_at: row.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            revoked_at: row
                .revoked_at
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            created_by: row.created_by,
        }
    }
}
