//! QR pairing (M6 T2). One desktop serves, the phone finds it via mDNS and
//! shows a QR that carries a 60-second single-use pairing token; the phone
//! trades that for a long-lived device token the desktop stores as
//! `paired_devices`. Every phone request shows the device token the way the
//! webview shows the launch token, and every user action still shows a
//! session — the device says which phone, the session says which person.

use argon2::password_hash::rand_core::{OsRng, RngCore};
use chrono::{Duration, NaiveDateTime};
use diesel::connection::{Connection, SimpleConnection};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::pairing::{DeviceToken, PairedDeviceRow, PairingToken};
use crate::repos::pairing as repo;

/// How long a QR lives: 60 seconds, single-use, as the mockup shows.
pub const PAIRING_TTL_SECONDS: i64 = 60;
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
    // BEGIN IMMEDIATE, not diesel's deferred begin: two claims racing one
    // QR both take a read lock under a deferred begin and neither upgrade
    // ever clears, so SQLite answers BUSY instead of serializing them.
    // Immediate takes the write lock up front; the loser waits out the
    // winner (busy_timeout, `db::open`) and then reads `used_at` set.
    // The flip stays conditional on top of that, so a zero-row flip is
    // already-claimed even if the read ever races again — answered the same
    // indistinguishable way as every other refusal here.
    conn.batch_execute("BEGIN IMMEDIATE")?;
    let claimed = (|| -> Result<(DeviceToken, PairedDeviceRow), CoreError> {
        let row = repo::pairing_by_hash(conn, shop_id, &hash)?.ok_or(CoreError::AuthRefused)?;
        if row.used_at.is_some() {
            return Err(CoreError::AuthRefused);
        }
        if now > row.expires_at {
            return Err(CoreError::AuthRefused);
        }
        if !repo::mark_pairing_used(conn, row.id, now)? {
            return Err(CoreError::AuthRefused);
        }
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
        // Inside the same transaction as the pairing it records, like every
        // other audited change (`services::audit`). The shop's only record
        // that this phone was ever trusted: the revoke beside it was
        // audited from the first day, the pairing was not, so a device list
        // could gain a row nobody could account for.
        crate::services::audit::record(
            conn,
            shop_id,
            row.created_by,
            crate::services::audit::Change {
                action: crate::services::audit::ACTION_DEVICE_PAIRED,
                entity: "paired_device",
                entity_id: Some(device_row.id),
                before: None,
                after: Some(serde_json::json!({ "name": device_row.name }).to_string()),
            },
        )?;
        Ok((device_token, device_row))
    })();
    match claimed {
        Ok(out) => {
            conn.batch_execute("COMMIT")?;
            Ok(out)
        }
        Err(refused) => {
            let _ = conn.batch_execute("ROLLBACK");
            Err(refused)
        }
    }
}

/// The phone a request claims to come from, for the API's device gate. A
/// token nobody issued and a revoked one are the same `AuthRefused` the
/// claim path answers, so a scanner learns nothing either way. A live token
/// slides `last_seen_at` forward — "which phones are still around" is read
/// off that column, not off the audit log.
pub fn device_for_request(
    conn: &mut SqliteConnection,
    shop_id: i32,
    token_hash: &str,
    now: NaiveDateTime,
) -> Result<PairedDeviceRow, CoreError> {
    let row = repo::device_by_hash(conn, shop_id, token_hash)?.ok_or(CoreError::AuthRefused)?;
    if row.revoked_at.is_some() {
        return Err(CoreError::AuthRefused);
    }
    repo::touch_device_last_seen(conn, row.id, now)?;
    // The row as it stands now that the touch landed, not as the lookup
    // found it a statement earlier.
    Ok(PairedDeviceRow {
        last_seen_at: Some(now),
        ..row
    })
}

pub fn list_devices(
    conn: &mut SqliteConnection,
    shop_id: i32,
) -> Result<Vec<PairedDeviceRow>, CoreError> {
    repo::list_devices(conn, shop_id)
}

