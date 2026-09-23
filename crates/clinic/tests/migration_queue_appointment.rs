// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Migration 000026: `queue_entries.appointment_id`, the booking a
//! checked-in patient came for.
//!
//! Raw INSERTs only, never the service, for the reason
//! `migration_patients.rs` gives: `services::queue::check_in` returns the
//! first entry before a second one is ever written, so only a raw write
//! reaches the column's CHECK, its reference and its one-entry index.

use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

mod common;

use common::{open_temp, revert_above};

const PATIENT: &str = "0199a0c1-0000-7000-8000-000000000001";
const BOOKING: &str = "0199a0c1-0000-7000-8000-0000000a0001";

fn patient_and_booking(conn: &mut SqliteConnection) {
    diesel::sql_query(format!(
        "INSERT INTO patients (id, shop_id, first_name, last_name, created_at, updated_at) \
         VALUES ('{PATIENT}', 1, 'Amina', 'Benali', '2026-09-23 08:00:00', '2026-09-23 08:00:00')"
    ))
    .execute(conn)
    .unwrap();
    diesel::sql_query(format!(
        "INSERT INTO appointments (id, shop_id, patient_id, starts_at, slot_minutes, \
         created_at, updated_at) VALUES ('{BOOKING}', 1, '{PATIENT}', '2026-09-23 10:00:00', \
         15, '2026-09-22 08:00:00', '2026-09-22 08:00:00')"
    ))
    .execute(conn)
    .unwrap();
}

/// An entry of the patient on 2026-09-23, seen so the one-live-entry index
/// stays out of the way, naming `appointment` (a raw SQL literal).
fn entry(conn: &mut SqliteConnection, n: u32, appointment: &str) -> QueryResult<usize> {
    diesel::sql_query(format!(
        "INSERT INTO queue_entries (id, shop_id, patient_id, day, arrived_at, called_at, \
         seen_at, created_at, updated_at, appointment_id) VALUES \
         ('0199a0c1-0000-7000-8000-00000000e00{n}', 1, '{PATIENT}', '2026-09-23', \
         '2026-09-23 09:0{n}:00', '2026-09-23 09:0{n}:00', '2026-09-23 09:0{n}:30', \
         '2026-09-23 09:00:00', '2026-09-23 09:00:00', {appointment})"
    ))
    .execute(conn)
}

#[test]
fn an_entry_names_one_real_booking_and_a_booking_has_one_entry() {
    let (_dir, mut conn) = open_temp();
    patient_and_booking(&mut conn);
    // A walk-in, any number of them.
    assert_eq!(entry(&mut conn, 1, "NULL").unwrap(), 1);
    assert_eq!(entry(&mut conn, 2, "NULL").unwrap(), 1);
    // Not an id's length, and an id no appointment has.
    assert!(entry(&mut conn, 3, "'short'").is_err());
    assert!(entry(&mut conn, 3, "'0199a0c1-0000-7000-8000-0000000a0999'").is_err());
    // The booking itself, once; a second entry for it is refused.
    assert_eq!(entry(&mut conn, 3, &format!("'{BOOKING}'")).unwrap(), 1);
    assert!(entry(&mut conn, 4, &format!("'{BOOKING}'")).is_err());
    // And the booking cannot be deleted from under its entry.
    assert!(diesel::sql_query("DELETE FROM appointments")
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

/// The down drops the index and the column and keeps every entry, linked
/// or not; the up adds them back with every entry a walk-in.
#[test]
fn the_migration_reverts_and_reapplies() {
    let (_dir, mut conn) = open_temp();
    patient_and_booking(&mut conn);
    entry(&mut conn, 1, "NULL").unwrap();
    entry(&mut conn, 2, &format!("'{BOOKING}'")).unwrap();
    let column = "SELECT COUNT(*) AS n FROM pragma_table_info('queue_entries') \
                  WHERE name = 'appointment_id'";
    let index = "SELECT COUNT(*) AS n FROM sqlite_master \
                 WHERE name = 'idx_queue_entries_appointment'";
    assert_eq!(count(&mut conn, column), 1);
    assert_eq!(count(&mut conn, index), 1);
    revert_above(&mut conn, "20260923000026");
    let reverted = conn
        .revert_last_migration(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(reverted.to_string(), "20260923000026");
    assert_eq!(count(&mut conn, column), 0);
    assert_eq!(count(&mut conn, index), 0);
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM queue_entries"),
        2
    );
    conn.run_pending_migrations(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(count(&mut conn, column), 1);
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM queue_entries WHERE appointment_id IS NULL"
        ),
        2
    );
}
