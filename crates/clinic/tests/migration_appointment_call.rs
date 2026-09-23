// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Migration 000028: `appointments.call_outcome` and `call_at`, the
//! confirmation call.
//!
//! Raw writes only, never the service, for the reason
//! `migration_patients.rs` gives: `services::appointments` always writes
//! the two together from the enum, so only a raw write reaches the CHECKs.

use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

mod common;

use common::{open_temp, revert_above};

const PATIENT: &str = "0199a0c1-0000-7000-8000-000000000001";

fn booking(conn: &mut SqliteConnection) {
    diesel::sql_query(format!(
        "INSERT INTO patients (id, shop_id, first_name, last_name, created_at, updated_at) \
         VALUES ('{PATIENT}', 1, 'Amina', 'Benali', '2026-09-23 08:00:00', '2026-09-23 08:00:00')"
    ))
    .execute(conn)
    .unwrap();
    diesel::sql_query(format!(
        "INSERT INTO appointments (id, shop_id, patient_id, starts_at, slot_minutes, \
         created_at, updated_at) VALUES ('0199a0c1-0000-7000-8000-0000000a0001', 1, \
         '{PATIENT}', '2026-09-24 09:00:00', 15, '2026-09-23 08:00:00', '2026-09-23 08:00:00')"
    ))
    .execute(conn)
    .unwrap();
}

fn set(conn: &mut SqliteConnection, outcome: &str, at: &str) -> QueryResult<usize> {
    diesel::sql_query(format!(
        "UPDATE appointments SET call_outcome = {outcome}, call_at = {at}"
    ))
    .execute(conn)
}

/// Two outcomes, and a moment exactly when there is one.
#[test]
fn an_outcome_is_one_of_two_and_comes_with_its_moment() {
    let (_dir, mut conn) = open_temp();
    booking(&mut conn);
    for (outcome, at) in [
        ("'maybe'", "'2026-09-23 18:00:00'"),
        ("'Confirmed'", "'2026-09-23 18:00:00'"),
        ("'confirmed'", "NULL"),
        ("NULL", "'2026-09-23 18:00:00'"),
    ] {
        assert!(
            set(&mut conn, outcome, at).is_err(),
            "took {outcome} at {at}"
        );
    }
    for (outcome, at) in [
        ("'confirmed'", "'2026-09-23 18:00:00'"),
        ("'no_answer'", "'2026-09-23 18:00:00.123456'"),
        ("NULL", "NULL"),
    ] {
        assert_eq!(
            set(&mut conn, outcome, at).unwrap(),
            1,
            "refused {outcome} at {at}"
        );
    }
}

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    n: i32,
}

fn count(conn: &mut SqliteConnection, sql: &str) -> i32 {
    diesel::sql_query(sql).get_result::<Count>(conn).unwrap().n
}

/// The down drops both columns and keeps every row; the up adds them back
/// empty.
#[test]
fn the_migration_reverts_and_reapplies() {
    let (_dir, mut conn) = open_temp();
    booking(&mut conn);
    set(&mut conn, "'confirmed'", "'2026-09-23 18:00:00'").unwrap();
    let columns = "SELECT COUNT(*) AS n FROM pragma_table_info('appointments') \
                   WHERE name IN ('call_outcome', 'call_at')";
    assert_eq!(count(&mut conn, columns), 2);
    revert_above(&mut conn, "20260923000028");
    let reverted = conn
        .revert_last_migration(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(reverted.to_string(), "20260923000028");
    assert_eq!(count(&mut conn, columns), 0);
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM appointments"),
        1
    );
    conn.run_pending_migrations(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM appointments WHERE call_outcome IS NULL AND call_at IS NULL"
        ),
        1
    );
}