pub fn revoke_device(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    device_id: i32,
    now: NaiveDateTime,
) -> Result<PairedDeviceRow, CoreError> {
    // One transaction for the change and the row that records it: a crash
    // between the two would leave a phone revoked with nothing to say who
    // did it, and `gates.rs` calls that row the only record the shop has.
    conn.transaction(|conn| {
        let row = repo::revoke_device(conn, shop_id, device_id, now)?;
        crate::services::audit::record(
            conn,
            shop_id,
            actor_id,
            crate::services::audit::Change {
                action: crate::services::audit::ACTION_DEVICE_REVOKED,
                entity: "paired_device",
                entity_id: Some(device_id),
                before: None,
                after: None,
            },
        )?;
        Ok(row)
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
    fn two_concurrent_claims_pair_only_one_phone() {
        use std::sync::Barrier;
        let (dir, mut first) = crate::repos::testdb::open();
        let path = dir.path().join("t.db");
        let mut second = crate::db::open(&path).unwrap();
        let shop = crate::repos::testdb::SHOP;
        let owner = crate::repos::testdb::OWNER;
        let secret = create_pairing_token(&mut first, shop, owner, noon())
            .unwrap()
            .expose()
            .to_string();
        let gate = Barrier::new(2);
        let (a, b) = std::thread::scope(|s| {
            let gate = &gate;
            let one = s.spawn(|| {
                gate.wait();
                match claim_pairing_token(&mut first, shop, &secret, noon(), "Phone A") {
                    Ok(_) => "paired",
                    Err(CoreError::AuthRefused) => "refused",
                    Err(other) => panic!("loser must fail closed, not {other:?}"),
                }
            });
            let two = s.spawn(|| {
                gate.wait();
                match claim_pairing_token(&mut second, shop, &secret, noon(), "Phone B") {
                    Ok(_) => "paired",
                    Err(CoreError::AuthRefused) => "refused",
                    Err(other) => panic!("loser must fail closed, not {other:?}"),
                }
            });
            (one.join().unwrap(), two.join().unwrap())
        });
        assert!(
            (a == "paired") ^ (b == "paired"),
            "exactly one of two concurrent claims pairs: {a:?} vs {b:?}"
        );
        let mut check = crate::db::open(&path).unwrap();
        assert_eq!(list_devices(&mut check, shop).unwrap().len(), 1);
    }

    #[test]
    fn a_live_device_passes_and_slides_last_seen_forward() {
        let (_dir, mut conn) = crate::repos::testdb::open();
        let shop = crate::repos::testdb::SHOP;
        let owner = crate::repos::testdb::OWNER;
        let t = create_pairing_token(&mut conn, shop, owner, noon()).unwrap();
        let (dev, _) = claim_pairing_token(&mut conn, shop, t.expose(), noon(), "Phone").unwrap();
        let hash = token_digest(dev.expose());
        let seen = device_for_request(&mut conn, shop, &hash, later(30)).unwrap();
        assert_eq!(seen.last_seen_at, Some(later(30)));
        // And the column, not just the returned struct: the touch landed.
        let stored = repo::device_by_hash(&mut conn, shop, &hash)
            .unwrap()
            .unwrap();
        assert_eq!(stored.last_seen_at, Some(later(30)));
    }

    #[test]
    fn an_unknown_device_and_a_revoked_one_are_refused_alike() {
        let (_dir, mut conn) = crate::repos::testdb::open();
        let shop = crate::repos::testdb::SHOP;
        let owner = crate::repos::testdb::OWNER;
        assert!(matches!(
            device_for_request(&mut conn, shop, &"0".repeat(64), noon()),
            Err(CoreError::AuthRefused)
        ));
        let t = create_pairing_token(&mut conn, shop, owner, noon()).unwrap();
        let (dev, row) = claim_pairing_token(&mut conn, shop, t.expose(), noon(), "Phone").unwrap();
        revoke_device(&mut conn, shop, owner, row.id, noon()).unwrap();
        let hash = token_digest(dev.expose());
        assert!(matches!(
            device_for_request(&mut conn, shop, &hash, later(30)),
            Err(CoreError::AuthRefused)
        ));
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
