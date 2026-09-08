// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The first migration is the schema every later feature builds on, so the
//! test asserts the shape by name: the tables, `shop_id` on each of them,
//! the seeded shop with its owner and its dated régime fiscal.

use diesel::prelude::*;
use diesel::sql_types::{Integer, Text};

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = Integer)]
    n: i32,
}

#[derive(QueryableByName)]
struct Name {
    #[diesel(sql_type = Text)]
    name: String,
}

#[derive(QueryableByName)]
struct Pragma {
    #[diesel(sql_type = Integer)]
    foreign_keys: i32,
}

fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let conn = dzpos_core::db::open(&path).unwrap();
    (dir, conn)
}

fn count(conn: &mut SqliteConnection, sql: &str) -> i32 {
    let row: Count = diesel::sql_query(sql).get_result(conn).unwrap();
    row.n
}

#[test]
fn migration_creates_every_table() {
    let (_dir, mut conn) = open_temp();
    let rows: Vec<Name> = diesel::sql_query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
         AND name NOT LIKE '__diesel%' ORDER BY name",
    )
    .load(&mut conn)
    .unwrap();
    let names: Vec<String> = rows.into_iter().map(|r| r.name).collect();
    assert_eq!(
        names,
        vec!["categories", "products", "settings", "shops", "users"]
    );
}

#[test]
fn every_table_carries_shop_id() {
    // Rule 3 in docs/architecture.md: `shop_id` from day one, on every table.
    // `shops` carries it as its own primary key.
    let (_dir, mut conn) = open_temp();
    for table in ["categories", "products", "settings", "users"] {
        let n = count(
            &mut conn,
            &format!(
                "SELECT COUNT(*) AS n FROM pragma_table_info('{table}') WHERE name = 'shop_id'"
            ),
        );
        assert_eq!(n, 1, "{table} has no shop_id column");
    }
}

#[test]
fn money_columns_are_integer_centimes() {
    let (_dir, mut conn) = open_temp();
    let rows: Vec<Name> = diesel::sql_query(
        "SELECT name FROM pragma_table_info('products') WHERE name LIKE '%centimes' \
         AND type <> 'INTEGER'",
    )
    .load(&mut conn)
    .unwrap();
    assert!(
        rows.is_empty(),
        "a *_centimes column is not INTEGER: {:?}",
        rows.into_iter().map(|r| r.name).collect::<Vec<_>>()
    );
    let n = count(
        &mut conn,
        "SELECT COUNT(*) AS n FROM pragma_table_info('products') WHERE name LIKE '%centimes'",
    );
    assert_eq!(n, 3, "cost, selling and wholesale are the money columns");
}

#[test]
fn one_shop_is_seeded_with_an_owner() {
    let (_dir, mut conn) = open_temp();
    assert_eq!(count(&mut conn, "SELECT COUNT(*) AS n FROM shops"), 1);
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM users u JOIN shops s ON s.id = u.shop_id \
             WHERE u.role = 'owner'"
        ),
        1,
        "every shop needs one owner so later rows have an author"
    );
}

#[test]
fn regime_fiscal_is_a_dated_setting_defaulting_to_reel() {
    // features.md, Régime fiscal row: shop-level, dated, `ifu` or `reel`.
    let (_dir, mut conn) = open_temp();
    let rows: Vec<Name> = diesel::sql_query(
        "SELECT value AS name FROM settings WHERE key = 'regime_fiscal' \
         ORDER BY valid_from DESC LIMIT 1",
    )
    .load(&mut conn)
    .unwrap();
    assert_eq!(rows.len(), 1, "no regime_fiscal row was seeded");
    assert_eq!(rows[0].name, "reel");
    let dated = count(
        &mut conn,
        "SELECT COUNT(*) AS n FROM settings WHERE key = 'regime_fiscal' \
         AND valid_from IS NOT NULL",
    );
    assert_eq!(dated, 1, "the setting must carry valid_from");
}

#[test]
fn regime_fiscal_only_accepts_ifu_or_reel() {
    let (_dir, mut conn) = open_temp();
    let bad = diesel::sql_query(
        "INSERT INTO settings (shop_id, key, value, valid_from) \
         VALUES (1, 'regime_fiscal', 'forfait', '2027-01-01 00:00:00')",
    )
    .execute(&mut conn);
    assert!(bad.is_err(), "an unknown régime fiscal was accepted");
}

#[test]
fn barcode_is_unique_per_shop_and_may_be_null() {
    let (_dir, mut conn) = open_temp();
    let insert = |conn: &mut SqliteConnection, barcode: &str| {
        diesel::sql_query(format!(
            "INSERT INTO products (shop_id, name, barcode, unit, cost_centimes, \
             selling_centimes, qty_on_hand_milli, low_stock_at_milli, rate_bps) \
             VALUES (1, 'p', {barcode}, 'piece', 0, 0, 0, 0, 1900)"
        ))
        .execute(conn)
    };
    assert!(insert(&mut conn, "'613'").is_ok());
    assert!(
        insert(&mut conn, "'613'").is_err(),
        "a duplicate barcode was accepted inside one shop"
    );
    assert!(insert(&mut conn, "NULL").is_ok());
    assert!(
        insert(&mut conn, "NULL").is_ok(),
        "a blank barcode must stay allowed more than once"
    );
}

#[test]
fn the_migration_reverts_and_reapplies() {
    // architecture.md, Data: a migration ships with a test that runs it.
    // This is the first one, so the round trip is what there is to prove.
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_temp();
    conn.revert_last_migration(dzpos_core::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'products'"
        ),
        0,
        "down.sql left products behind"
    );
    conn.run_pending_migrations(dzpos_core::db::MIGRATIONS)
        .unwrap();
    assert_eq!(count(&mut conn, "SELECT COUNT(*) AS n FROM shops"), 1);
}

#[test]
fn foreign_keys_are_enforced() {
    let (_dir, mut conn) = open_temp();
    let row: Pragma = diesel::sql_query("PRAGMA foreign_keys")
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(row.foreign_keys, 1);
    let orphan = diesel::sql_query(
        "INSERT INTO products (shop_id, name, unit, cost_centimes, selling_centimes, \
         qty_on_hand_milli, low_stock_at_milli, rate_bps) \
         VALUES (999, 'p', 'piece', 0, 0, 0, 0, 1900)",
    )
    .execute(&mut conn);
    assert!(
        orphan.is_err(),
        "a product landed under a shop that does not exist"
    );
}
