// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Migration 000025: the `no_show_at` column on `appointments`.
//!
//! Raw INSERTs only, never the service, for the reason
//! `migration_patients.rs` gives: `services::appointments` refuses a mark
//! on a cancelled row first, and this file holds the column's own CHECK.

use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

mod common;

use common::{open_temp, revert_above};

const PATIENT: &str = "0199a0c1-0000-7000-8000-000000000001";
const FIRST: &str = "'0199a0c1-0000-7000-8000-0000000a0001'";

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

/// A mark never sits beside a cancellation.
#[test]
fn the_mark_is_never_on_a_cancelled_row() {
    let (_dir, mut conn) = open_temp();
    patient(&mut conn);
    let refused: [&[(&str, &str)]; 1] = [&[
        ("no_show_at", "'2026-09-24 09:30:00'"),
        ("cancelled_at", "'2026-09-24 08:00:00'"),
    ]];
    for row in refused {
        assert!(insert(&mut conn, row).is_err(), "appointments took {row:?}");
        clear(&mut conn);
    }
    let taken: [&[(&str, &str)]; 4] = [
        &[],
        &[("no_show_at", "'2026-09-24 09:30:00'")],
        &[("no_show_at", "'2026-09-24 09:30:00.123456'")],
        &[("cancelled_at", "'2026-09-24 08:00:00'")],
    ];
    for row in taken {
        assert_eq!(
            insert(&mut conn, row).unwrap(),
            1,
            "appointments refused {row:?}"
        );
        clear(&mut conn);
    }
    // Cancelling a marked row is refused by the table too.
    insert(&mut conn, &[("no_show_at", "'2026-09-24 09:30:00'")]).unwrap();
    assert!(
        diesel::sql_query("UPDATE appointments SET cancelled_at = '2026-09-24 10:00:00'")
            .execute(&mut conn)
            .is_err()
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

/// The down drops the column and keeps every row, marked or not; the up
/// adds it back empty.
#[test]
fn the_migration_reverts_and_reapplies() {
    let (_dir, mut conn) = open_temp();
    patient(&mut conn);
    insert(&mut conn, &[("no_show_at", "'2026-09-24 09:30:00'")]).unwrap();
    let column = "SELECT COUNT(*) AS n FROM pragma_table_info('appointments') \
                  WHERE name = 'no_show_at'";
    assert_eq!(count(&mut conn, column), 1);
    revert_above(&mut conn, "20260923000025");
    let reverted = conn
        .revert_last_migration(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(reverted.to_string(), "20260923000025");
    assert_eq!(count(&mut conn, column), 0);
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM appointments"),
        1
    );
    conn.run_pending_migrations(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(count(&mut conn, column), 1);
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM appointments WHERE no_show_at IS NULL"
        ),
        1
    );
}
