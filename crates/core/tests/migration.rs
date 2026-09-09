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
        vec![
            "categories",
            "counters",
            "products",
            "settings",
            "shops",
            "users"
        ]
    );
}

#[test]
fn every_table_carries_shop_id() {
    // Rule 3 in docs/architecture.md: `shop_id` from day one, on every table.
    // `shops` carries it as its own primary key.
    let (_dir, mut conn) = open_temp();
    for table in ["categories", "counters", "products", "settings", "users"] {
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
fn the_bundled_sqlite_is_new_enough_for_strict_tables() {
    // STRICT arrived in SQLite 3.37. libsqlite3-sys is pinned bundled, so the
    // version is the crate's, not the machine's.
    let (_dir, mut conn) = open_temp();
    let rows: Vec<Name> = diesel::sql_query("SELECT sqlite_version() AS name")
        .load(&mut conn)
        .unwrap();
    let version = rows[0].name.clone();
    let parts: Vec<u32> = version.split('.').filter_map(|p| p.parse().ok()).collect();
    assert!(
        parts[0] > 3 || (parts[0] == 3 && parts[1] >= 37),
        "bundled SQLite {version} is older than 3.37"
    );
}

#[test]
fn every_table_is_strict() {
    // A STRICT table refuses text where an integer belongs, so a price can
    // never be read back as something other than centimes. The flag is read
    // from pragma_table_list: grepping the CREATE text for the word matched
    // a comment and stayed green with STRICT removed.
    let (_dir, mut conn) = open_temp();
    for table in [
        "shops",
        "settings",
        "users",
        "counters",
        "categories",
        "products",
    ] {
        let strict = count(
            &mut conn,
            &format!("SELECT strict AS n FROM pragma_table_list WHERE name = '{table}'"),
        );
        assert_eq!(strict, 1, "{table} is not STRICT");
    }
}

/// One INSERT per table with every constrained column at a valid value, so a
/// probe can swap a single column for a bad literal.
fn insert_with(table: &str, column: &str, literal: &str) -> String {
    let (columns, values): (&str, &[&str]) = match table {
        "products" => (
            "shop_id, name, unit, cost_centimes, selling_centimes, wholesale_centimes, \
             qty_on_hand_milli, low_stock_at_milli, rate_bps",
            &["1", "'p'", "'piece'", "0", "0", "0", "0", "0", "1900"],
        ),
        "categories" => ("shop_id, name, default_rate_bps", &["1", "'c'", "1900"]),
        "counters" => ("shop_id, name, next_value", &["1", "'probe'", "1"]),
        other => panic!("no insert template for {other}"),
    };
    let names: Vec<&str> = columns.split(',').map(str::trim).collect();
    assert_eq!(names.len(), values.len(), "template for {table} is uneven");
    let at = names
        .iter()
        .position(|n| *n == column)
        .unwrap_or_else(|| panic!("{table} template has no column {column}"));
    let mut row: Vec<&str> = values.to_vec();
    row[at] = literal;
    format!(
        "INSERT INTO {table} ({}) VALUES ({})",
        names.join(", "),
        row.join(", ")
    )
}

fn probe(
    conn: &mut SqliteConnection,
    table: &str,
    column: &str,
    literal: &str,
) -> QueryResult<usize> {
    diesel::sql_query(insert_with(table, column, literal)).execute(conn)
}

#[test]
fn every_money_and_rate_column_refuses_a_real_a_text_and_a_negative() {
    // Rule 6: money is integer centimes. 19.99 was once read back as
    // Money(19). Every constrained column of every table is probed, so a
    // dropped CHECK on any one of them goes red here.
    let (_dir, mut conn) = open_temp();
    let money_columns = [
        ("products", "cost_centimes"),
        ("products", "selling_centimes"),
        ("products", "wholesale_centimes"),
        ("products", "low_stock_at_milli"),
    ];
    for (table, column) in money_columns {
        assert!(
            probe(&mut conn, table, column, "0").is_ok(),
            "{table}.{column}: an integer was refused"
        );
        for bad in ["19.99", "'19.99'", "'abc'", "-1"] {
            assert!(
                probe(&mut conn, table, column, bad).is_err(),
                "{table}.{column} accepted {bad}"
            );
        }
    }
    // A stock count may be negative (a sale before the receipt is keyed in);
    // it still has to be an integer of milli-units.
    assert!(probe(&mut conn, "products", "qty_on_hand_milli", "-1").is_ok());
    for bad in ["1.5", "'abc'"] {
        assert!(
            probe(&mut conn, "products", "qty_on_hand_milli", bad).is_err(),
            "products.qty_on_hand_milli accepted {bad}"
        );
    }
    // A counter starts at 1 and only ever goes up.
    assert!(probe(&mut conn, "counters", "next_value", "1").is_ok());
    for bad in ["0", "-1", "1.5", "'abc'"] {
        assert!(
            probe(&mut conn, "counters", "next_value", bad).is_err(),
            "counters.next_value accepted {bad}"
        );
    }
}

#[test]
fn a_rate_column_refuses_anything_outside_zero_to_one_whole() {
    let (_dir, mut conn) = open_temp();
    for (table, column) in [("products", "rate_bps"), ("categories", "default_rate_bps")] {
        for fine in ["0", "10000"] {
            assert!(
                probe(&mut conn, table, column, fine).is_ok(),
                "{table}.{column} refused {fine}"
            );
            // A category name is unique per shop, so the probe row goes
            // before the next valid value lands on the same name.
            diesel::sql_query(format!("DELETE FROM {table} WHERE name IN ('p', 'c')"))
                .execute(&mut conn)
                .unwrap();
        }
        for bad in ["190000", "10001", "-1", "1.5", "'abc'"] {
            assert!(
                probe(&mut conn, table, column, bad).is_err(),
                "{table}.{column} accepted {bad}"
            );
        }
    }
}

#[test]
fn a_lossless_real_is_stored_as_an_integer_by_strict_itself() {
    // STRICT coerces 19.0 and '19' to integer 19 before any CHECK runs, so
    // the typeof() clauses in the migration never see a real. They stay as
    // a statement of intent; this pins what the engine actually does, so a
    // future comment cannot claim more.
    let (_dir, mut conn) = open_temp();
    for literal in ["19.0", "'19'"] {
        probe(&mut conn, "products", "cost_centimes", literal).unwrap();
    }
    let integers = count(
        &mut conn,
        "SELECT COUNT(*) AS n FROM products \
         WHERE cost_centimes = 19 AND typeof(cost_centimes) = 'integer'",
    );
    assert_eq!(
        integers, 2,
        "a lossless real or a numeric text was not coerced to integer"
    );
}

#[test]
fn a_deleted_id_is_never_handed_out_again() {
    // Without AUTOINCREMENT SQLite reuses the highest deleted rowid, so a
    // deleted product's id would come back and with it its in-store barcode.
    let (_dir, mut conn) = open_temp();
    let rows = [
        ("shops", "INSERT INTO shops (name) VALUES ('autre magasin')"),
        (
            "users",
            "INSERT INTO users (shop_id, name, role) VALUES (1, 'caissier', 'cashier')",
        ),
        (
            "categories",
            "INSERT INTO categories (shop_id, name, default_rate_bps) VALUES (1, 'c', 1900)",
        ),
        (
            "products",
            "INSERT INTO products (shop_id, name, unit, cost_centimes, selling_centimes, \
             qty_on_hand_milli, low_stock_at_milli, rate_bps) \
             VALUES (1, 'p', 'piece', 0, 0, 0, 0, 1900)",
        ),
    ];
    for (table, insert) in rows {
        diesel::sql_query(insert).execute(&mut conn).unwrap();
        let first = count(&mut conn, &format!("SELECT MAX(id) AS n FROM {table}"));
        diesel::sql_query(format!("DELETE FROM {table} WHERE id = {first}"))
            .execute(&mut conn)
            .unwrap();
        diesel::sql_query(insert).execute(&mut conn).unwrap();
        let second = count(&mut conn, &format!("SELECT MAX(id) AS n FROM {table}"));
        assert_ne!(second, first, "{table} handed out a deleted id again");
    }
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
fn the_seeded_owner_carries_a_pin_that_cannot_verify() {
    // A NULL pin_hash reads as "no PIN set", which is one careless check away
    // from "anyone may log in". M4 replaces the sentinel with a real hash.
    let (_dir, mut conn) = open_temp();
    let rows: Vec<Name> = diesel::sql_query(
        "SELECT pin_hash AS name FROM users WHERE shop_id = 1 AND role = 'owner'",
    )
    .load(&mut conn)
    .unwrap();
    assert_eq!(rows.len(), 1, "the seeded owner has a NULL pin_hash");
    assert_eq!(rows[0].name, "!unset");

    let nullable = count(
        &mut conn,
        "SELECT COUNT(*) AS n FROM pragma_table_info('users') \
         WHERE name = 'pin_hash' AND \"notnull\" = 1",
    );
    assert_eq!(nullable, 1, "pin_hash still accepts NULL");

    let null_row = diesel::sql_query(
        "INSERT INTO users (shop_id, name, role, pin_hash) VALUES (1, 'x', 'cashier', NULL)",
    )
    .execute(&mut conn);
    assert!(null_row.is_err(), "a NULL pin_hash was accepted");
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
