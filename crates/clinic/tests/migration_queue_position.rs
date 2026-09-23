// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Migration 000027: `queue_entries.position`, the desk's order of a day.
//!
//! Raw writes only, never the service, for the reason
//! `migration_patients.rs` gives: `services::queue` always writes a place
//! of 1 or more, so only a raw write reaches the column's CHECK, and only
//! rows written before the migration show how the up numbers them.

use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

mod common;

use common::{open_temp, revert_above};

const PATIENT: &str = "0199a0c1-0000-7000-8000-000000000001";

fn patient(conn: &mut SqliteConnection) {
    diesel::sql_query(format!(
        "INSERT INTO patients (id, shop_id, first_name, last_name, created_at, updated_at) \
         VALUES ('{PATIENT}', 1, 'Amina', 'Benali', '2026-09-23 08:00:00', '2026-09-23 08:00:00')"
    ))
    .execute(conn)
    .unwrap();
}

/// A seen entry (so the one-live-entry index stays out of the way) with
/// the id's last digit `n`, on `day` at `time`, the columns before this
/// migration only.
fn entry(conn: &mut SqliteConnection, n: u32, day: &str, time: &str) {
    diesel::sql_query(format!(
        "INSERT INTO queue_entries (id, shop_id, patient_id, day, arrived_at, called_at, \
         seen_at, created_at, updated_at) VALUES \
         ('0199a0c1-0000-7000-8000-00000000e00{n}', 1, '{PATIENT}', '{day}', \
         '{day} {time}', '{day} {time}', '{day} {time}', '{day} {time}', '{day} {time}')"
    ))
    .execute(conn)
    .unwrap();
}

#[derive(QueryableByName)]
struct Place {
    #[diesel(sql_type = diesel::sql_types::Text)]
    id: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    position: i32,
}

fn places(conn: &mut SqliteConnection) -> Vec<(String, i32)> {
    diesel::sql_query("SELECT id, position FROM queue_entries ORDER BY id")
        .load::<Place>(conn)
        .unwrap()
        .into_iter()
        .map(|p| (p.id[32..].to_string(), p.position))
        .collect()
}

/// The entries already there are numbered per shop and day in arrival
/// order, a tie in arrival broken by id; a later id may have arrived first.
#[test]
fn the_up_numbers_each_days_entries_in_the_order_they_arrived() {
    let (_dir, mut conn) = open_temp();
    revert_above(&mut conn, "20260923000026");
    patient(&mut conn);
    entry(&mut conn, 1, "2026-09-23", "09:30:00");
    entry(&mut conn, 2, "2026-09-23", "09:00:00");
    entry(&mut conn, 3, "2026-09-23", "09:00:00");
    entry(&mut conn, 4, "2026-09-24", "08:00:00");
    conn.run_pending_migrations(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        places(&mut conn),
        [
            ("e001".to_string(), 3),
            ("e002".to_string(), 1),
            ("e003".to_string(), 2),
            ("e004".to_string(), 1),
        ]
    );
}

#[test]
fn a_place_is_one_or_more() {
    let (_dir, mut conn) = open_temp();
    patient(&mut conn);
    entry(&mut conn, 1, "2026-09-23", "09:00:00");
    for refused in ["0", "-1", "'one'", "1.5", "NULL"] {
        assert!(
            diesel::sql_query(format!("UPDATE queue_entries SET position = {refused}"))
                .execute(&mut conn)
                .is_err(),
            "position took {refused}"
        );
    }
    assert_eq!(
        diesel::sql_query("UPDATE queue_entries SET position = 7")
            .execute(&mut conn)
            .unwrap(),
        1
    );
}

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    n: i32,
}

fn count(conn: &mut SqliteConnection, sql: &str) -> i32 {
    diesel::sql_query(sql).get_result::<Count>(conn).unwrap().n
}

/// The down drops the index and the column and keeps every entry.
#[test]
fn the_migration_reverts_and_reapplies() {
    let (_dir, mut conn) = open_temp();
    patient(&mut conn);
    entry(&mut conn, 1, "2026-09-23", "09:00:00");
    let column = "SELECT COUNT(*) AS n FROM pragma_table_info('queue_entries') \
                  WHERE name = 'position'";
    let index = "SELECT COUNT(*) AS n FROM sqlite_master WHERE name = 'idx_queue_entries_order'";
    assert_eq!(count(&mut conn, column), 1);
    assert_eq!(count(&mut conn, index), 1);
    revert_above(&mut conn, "20260923000027");
    let reverted = conn
        .revert_last_migration(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(reverted.to_string(), "20260923000027");
    assert_eq!(count(&mut conn, column), 0);
    assert_eq!(count(&mut conn, index), 0);
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM queue_entries"),
        1
    );
    conn.run_pending_migrations(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM queue_entries WHERE position = 1"
        ),
        1
    );
}
