use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::schema::{paired_devices, pairing_tokens};

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = pairing_tokens)]
pub struct PairingTokenRow {
    pub id: i32,
    pub shop_id: i32,
    pub token_hash: String,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub used_at: Option<NaiveDateTime>,
    pub created_by: i32,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = pairing_tokens)]
pub struct PairingTokenWrite {
    pub shop_id: i32,
    pub token_hash: String,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub used_at: Option<NaiveDateTime>,
    pub created_by: i32,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = paired_devices)]
pub struct PairedDeviceRow {
    pub id: i32,
    pub shop_id: i32,
    pub token_hash: String,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub last_seen_at: Option<NaiveDateTime>,
    pub revoked_at: Option<NaiveDateTime>,
    pub created_by: i32,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = paired_devices)]
pub struct PairedDeviceWrite {
    pub shop_id: i32,
    pub token_hash: String,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub last_seen_at: Option<NaiveDateTime>,
    pub revoked_at: Option<NaiveDateTime>,
    pub created_by: i32,
}

/// The pairing secret, like `SessionToken`: 32 random bytes as 64 hex, never logged.
#[derive(Clone)]
pub struct PairingToken(pub String);

impl PairingToken {
    pub(crate) fn new(hex: String) -> Self {
        Self(hex)
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}

#[derive(Clone)]
pub struct DeviceToken(pub String);

impl DeviceToken {
    pub(crate) fn new(hex: String) -> Self {
        Self(hex)
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}
