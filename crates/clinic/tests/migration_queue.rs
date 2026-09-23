// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Migration 000020: the `queue_entries` table.
//!
//! Raw INSERTs and UPDATEs only, never the service, for the reason
//! `migration_patients.rs` gives: `services::queue` refuses most of these
//! first, and this file is what holds the table's own CHECKs and its
//! one-live-entry index if the service's checks are ever taken out. The
//! shape tests every table shares and the whole-stack revert live in
//! `crates/retail/tests/migration.rs`.

use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

mod common;

use common::open_temp;

const PATIENT: &str = "0199a0c1-0000-7000-8000-000000000001";
const ENTRY: &str = "0199a0c1-0000-7000-8000-00000000e001";

fn patient(conn: &mut SqliteConnection) {
    diesel::sql_query(format!(
        "INSERT INTO patients (id, shop_id, first_name, last_name, created_at, updated_at) \
         VALUES ('{PATIENT}', 1, 'Amina', 'Benali', '2026-09-23 08:00:00', '2026-09-23 08:00:00')"
    ))
    .execute(conn)
    .unwrap();
}

/// One entry with every column set to something the table takes, and some
/// columns overridden with raw SQL literals.
fn insert(conn: &mut SqliteConnection, overrides: &[(&str, &str)]) -> QueryResult<usize> {
    let mut values = vec![
        ("id", format!("'{ENTRY}'")),
        ("shop_id", "1".to_string()),
        ("patient_id", format!("'{PATIENT}'")),
        ("day", "'2026-09-23'".to_string()),
        ("arrived_at", "'2026-09-23 09:00:00'".to_string()),
        ("created_at", "'2026-09-23 09:00:00'".to_string()),
        ("updated_at", "'2026-09-23 09:00:00'".to_string()),
    ];
    for (column, literal) in overrides {
        match values.iter_mut().find(|(name, _)| name == column) {
            Some(value) => value.1 = (*literal).to_string(),
            None => values.push((column, (*literal).to_string())),
        }
    }
    let names: Vec<&str> = values.iter().map(|(n, _)| *n).collect();
    let literals: Vec<&str> = values.iter().map(|(_, v)| v.as_str()).collect();
    diesel::sql_query(format!(
        "INSERT INTO queue_entries ({}) VALUES ({})",
        names.join(", "),
        literals.join(", ")
    ))
    .execute(conn)
}

fn clear(conn: &mut SqliteConnection) {
    diesel::sql_query("DELETE FROM queue_entries")
        .execute(conn)
        .unwrap();
}

const CALLED: (&str, &str) = ("called_at", "'2026-09-23 09:10:00'");
const SEEN: (&str, &str) = ("seen_at", "'2026-09-23 09:30:00'");
const LEFT: (&str, &str) = ("left_at", "'2026-09-23 09:20:00'");

/// Each refused row beside one the table takes, so a CHECK that refused
/// everything would fail here too.
#[test]
fn each_row_the_file_says_it_may_not_hold_is_refused() {
    let (_dir, mut conn) = open_temp();
    patient(&mut conn);
    let refused: [&[(&str, &str)]; 9] = [
        &[("id", "'short'")],
        &[("id", "42")],
        &[
            ("day", "'2026-02-30'"),
            ("arrived_at", "'2026-03-02 09:00:00'"),
        ],
        &[("day", "'2026-9-23'")],
        &[("day", "NULL")],
        // The day is the arrival's own.
        &[("day", "'2026-09-22'")],
        &[("arrived_at", "NULL")],
        // Seen needs called, and seen and left are one or the other.
        &[SEEN],
        &[CALLED, SEEN, LEFT],
    ];
    for row in refused {
        assert!(
            insert(&mut conn, row).is_err(),
            "queue_entries took {row:?}"
        );
        clear(&mut conn);
    }
    let taken: [&[(&str, &str)]; 5] = [&[], &[CALLED], &[CALLED, SEEN], &[LEFT], &[CALLED, LEFT]];
    for row in taken {
        assert_eq!(
            insert(&mut conn, row).unwrap(),
            1,
            "queue_entries refused {row:?}"
        );
        clear(&mut conn);
    }
}

