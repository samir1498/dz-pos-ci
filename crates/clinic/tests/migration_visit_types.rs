// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Migration 000024: the `visit_types` table.
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
        ("id", "'0199a0c1-0000-7000-8000-0000000d0001'".to_string()),
        ("shop_id", "1".to_string()),
        ("name", "'Contrôle'".to_string()),
        ("minutes", "30".to_string()),
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
        "INSERT INTO visit_types ({}) VALUES ({})",
        names.join(", "),
        literals.join(", ")
    ))
    .execute(conn)
}

fn clear(conn: &mut SqliteConnection) {
    diesel::sql_query("DELETE FROM visit_types")
        .execute(conn)
        .unwrap();
}

/// Each refused row beside the edge the table takes.
#[test]
fn each_row_the_file_says_it_may_not_hold_is_refused() {
    let (_dir, mut conn) = open_temp();
    let long = format!("'{}'", "x".repeat(61));
    let refused: [&[(&str, &str)]; 11] = [
        &[("id", "'short'")],
        &[("name", "''")],
        &[("name", "'  '")],
        &[("name", long.as_str())],
        &[("minutes", "0")],
        &[("minutes", "4")],
        &[("minutes", "7")],
        &[("minutes", "245")],
        &[("minutes", "'thirty'")],
        &[("updated_at", "NULL")],
        &[("shop_id", "99")],
    ];
    for row in refused {
        assert!(insert(&mut conn, row).is_err(), "visit_types took {row:?}");
        clear(&mut conn);
    }
    let sixty = format!("'{}'", "x".repeat(60));
    let taken: [&[(&str, &str)]; 3] = [
        &[("minutes", "5")],
        &[("minutes", "240")],
        &[("name", sixty.as_str())],
    ];
    for row in taken {
        assert_eq!(
            insert(&mut conn, row).unwrap(),
            1,
            "visit_types refused {row:?}"
        );
        clear(&mut conn);
    }
}

/// One name per shop whatever the case, and the shop is not deleted from
/// under its types.
#[test]
fn a_name_is_taken_once_per_shop_whatever_its_case() {
    let (_dir, mut conn) = open_temp();
    insert(&mut conn, &[]).unwrap();
    assert!(insert(
        &mut conn,
        &[
            ("id", "'0199a0c1-0000-7000-8000-0000000d0002'"),
            ("name", "'CONTRÔLE'"),
        ]
    )
    .is_ok());
    assert!(insert(
        &mut conn,
        &[
            ("id", "'0199a0c1-0000-7000-8000-0000000d0003'"),
            ("name", "'contrôle'"),
        ]
    )
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

/// With the migrations above it taken off, the down drops the table and its
/// index and nothing else, and the up makes them again.
#[test]
fn the_migration_reverts_and_reapplies() {
    let (_dir, mut conn) = open_temp();
    let objects = "SELECT COUNT(*) AS n FROM sqlite_master WHERE name IN \
                   ('visit_types', 'idx_visit_types_shop_name')";
    assert_eq!(count(&mut conn, objects), 2);
    revert_above(&mut conn, "20260923000024");
    let reverted = conn
        .revert_last_migration(dzpos_kernel::db::MIGRATIONS)
        .unwrap();
    assert_eq!(reverted.to_string(), "20260923000024");
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
