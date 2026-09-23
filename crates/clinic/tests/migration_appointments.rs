// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Migration 000021: the `appointments` table.
//!
//! Raw INSERTs and UPDATEs only, never the service, for the reason
//! `migration_patients.rs` gives: `services::appointments` refuses a taken
//! slot first, and this file is what holds the table's own CHECKs and its
//! one-live-start index when two desks both get past the service's check.
//! The shape tests every table shares and the whole-stack revert live in
//! `crates/retail/tests/migration.rs`.

use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

mod common;

use common::{open_temp, revert_above};

const PATIENT: &str = "0199a0c1-0000-7000-8000-000000000001";
const FIRST: &str = "'0199a0c1-0000-7000-8000-0000000a0001'";
const SECOND: &str = "'0199a0c1-0000-7000-8000-0000000a0002'";

fn patient(conn: &mut SqliteConnection) {
    diesel::sql_query(format!(
        "INSERT INTO patients (id, shop_id, first_name, last_name, created_at, updated_at) \
         VALUES ('{PATIENT}', 1, 'Amina', 'Benali', '2026-09-23 08:00:00', '2026-09-23 08:00:00')"
    ))
    .execute(conn)
    .unwrap();
}

/// One appointment with every column set to something the table takes, and
/// some columns overridden with raw SQL literals.
fn insert(conn: &mut SqliteConnection, overrides: &[(&str, &str)]) -> QueryResult<usize> {
    let mut values = vec![
        ("id", FIRST.to_string()),
        ("shop_id", "1".to_string()),
        ("patient_id", format!("'{PATIENT}'")),
        ("starts_at", "'2026-09-24 09:00:00'".to_string()),
        ("slot_minutes", "15".to_string()),
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
        "INSERT INTO appointments ({}) VALUES ({})",
        names.join(", "),
        literals.join(", ")
    ))
    .execute(conn)
}

fn clear(conn: &mut SqliteConnection) {
    diesel::sql_query("DELETE FROM appointments")
        .execute(conn)
        .unwrap();
}

/// Each refused row beside the ones the table takes, so a CHECK that refused
/// everything would fail here too.
#[test]
fn each_row_the_file_says_it_may_not_hold_is_refused() {
    let (_dir, mut conn) = open_temp();
    patient(&mut conn);
    let refused: [&[(&str, &str)]; 9] = [
        &[("id", "'short'")],
        &[("id", "42")],
        &[("starts_at", "'2026-09-24T09:00:00'")],
        &[("starts_at", "'2026-09-24 09:00'")],
        &[("starts_at", "'2026-09-24 09:00:00.5'")],
        &[("starts_at", "'2026-09-24 09:00:30'")],
        &[("slot_minutes", "0")],
        &[("slot_minutes", "241")],
        &[("note", "'   '")],
    ];
    for row in refused {
        assert!(insert(&mut conn, row).is_err(), "appointments took {row:?}");
        clear(&mut conn);
    }
    let taken: [&[(&str, &str)]; 4] = [
        &[],
        &[("slot_minutes", "240")],
        &[("slot_minutes", "1")],
        &[
            ("note", "'contrôle'"),
            ("cancelled_at", "'2026-09-23 10:00:00'"),
        ],
    ];
    for row in taken {
        assert_eq!(
            insert(&mut conn, row).unwrap(),
            1,
            "appointments refused {row:?}"
        );
        clear(&mut conn);
    }
}

/// The service's double-booking check bypassed: two live rows on one start
/// of one shop are refused by the index alone, on insert and on a move.
#[test]
fn the_index_refuses_a_second_live_row_on_one_start() {
    let (_dir, mut conn) = open_temp();
    patient(&mut conn);
    insert(&mut conn, &[]).unwrap();
    assert!(insert(&mut conn, &[("id", SECOND)]).is_err());
    // A different start is taken, and moving it onto the first is not.
    insert(
        &mut conn,
        &[("id", SECOND), ("starts_at", "'2026-09-24 09:15:00'")],
    )
    .unwrap();
    assert!(diesel::sql_query(format!(
        "UPDATE appointments SET starts_at = '2026-09-24 09:00:00' WHERE id = {SECOND}"
    ))
    .execute(&mut conn)
    .is_err());
}

/// A cancelled row stops holding its start, and another shop's start is its
/// own.
#[test]
fn a_cancelled_row_frees_its_start_and_shops_do_not_share_one() {
    let (_dir, mut conn) = open_temp();
    patient(&mut conn);
    insert(&mut conn, &[("cancelled_at", "'2026-09-23 10:00:00'")]).unwrap();
    assert_eq!(insert(&mut conn, &[("id", SECOND)]).unwrap(), 1);

    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième cabinet')")
        .execute(&mut conn)
        .unwrap();
    let third = [
        ("id", "'0199a0c1-0000-7000-8000-0000000a0003'"),
        ("shop_id", "2"),
    ];
    assert_eq!(insert(&mut conn, &third).unwrap(), 1);
}

/// The appointment names a shop and a patient that exist, and neither can be
/// deleted from under it.
#[test]
fn the_shop_and_the_patient_are_real_and_are_not_deleted_from_under_the_book() {
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

/// With the book tools above it taken off, it goes down and comes back up:
/// the down drops the table and its index and nothing else (the patient it
/// pointed at stays), and the up makes them again.
#[test]
fn the_migration_reverts_and_reapplies() {
    let (_dir, mut conn) = open_temp();
    patient(&mut conn);
    let objects = "SELECT COUNT(*) AS n FROM sqlite_master WHERE name IN \
                   ('appointments', 'idx_appointments_live')";
    assert_eq!(count(&mut conn, objects), 2);

    revert_above(&mut conn, "20260923000021");
    let reverted = conn
        .revert_last_migration(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(reverted.to_string(), "20260923000021");
    assert_eq!(count(&mut conn, objects), 0);
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM patients"),
        1,
        "the book's down took a patient with it"
    );

    conn.run_pending_migrations(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(count(&mut conn, objects), 2);
    insert(&mut conn, &[]).unwrap();
}