/// A later write is held to the same CHECKs as an insert: a waiting entry
/// cannot be stamped seen.
#[test]
fn an_update_to_seen_before_called_is_refused() {
    let (_dir, mut conn) = open_temp();
    patient(&mut conn);
    insert(&mut conn, &[]).unwrap();
    assert!(
        diesel::sql_query("UPDATE queue_entries SET seen_at = '2026-09-23 09:30:00'")
            .execute(&mut conn)
            .is_err()
    );
}

/// One live entry per patient per shop per day. A seen or gone entry stops
/// counting, and another day is another line.
#[test]
fn a_patient_holds_one_live_entry_a_day() {
    let (_dir, mut conn) = open_temp();
    patient(&mut conn);
    insert(&mut conn, &[]).unwrap();
    let second = "'0199a0c1-0000-7000-8000-00000000e002'";
    assert!(insert(&mut conn, &[("id", second)]).is_err());
    // Called is still live.
    diesel::sql_query("UPDATE queue_entries SET called_at = '2026-09-23 09:10:00'")
        .execute(&mut conn)
        .unwrap();
    assert!(insert(&mut conn, &[("id", second)]).is_err());
    // Seen is not.
    diesel::sql_query("UPDATE queue_entries SET seen_at = '2026-09-23 09:30:00'")
        .execute(&mut conn)
        .unwrap();
    assert_eq!(insert(&mut conn, &[("id", second)]).unwrap(), 1);
    // Nor is gone: the second one leaves and a third comes in.
    diesel::sql_query(format!(
        "UPDATE queue_entries SET left_at = '2026-09-23 10:00:00' WHERE id = {second}"
    ))
    .execute(&mut conn)
    .unwrap();
    let third = "'0199a0c1-0000-7000-8000-00000000e003'";
    assert_eq!(insert(&mut conn, &[("id", third)]).unwrap(), 1);
    // The next day is its own line, beside today's live third.
    let fourth = [
        ("id", "'0199a0c1-0000-7000-8000-00000000e004'"),
        ("day", "'2026-09-24'"),
        ("arrived_at", "'2026-09-24 09:00:00'"),
    ];
    assert_eq!(insert(&mut conn, &fourth).unwrap(), 1);
}

/// The entry names a shop and a patient that exist, and neither can be
/// deleted from under it.
#[test]
fn the_shop_and_the_patient_are_real_and_are_not_deleted_from_under_the_queue() {
    let (_dir, mut conn) = open_temp();
    patient(&mut conn);
    assert!(insert(&mut conn, &[("shop_id", "99")]).is_err());
    assert!(insert(
        &mut conn,
        &[("patient_id", "'0199a0c1-0000-7000-8000-0000000000ff'")]
    )
    .is_err());
    insert(&mut conn, &[]).unwrap();
    assert!(diesel::sql_query("DELETE FROM patients")
        .execute(&mut conn)
        .is_err());
    assert!(diesel::sql_query("DELETE FROM shops WHERE id = 1")
        .execute(&mut conn)
        .is_err());
}

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    n: i32,
}

fn count(conn: &mut SqliteConnection, sql: &str) -> i32 {
    diesel::sql_query(sql).get_result::<Count>(conn).unwrap().n
}

/// The top of the stack goes down and comes back up: the down drops the
/// table and both indexes and nothing else (the patients it pointed at stay),
/// and the up makes them again.
#[test]
fn the_migration_reverts_and_reapplies() {
    let (_dir, mut conn) = open_temp();
    patient(&mut conn);
    let objects = "SELECT COUNT(*) AS n FROM sqlite_master WHERE name IN \
                   ('queue_entries', 'idx_queue_entries_live', 'idx_queue_entries_shop_day')";
    assert_eq!(count(&mut conn, objects), 3);

    let reverted = conn
        .revert_last_migration(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(reverted.to_string(), "20260923000020");
    assert_eq!(count(&mut conn, objects), 0);
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM patients"),
        1,
        "the queue's down took a patient with it"
    );

    conn.run_pending_migrations(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(count(&mut conn, objects), 3);
    insert(&mut conn, &[]).unwrap();
}
