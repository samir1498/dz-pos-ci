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

/// The claim drives its own transaction, so it owns ending it. A path
/// that returns without a COMMIT or a ROLLBACK leaves the connection
/// inside a transaction, and the API holds one connection for the life
/// of the process: every write after it would be refused for starting a
/// transaction inside a transaction, until somebody restarted the till.
///
/// A refusal is the reachable case and is what this holds. The other
/// one, a COMMIT that itself fails, is argued in the code rather than
/// proven here: forcing it wants a deferred constraint or a full disk,
/// neither of which this file can arrange around a function that takes a
/// connection it did not open.
#[test]
fn a_refused_claim_gives_the_connection_back() {
    let (_dir, mut conn) = crate::repos::testdb::open();
    let shop = crate::repos::testdb::SHOP;
    let owner = crate::repos::testdb::OWNER;
    let t = create_pairing_token(&mut conn, shop, owner, noon()).unwrap();

    assert!(matches!(
        claim_pairing_token(&mut conn, shop, t.expose(), later(61), "Phone"),
        Err(CoreError::AuthRefused)
    ));

    // The next transaction on the same connection is the check: BEGIN
    // inside a live transaction is an error, so this fails if the
    // refusal walked away from one.
    conn.batch_execute("BEGIN IMMEDIATE")
        .expect("the refused claim left no transaction open");
    conn.batch_execute("ROLLBACK").unwrap();
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
