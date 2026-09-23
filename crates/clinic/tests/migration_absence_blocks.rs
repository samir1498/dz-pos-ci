// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Migration 000023: the `absence_blocks` table.
//!
//! Raw INSERTs only, for the reason `migration_patients.rs` gives: the
//! service checks a week before it writes, and this file holds the table's
//! own CHECKs and index for a row that reaches it another way.

use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

mod common;

use common::{open_temp, revert_above};

/// One range with every column set to something the table takes, some
/// overridden with raw SQL literals.
fn insert(conn: &mut SqliteConnection, overrides: &[(&str, &str)]) -> QueryResult<usize> {
    let mut values = vec![
        ("id", "'0199a0c1-0000-7000-8000-0000000c0001'".to_string()),
        ("shop_id", "1".to_string()),
        ("starts_at", "'2026-09-24 10:00:00'".to_string()),
        ("ends_at", "'2026-09-24 11:00:00'".to_string()),
        ("created_at", "'2026-09-23 09:00:00'".to_string()),
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
        "INSERT INTO absence_blocks ({}) VALUES ({})",
        names.join(", "),
        literals.join(", ")
    ))
    .execute(conn)
}

fn clear(conn: &mut SqliteConnection) {
    diesel::sql_query("DELETE FROM absence_blocks")
        .execute(conn)
        .unwrap();
}

/// Each refused row beside the edge the table takes.
#[test]
fn each_row_the_file_says_it_may_not_hold_is_refused() {
    let (_dir, mut conn) = open_temp();
    let long = format!("'{}'", "x".repeat(61));
    let refused: [&[(&str, &str)]; 12] = [
        &[("id", "'short'")],
        &[("starts_at", "'2026-09-24T10:00:00'")],
        &[("starts_at", "'2026-09-24 10:00'")],
        &[("starts_at", "'2026-09-24 09:59:30'")],
        &[("ends_at", "'2026-09-24 11:00:30'")],
        &[("ends_at", "'2026-09-24 10:00:00'")],
        &[("ends_at", "'2026-09-24 09:00:00'")],
        &[("ends_at", "NULL")],
        &[("label", "''")],
        &[("label", "'   '")],
        &[("label", long.as_str())],
        &[("shop_id", "99")],
    ];
    for row in refused {
        assert!(
            insert(&mut conn, row).is_err(),
            "absence_blocks took {row:?}"
        );
        clear(&mut conn);
    }
    let sixty = format!("'{}'", "x".repeat(60));
    let taken: [&[(&str, &str)]; 3] = [
        &[("ends_at", "'2026-09-24 10:01:00'")],
        &[("label", sixty.as_str())],
        &[("ends_at", "'2026-09-30 08:00:00'"), ("label", "'congrès'")],
    ];
    for row in taken {
        assert_eq!(
            insert(&mut conn, row).unwrap(),
            1,
            "absence_blocks refused {row:?}"
        );
        clear(&mut conn);
    }
    insert(&mut conn, &[]).unwrap();
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

/// With the migrations above it taken off, the down drops the table and its
/// index and nothing else, and the up makes them again.
#[test]
fn the_migration_reverts_and_reapplies() {
    let (_dir, mut conn) = open_temp();
    let objects = "SELECT COUNT(*) AS n FROM sqlite_master WHERE name IN \
                   ('absence_blocks', 'idx_absence_blocks_shop_start')";
    assert_eq!(count(&mut conn, objects), 2);
    revert_above(&mut conn, "20260923000023");
    let reverted = conn
        .revert_last_migration(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(reverted.to_string(), "20260923000023");
    assert_eq!(count(&mut conn, objects), 0);
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM shops WHERE id = 1"),
        1
    );
    conn.run_pending_migrations(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(count(&mut conn, objects), 2);
    insert(&mut conn, &[]).unwrap();
}
