// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Migration 000019: the `patients` table.
//!
//! Every case here goes through a raw INSERT and never through the service.
//! `services::patients` repeats most of these checks for a kinder message,
//! and a test that went through it would stay green with the CHECKs deleted
//! from `up.sql`, which is what this file is here to hold. The shape tests
//! every table shares (named, STRICT, carrying `shop_id`) and the whole-stack
//! revert live in `crates/retail/tests/migration.rs` beside every other
//! table's.

use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

mod common;

use common::{open_temp, revert_above};

const ID: &str = "0199a0c1-0000-7000-8000-000000000001";

/// One row with every column set to something the table takes, and one
/// column overridden with a raw SQL literal.
fn insert(conn: &mut SqliteConnection, column: &str, literal: &str) -> QueryResult<usize> {
    let mut values = vec![
        ("id", format!("'{ID}'")),
        ("shop_id", "1".to_string()),
        ("first_name", "'Amina'".to_string()),
        ("last_name", "'Benali'".to_string()),
        ("sex", "'female'".to_string()),
        ("date_of_birth", "'1990-05-17'".to_string()),
        ("phone", "'0555123456'".to_string()),
        ("created_at", "'2026-09-23 10:00:00'".to_string()),
        ("updated_at", "'2026-09-23 10:00:00'".to_string()),
    ];
    match values.iter_mut().find(|(name, _)| *name == column) {
        Some(value) => value.1 = literal.to_string(),
        None => values.push((column, literal.to_string())),
    }
    let names: Vec<&str> = values.iter().map(|(n, _)| *n).collect();
    let literals: Vec<&str> = values.iter().map(|(_, v)| v.as_str()).collect();
    diesel::sql_query(format!(
        "INSERT INTO patients ({}) VALUES ({})",
        names.join(", "),
        literals.join(", ")
    ))
    .execute(conn)
}

fn clear(conn: &mut SqliteConnection) {
    diesel::sql_query("DELETE FROM patients")
        .execute(conn)
        .unwrap();
}

/// Each refused value beside one the same column takes, so a CHECK that
/// refused everything (an unconditional `0 = 1`) would fail here too.
#[test]
fn each_column_refuses_what_the_file_says_it_may_not_hold() {
    let (_dir, mut conn) = open_temp();
    let cases: [(&str, &[&str], &[&str]); 6] = [
        (
            "id",
            &["'short'", "42", "'0199a0c1-0000-7000-8000-0000000000011'"],
            &[&format!("'{ID}'")],
        ),
        ("first_name", &["''", "'   '", "NULL"], &["'Amina'"]),
        ("last_name", &["''", "' '", "NULL"], &["'Benali'"]),
        ("sex", &["'f'", "'other'", "'Female'"], &["'male'", "NULL"]),
        (
            "date_of_birth",
            &[
                "'2026-02-30'",
                "'1990-5-17'",
                "'1990-05-17 00:00:00'",
                "'17/05/1990'",
            ],
            &["'2024-02-29'", "NULL"],
        ),
        ("created_at", &["NULL"], &["'2026-09-23 10:00:00'"]),
    ];
    for (column, refused, taken) in cases {
        for bad in refused {
            assert!(
                insert(&mut conn, column, bad).is_err(),
                "patients.{column} took {bad}"
            );
            clear(&mut conn);
        }
        for good in taken {
            assert_eq!(
                insert(&mut conn, column, good).unwrap(),
                1,
                "patients.{column} refused {good}"
            );
            clear(&mut conn);
        }
    }
}

/// The id is the key: a second row with the same one is refused, whatever
/// shop it claims.
#[test]
fn an_id_is_taken_once() {
    let (_dir, mut conn) = open_temp();
    insert(&mut conn, "id", &format!("'{ID}'")).unwrap();
    assert!(insert(&mut conn, "id", &format!("'{ID}'")).is_err());
}

/// A patient names a shop that exists, and the shop cannot be deleted from
/// under its patients.
#[test]
fn the_shop_is_a_real_one_and_is_not_deleted_from_under_its_patients() {
    let (_dir, mut conn) = open_temp();
    assert!(insert(&mut conn, "shop_id", "99").is_err());
    insert(&mut conn, "shop_id", "1").unwrap();
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

/// Its down drops the table and both indexes and nothing else, and the up
/// makes them again. The queue (000020) and everything after it sit on top and
/// point at `patients`, so they go down first; each file holds its own.
#[test]
fn the_migration_reverts_and_reapplies() {
    let (_dir, mut conn) = open_temp();
    let objects = "SELECT COUNT(*) AS n FROM sqlite_master WHERE name IN \
                   ('patients', 'idx_patients_shop_name', 'idx_patients_shop_phone')";
    assert_eq!(count(&mut conn, objects), 3);

    let above = revert_above(&mut conn, "20260923000019");
    assert!(above.contains(&"20260923000020".to_string()), "{above:?}");
    assert_eq!(count(&mut conn, objects), 3);
    let reverted = conn
        .revert_last_migration(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(reverted.to_string(), "20260923000019");
    assert_eq!(count(&mut conn, objects), 0);
    // The shop and its seeded owner are still there.
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM users WHERE id = 1"),
        1
    );

    conn.run_pending_migrations(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(count(&mut conn, objects), 3);
}
