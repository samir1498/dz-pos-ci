//! QR pairing (M6 T2). One desktop serves, the phone finds it via mDNS and
//! shows a QR that carries a 60-second single-use pairing token; the phone
//! trades that for a long-lived device token the desktop stores as
//! `paired_devices`. Every phone request shows the device token the way the
//! webview shows the launch token, and every user action still shows a
//! session — the device says which phone, the session says which person.

use argon2::password_hash::rand_core::{OsRng, RngCore};
use chrono::{Duration, NaiveDateTime};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::pairing::{DeviceToken, PairedDeviceRow, PairingToken};
use crate::repos::pairing as repo;

/// How long a QR lives: 60 seconds, single-use, as the mockup shows.
const PAIRING_TTL_SECONDS: i64 = 60;
const TOKEN_BYTES: usize = 32;

fn mint_hex() -> Result<String, CoreError> {
    let mut bytes = [0u8; TOKEN_BYTES];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| CoreError::from(std::io::Error::other("no randomness")))?;
    let mut hex = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(hex, "{b:02x}");
    }
    Ok(hex)
}

pub fn token_digest(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hex = String::with_capacity(64);
    for b in Sha256::digest(token.as_bytes()) {
        use std::fmt::Write;
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

/// The desktop (owner|manager) asks for a QR to show. Returns the secret the
/// QR carries; the row stores its hash and the expiry. Permission is gated in
/// `crates/api/src/gates.rs` (`ManageUsers`), not here.
pub fn create_pairing_token(
    conn: &mut SqliteConnection,
    shop_id: i32,
    created_by: i32,
    now: NaiveDateTime,
) -> Result<PairingToken, CoreError> {
    let token = PairingToken::new(mint_hex()?);
    let hash = token_digest(token.expose());
    let expires_at = now + Duration::seconds(PAIRING_TTL_SECONDS);
    let write = crate::models::pairing::PairingTokenWrite {
        shop_id,
        token_hash: hash,
        created_at: now,
        expires_at,
        used_at: None,
        created_by,
    };
    repo::insert_pairing_token(conn, &write)?;
    Ok(token)
}

/// The phone trades a pairing token for a device token. Single-use and
/// 60s: a token past its expiry, already used, or from another shop is
/// `AuthRefused` — indistinguishable from "no such token" so a scanner
/// learns nothing about which shop ever issued what.
pub fn claim_pairing_token(
    conn: &mut SqliteConnection,
    shop_id: i32,
    pairing_token: &str,
    now: NaiveDateTime,
    device_name: &str,
) -> Result<(DeviceToken, PairedDeviceRow), CoreError> {
    let name = device_name.trim();
    if name.is_empty() || name.chars().count() > 200 {
        return Err(CoreError::validation(
            "name",
            "device name is empty or too long",
        ));
    }
    let hash = token_digest(pairing_token);
    let row = repo::pairing_by_hash(conn, shop_id, &hash)?.ok_or(CoreError::AuthRefused)?;
    if row.used_at.is_some() {
        return Err(CoreError::AuthRefused);
    }
    if now > row.expires_at {
        return Err(CoreError::AuthRefused);
    }
    // Mark used first, inside the same transaction the device is inserted
    // in, so a concurrent claim cannot both succeed.
    conn.transaction::<_, CoreError, _>(|conn| {
        repo::mark_pairing_used(conn, row.id, now)?;
        let device_token = DeviceToken::new(mint_hex()?);
        let device_hash = token_digest(device_token.expose());
        let write = crate::models::pairing::PairedDeviceWrite {
            shop_id,
            token_hash: device_hash,
            name: name.to_string(),
            created_at: now,
            last_seen_at: Some(now),
            revoked_at: None,
            created_by: row.created_by,
        };
        let device_row = repo::insert_device(conn, &write)?;
        Ok((device_token, device_row))
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use chrono::NaiveDate;

    fn noon() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 9, 14)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    fn later(secs: i64) -> NaiveDateTime {
        noon() + Duration::seconds(secs)
    }

    #[test]
    fn a_pairing_token_is_sixty_four_hex_and_single_use() {
        let (_dir, mut conn) = crate::repos::testdb::open();
        let shop = crate::repos::testdb::SHOP;
        let owner = crate::repos::testdb::OWNER;
        let t = create_pairing_token(&mut conn, shop, owner, noon()).unwrap();
        assert_eq!(t.expose().len(), 64);
        // First claim succeeds.
        let (dev, _) = claim_pairing_token(&mut conn, shop, t.expose(), noon(), "Phone").unwrap();
        assert_eq!(dev.expose().len(), 64);
        // Second claim with same token is refused, indistinguishable from expiry.
        assert!(matches!(
            claim_pairing_token(&mut conn, shop, t.expose(), noon(), "Phone2"),
            Err(CoreError::AuthRefused)
        ));
    }

    #[test]
    fn a_pairing_token_past_sixty_seconds_is_refused() {
        let (_dir, mut conn) = crate::repos::testdb::open();
        let shop = crate::repos::testdb::SHOP;
        let owner = crate::repos::testdb::OWNER;
        let t = create_pairing_token(&mut conn, shop, owner, noon()).unwrap();
        assert!(matches!(
            claim_pairing_token(&mut conn, shop, t.expose(), later(61), "Phone"),
            Err(CoreError::AuthRefused)
        ));
        assert!(claim_pairing_token(&mut conn, shop, t.expose(), later(60), "Phone").is_ok());
    }

    #[test]
    fn a_pairing_token_from_another_shop_is_not_found() {
        let (_dir, mut conn) = crate::repos::testdb::open();
        let shop = crate::repos::testdb::SHOP;
        let owner = crate::repos::testdb::OWNER;
        let t = create_pairing_token(&mut conn, shop, owner, noon()).unwrap();
        assert!(matches!(
            claim_pairing_token(&mut conn, 2, t.expose(), noon(), "Phone"),
            Err(CoreError::AuthRefused)
        ));
    }
}
