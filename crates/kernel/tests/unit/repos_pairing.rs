// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::models::pairing::PairingTokenWrite;
use crate::repos::testdb;
use chrono::NaiveDate;

fn noon() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, 14)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}

/// The conditional flip, directly: the first mark flips, the second
/// finds nothing unused and answers false. This pins the `used_at IS
/// NULL` filter the concurrent-claim test passes through the earlier
/// `used_at.is_some()` check.
#[test]
fn marking_used_twice_flips_once() {
    let (_dir, mut conn) = testdb::open();
    let row = insert_pairing_token(
        &mut conn,
        &PairingTokenWrite {
            shop_id: testdb::SHOP,
            token_hash: "aa".to_string(),
            created_at: noon(),
            expires_at: noon(),
            used_at: None,
            created_by: testdb::OWNER,
        },
    )
    .unwrap();
    assert!(mark_pairing_used(&mut conn, row.id, noon()).unwrap());
    assert!(!mark_pairing_used(&mut conn, row.id, noon()).unwrap());
}
