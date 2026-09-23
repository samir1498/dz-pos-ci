// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use dzpos_core::models::audit::AuditEntry;
use dzpos_core::services::audit::EntryWithUser;

/// The moment travels through this conversion untouched. It used to be
/// shifted here, because the column held UTC and the screen wanted the
/// shop's calendar; the row is stamped from the shop clock now
/// (`2026-09-12-000013_audit_log_shop_clock`) and a shift put back by
/// hand would print every row an hour late. The seconds are also the
/// last of it: `clock::now()` carries nanoseconds and the column keeps
/// them, so the format string is what stands between the owner's screen
/// and a date ending in nine digits.
#[test]
fn the_moment_is_printed_as_it_stands_to_the_second() {
    let entry = AuditEntry {
        id: 1,
        shop_id: 1,
        user_id: 1,
        action: "product.update".to_string(),
        entity: "product".to_string(),
        entity_id: Some(7),
        before: None,
        after: None,
        created_at: chrono::NaiveDate::from_ymd_opt(2026, 9, 12)
            .unwrap()
            .and_hms_nano_opt(0, 30, 0, 123_456_789)
            .unwrap(),
    };
    let dto = AuditEntryDto::from(EntryWithUser {
        entry,
        user_name: "Propriétaire".to_string(),
    });
    assert_eq!(dto.created_at, "2026-09-12 00:30:00");
}
