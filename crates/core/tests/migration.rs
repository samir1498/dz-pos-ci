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

mod common;

use common::open_temp;

/// A database carrying the first `n` migrations and nothing after them, so
/// migration `n + 1` can be applied to a file that already holds a shop's
/// data. Every migration ships with a test that opens one of these
/// (architecture.md, Data).
fn open_at_migration(n: usize) -> (tempfile::TempDir, SqliteConnection) {
    use diesel::connection::SimpleConnection;
    use diesel_migrations::MigrationHarness;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = SqliteConnection::establish(&path.to_string_lossy()).unwrap();
    conn.batch_execute("PRAGMA foreign_keys=ON;").unwrap();
    let pending = conn.pending_migrations(dzpos_core::db::MIGRATIONS).unwrap();
    assert!(pending.len() > n, "there is no migration after {n}");
    for migration in pending.iter().take(n) {
        conn.run_migration(migration).unwrap();
    }
    (dir, conn)
}

/// A database carrying every migration up to but not including the one whose
/// name contains `marker`, so that migration can be applied to a file that
/// already holds a shop's data. Named rather than numbered because a
/// migration written on another branch can land in between and shift every
/// ordinal after it; the file this opens is the one this migration actually
/// runs against.
fn open_before_migration(marker: &str) -> (tempfile::TempDir, SqliteConnection) {
    use diesel::connection::SimpleConnection;
    use diesel::migration::Migration;
    use diesel_migrations::MigrationHarness;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = SqliteConnection::establish(&path.to_string_lossy()).unwrap();
    conn.batch_execute("PRAGMA foreign_keys=ON;").unwrap();
    let pending = conn.pending_migrations(dzpos_core::db::MIGRATIONS).unwrap();
    let mut reached = false;
    for migration in pending.iter() {
        if Migration::<diesel::sqlite::Sqlite>::name(migration.as_ref())
            .to_string()
            .contains(marker)
        {
            reached = true;
            break;
        }
        conn.run_migration(migration).unwrap();
    }
    assert!(reached, "no migration is named {marker}");
    (dir, conn)
}

/// What `shops(id)` does to a row of `table` when the shop is deleted, as
/// SQLite itself reports it.
fn on_delete_from_shops(conn: &mut SqliteConnection, table: &str) -> String {
    let rows: Vec<Name> = diesel::sql_query(format!(
        "SELECT \"on_delete\" AS name FROM pragma_foreign_key_list('{table}') \
         WHERE \"table\" = 'shops'"
    ))
    .load(conn)
    .unwrap();
    assert_eq!(rows.len(), 1, "{table} has no foreign key onto shops");
    rows[0].name.clone()
}

/// Orphans the file carries: rows whose parent is gone. `PRAGMA
/// foreign_key_check` reports them as rows and never fails a statement, so a
/// migration that runs with the keys off cannot catch its own; this is where
/// they are caught.
fn orphan_rows(conn: &mut SqliteConnection) -> i32 {
    count(conn, "SELECT COUNT(*) AS n FROM pragma_foreign_key_check")
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
            "audit_log",
            "categories",
            "counters",
            "customers",
            "debt_allocations",
            "debt_ledger",
            "document_lines",
            "document_tva",
            "documents",
            "products",
            "settings",
            "shops",
            "stock_movements",
            "users"
        ]
    );
}

#[test]
fn every_table_carries_shop_id() {
    // Rule 3 in docs/architecture.md: `shop_id` from day one, on every table.
    // `shops` carries it as its own primary key.
    let (_dir, mut conn) = open_temp();
    for table in [
        "audit_log",
        "categories",
        "counters",
        "customers",
        "debt_allocations",
        "debt_ledger",
        "document_lines",
        "document_tva",
        "documents",
        "products",
        "settings",
        "stock_movements",
        "users",
    ] {
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
        "documents",
        "document_lines",
        "document_tva",
        "stock_movements",
        "audit_log",
        "customers",
        "debt_ledger",
        "debt_allocations",
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
        "documents" => (
            "shop_id, kind, series, number, issued_at, user_id, regime, payment_mode, \
             seller_name, total_ht_centimes, discount_centimes, subtotal_ht_centimes, \
             tva_centimes, total_ttc_centimes, stamp_centimes, net_to_pay_centimes, \
             tendered_centimes, change_centimes, old_balance_centimes, \
             remaining_debt_centimes, total_debt_centimes, buyer_party_kind, status",
            &[
                "1",
                "'ticket'",
                "'doc_ticket'",
                "1",
                "'2026-09-09 10:00:00'",
                "1",
                "'reel'",
                "'cash'",
                "'Mon magasin'",
                "0",
                "0",
                "0",
                "0",
                "0",
                "0",
                "0",
                "0",
                "0",
                "NULL",
                "NULL",
                "NULL",
                "NULL",
                "'issued'",
            ],
        ),
        "document_lines" => (
            "shop_id, document_id, position, name, qty_milli, unit_price_centimes, \
             line_discount_centimes, rate_bps, line_total_centimes",
            &["1", "1", "0", "'p'", "1000", "0", "0", "1900", "0"],
        ),
        "document_tva" => (
            "shop_id, document_id, rate_bps, base_centimes, amount_centimes",
            &["1", "1", "1900", "0", "0"],
        ),
        "stock_movements" => (
            "shop_id, product_id, kind, qty_milli, unit_cost_centimes, user_id",
            &["1", "1", "'sale'", "-1000", "0", "1"],
        ),
        "audit_log" => (
            "shop_id, user_id, action, entity, entity_id",
            &["1", "1", "'update'", "'product'", "1"],
        ),
        "customers" => (
            "shop_id, name, party_kind, credit_limit_centimes, warn_threshold_centimes, active",
            &["1", "'Ahmed'", "'company'", "0", "0", "1"],
        ),
        "debt_ledger" => (
            "shop_id, customer_id, kind, debit_centimes, credit_centimes, user_id, note",
            &["1", "1", "'sale'", "0", "0", "1", "NULL"],
        ),
        "debt_allocations" => (
            "shop_id, payment_ledger_id, document_id, amount_centimes",
            &["1", "1", "1", "1"],
        ),
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

/// The probe rows of the document tables are unique on more than one column,
/// so a row that landed is cleared before the next valid one is tried.
fn clear(conn: &mut SqliteConnection, table: &str) {
    let sql = match table {
        // The seeded document is what the line, TVA and movement probes point
        // at, so it stays; the probe rows above it go.
        "documents" => "DELETE FROM documents WHERE series <> 'seed'".to_string(),
        // The seeded customer and the seeded payment are what the ledger and
        // the allocation probes point at, and both RESTRICT, so a blanket
        // DELETE would fail rather than clear. The seed rows carry a marker
        // the probe rows do not.
        "customers" => "DELETE FROM customers WHERE name <> 'seed'".to_string(),
        "debt_ledger" => "DELETE FROM debt_ledger WHERE note IS NULL".to_string(),
        other => format!("DELETE FROM {other}"),
    };
    diesel::sql_query(sql).execute(conn).unwrap();
}

/// A product and a document the line and movement probes can point at.
fn seed_for_probes(conn: &mut SqliteConnection) {
    diesel::sql_query(
        "INSERT INTO products (id, shop_id, name, unit, cost_centimes, selling_centimes, \
         qty_on_hand_milli, low_stock_at_milli, rate_bps) \
         VALUES (1, 1, 'p', 'piece', 0, 0, 0, 0, 1900)",
    )
    .execute(conn)
    .unwrap();
    diesel::sql_query(insert_with("documents", "series", "'seed'"))
        .execute(conn)
        .unwrap();
}

/// The above, plus the customer the ledger probes point at and the payment
/// row the allocation probes settle. Both are marked so `clear` can tell them
/// from a probe row.
fn seed_for_debt_probes(conn: &mut SqliteConnection) {
    seed_for_probes(conn);
    diesel::sql_query(insert_with("customers", "name", "'seed'"))
        .execute(conn)
        .unwrap();
    diesel::sql_query(insert_with("debt_ledger", "note", "'seed'"))
        .execute(conn)
        .unwrap();
}

#[test]
fn every_money_column_of_a_customer_and_the_debt_ledger_refuses_a_real_and_a_text() {
    // Rule 6 again, on the tables migration 4 adds. A credit limit and a
    // ledger movement are both amounts owed, and a debt read back as
    // something other than centimes is a customer billed the wrong figure.
    let (_dir, mut conn) = open_temp();
    seed_for_debt_probes(&mut conn);
    let columns = [
        ("customers", "credit_limit_centimes"),
        ("customers", "warn_threshold_centimes"),
        ("debt_ledger", "debit_centimes"),
        ("debt_ledger", "credit_centimes"),
    ];
    for (table, column) in columns {
        assert!(
            probe(&mut conn, table, column, "0").is_ok(),
            "{table}.{column}: an integer was refused"
        );
        clear(&mut conn, table);
        for bad in ["19.99", "'19.99'", "'abc'", "-1"] {
            assert!(
                probe(&mut conn, table, column, bad).is_err(),
                "{table}.{column} accepted {bad}"
            );
        }
    }
    // An allocation of nothing settles nothing, so zero is refused where the
    // other columns take it.
    assert!(probe(&mut conn, "debt_allocations", "amount_centimes", "1").is_ok());
    clear(&mut conn, "debt_allocations");
    for bad in ["0", "-1", "19.99", "'abc'"] {
        assert!(
            probe(&mut conn, "debt_allocations", "amount_centimes", bad).is_err(),
            "debt_allocations.amount_centimes accepted {bad}"
        );
    }
}

#[test]
fn a_ledger_row_carries_one_direction_and_a_customer_one_of_the_two_party_kinds() {
    let (_dir, mut conn) = open_temp();
    seed_for_debt_probes(&mut conn);
    // features.md §2: a row raises the debt or lowers it. A row carrying both
    // would be two movements wearing one id.
    assert!(
        diesel::sql_query(
            "INSERT INTO debt_ledger (shop_id, customer_id, kind, debit_centimes, \
             credit_centimes, user_id) VALUES (1, 1, 'sale', 1000, 1000, 1)"
        )
        .execute(&mut conn)
        .is_err(),
        "a ledger row carried a debit and a credit at once"
    );
    for one_way in [("1000", "0"), ("0", "1000")] {
        diesel::sql_query(format!(
            "INSERT INTO debt_ledger (shop_id, customer_id, kind, debit_centimes, \
             credit_centimes, user_id) VALUES (1, 1, 'sale', {}, {}, 1)",
            one_way.0, one_way.1
        ))
        .execute(&mut conn)
        .unwrap();
    }
    for value in [
        "'opening'",
        "'sale'",
        "'payment'",
        "'avoir'",
        "'adjustment'",
    ] {
        assert!(
            probe(&mut conn, "debt_ledger", "kind", value).is_ok(),
            "debt_ledger.kind refused {value}"
        );
    }
    assert!(probe(&mut conn, "debt_ledger", "kind", "'writeoff'").is_err());
    for value in ["'company'", "'consumer'"] {
        assert!(
            probe(&mut conn, "customers", "party_kind", value).is_ok(),
            "customers.party_kind refused {value}"
        );
    }
    for value in ["'entreprise'", "''"] {
        assert!(
            probe(&mut conn, "customers", "party_kind", value).is_err(),
            "customers.party_kind accepted {value}"
        );
    }
}

#[test]
fn the_balance_columns_of_a_document_take_a_negative_and_the_others_do_not() {
    // A customer who overpays is owed money, and the statement says so
    // (features.md §2). The triple is the one place a document carries a
    // signed amount, so it is pinned here rather than left to a reader to
    // notice the missing `>= 0`.
    let (_dir, mut conn) = open_temp();
    seed_for_probes(&mut conn);
    for column in [
        "old_balance_centimes",
        "remaining_debt_centimes",
        "total_debt_centimes",
    ] {
        assert!(
            probe(&mut conn, "documents", column, "-5000").is_ok(),
            "documents.{column} refused a negative balance"
        );
        clear(&mut conn, "documents");
        for bad in ["19.99", "'abc'"] {
            assert!(
                probe(&mut conn, "documents", column, bad).is_err(),
                "documents.{column} accepted {bad}"
            );
        }
    }
}

#[test]
fn every_money_and_quantity_column_of_a_document_refuses_a_real_a_text_and_a_negative() {
    // Same rule 6 probe as the first migration's tables: a wrong centime on a
    // stored document is a legal problem, and a dropped CHECK goes red here.
    let (_dir, mut conn) = open_temp();
    seed_for_probes(&mut conn);
    let columns = [
        ("documents", "total_ht_centimes"),
        ("documents", "discount_centimes"),
        ("documents", "subtotal_ht_centimes"),
        ("documents", "tva_centimes"),
        ("documents", "total_ttc_centimes"),
        ("documents", "stamp_centimes"),
        ("documents", "net_to_pay_centimes"),
        ("documents", "tendered_centimes"),
        ("documents", "change_centimes"),
        ("document_lines", "unit_price_centimes"),
        ("document_lines", "line_discount_centimes"),
        ("document_lines", "line_total_centimes"),
        ("document_tva", "base_centimes"),
        ("document_tva", "amount_centimes"),
        ("stock_movements", "unit_cost_centimes"),
    ];
    for (table, column) in columns {
        assert!(
            probe(&mut conn, table, column, "0").is_ok(),
            "{table}.{column}: an integer was refused"
        );
        clear(&mut conn, table);
        for bad in ["19.99", "'19.99'", "'abc'", "-1"] {
            assert!(
                probe(&mut conn, table, column, bad).is_err(),
                "{table}.{column} accepted {bad}"
            );
        }
    }
    // A sold quantity is above zero; a ledger movement is signed, since a sale
    // takes stock out.
    for bad in ["0", "-1000", "1.5", "'abc'"] {
        assert!(
            probe(&mut conn, "document_lines", "qty_milli", bad).is_err(),
            "document_lines.qty_milli accepted {bad}"
        );
    }
    assert!(probe(&mut conn, "stock_movements", "qty_milli", "-1500").is_ok());
    clear(&mut conn, "stock_movements");
    for bad in ["1.5", "'abc'"] {
        assert!(
            probe(&mut conn, "stock_movements", "qty_milli", bad).is_err(),
            "stock_movements.qty_milli accepted {bad}"
        );
    }
}

#[test]
fn a_document_only_takes_the_kinds_regimes_modes_and_states_the_spec_names() {
    let (_dir, mut conn) = open_temp();
    seed_for_probes(&mut conn);
    let sets = [
        (
            "kind",
            vec![
                "'ticket'",
                "'facture'",
                "'proforma'",
                "'bon_de_livraison'",
                "'avoir'",
                "'bon_de_reception'",
                // Admitted since migration 4 so the stamped receipt the
                // comptable may ask for is not another table rebuild.
                // Nothing issues one.
                "'quittance'",
            ],
            vec!["'recu'", "''"],
        ),
        ("regime", vec!["'ifu'", "'reel'"], vec!["'forfait'", "''"]),
        (
            "payment_mode",
            vec!["'cash'", "'card'", "'credit'", "'cheque'", "'transfer'"],
            vec!["'bitcoin'", "''"],
        ),
        (
            "status",
            vec!["'issued'", "'cancelled'"],
            vec!["'draft'", "''"],
        ),
        // The snapshotted buyer kind. NULL is the ticket with no buyer at
        // all, which the template already covers.
        (
            "buyer_party_kind",
            vec!["'company'", "'consumer'"],
            vec!["'entreprise'", "''"],
        ),
    ];
    for (column, good, bad) in sets {
        for value in good {
            assert!(
                probe(&mut conn, "documents", column, value).is_ok(),
                "documents.{column} refused {value}"
            );
            clear(&mut conn, "documents");
        }
        for value in bad {
            assert!(
                probe(&mut conn, "documents", column, value).is_err(),
                "documents.{column} accepted {value}"
            );
        }
    }
    for value in [
        "'opening'",
        "'purchase'",
        "'sale'",
        "'adjustment'",
        "'return'",
    ] {
        assert!(
            probe(&mut conn, "stock_movements", "kind", value).is_ok(),
            "stock_movements.kind refused {value}"
        );
    }
    assert!(probe(&mut conn, "stock_movements", "kind", "'shrinkage'").is_err());
}

#[test]
fn a_number_is_unique_inside_its_series_and_free_in_another() {
    // features.md, Numbering row: one uninterrupted series per kind, numbers
    // never reused. The uniqueness is what makes the counter's gapless
    // promise checkable by the file itself.
    let (_dir, mut conn) = open_temp();
    let row = |series: &str, number: i64| {
        insert_with("documents", "series", &format!("'{series}'"))
            .replace(", 1, '2026-09-09", &format!(", {number}, '2026-09-09"))
    };
    assert!(diesel::sql_query(row("doc_ticket", 1))
        .execute(&mut conn)
        .is_ok());
    assert!(
        diesel::sql_query(row("doc_ticket", 1))
            .execute(&mut conn)
            .is_err(),
        "a ticket number came round twice"
    );
    assert!(
        diesel::sql_query(row("doc_facture", 1))
            .execute(&mut conn)
            .is_ok(),
        "the facture series must start at its own 1"
    );
    assert!(diesel::sql_query(row("doc_ticket", 2))
        .execute(&mut conn)
        .is_ok());
}

#[test]
fn a_database_at_the_first_migration_takes_the_second() {
    // architecture.md, Data: a migration ships with a test that opens a
    // database built by the previous ones and applies it.
    let (_dir, mut conn) = open_at_migration(1);
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'documents'"
        ),
        0,
        "the first migration already carries documents"
    );
    use diesel_migrations::MigrationHarness;
    conn.run_pending_migrations(dzpos_core::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'documents'"
        ),
        1
    );
    // The first migration's seeded rows survive: a shop that already sells is
    // what this migration runs on.
    assert_eq!(count(&mut conn, "SELECT COUNT(*) AS n FROM shops"), 1);
    assert_eq!(count(&mut conn, "SELECT COUNT(*) AS n FROM users"), 1);

    // And the upgraded file sells: the point of the migration is a till that
    // keeps working, not a schema that merely applies.
    use dzpos_core::models::product::{NewProduct, Unit};
    use dzpos_core::money::{Bps, Money, PaymentMode};
    use dzpos_core::services::sales::{NewSale, NewSaleLine, SaleKind};
    use dzpos_core::services::{products, sales};
    let product = products::create(
        &mut conn,
        1,
        1,
        NewProduct {
            name: "Sucre".to_string(),
            barcode: None,
            category_id: None,
            unit: Unit::Piece,
            cost: Money::centimes(500),
            selling: Money::centimes(1_000),
            wholesale: None,
            qty_on_hand_milli: 5_000,
            low_stock_at_milli: 0,
            rate_bps: Some(Bps::new(1900).unwrap()),
            active: true,
        },
    )
    .unwrap();
    let sale = sales::issue(
        &mut conn,
        1,
        1,
        NewSale {
            lines: vec![NewSaleLine {
                product_id: product.id,
                qty_milli: 2_000,
                unit_price: None,
                line_discount: Money::ZERO,
            }],
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Cash,
            tendered: Some(Money::centimes(5_000)),
            customer_id: None,
            override_credit: false,
            kind: SaleKind::Ticket,
            issued_at: None,
        },
    )
    .unwrap()
    .document;
    assert_eq!(sale.number, 1);
    assert_eq!(sale.totals.total_ht, Money::centimes(2_000));
    assert_eq!(
        products::get(&mut conn, 1, product.id)
            .unwrap()
            .qty_on_hand_milli,
        3_000
    );
}

#[test]
fn a_database_at_the_second_migration_takes_the_third() {
    // architecture.md, Data: a migration ships with a test that opens a
    // database built by the previous ones and applies it. The file this one
    // runs on already has a ledger, so the copy is what has to be proved, not
    // only the new foreign key.
    let (_dir, mut conn) = open_at_migration(2);
    assert_eq!(
        on_delete_from_shops(&mut conn, "stock_movements"),
        "CASCADE",
        "migration 2 is not the version this test claims to start from"
    );
    diesel::sql_query(
        "INSERT INTO products (id, shop_id, name, unit, cost_centimes, selling_centimes, \
         qty_on_hand_milli, low_stock_at_milli, rate_bps) \
         VALUES (7, 1, 'Sucre', 'piece', 0, 0, 4000, 0, 1900)",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO stock_movements (id, shop_id, product_id, kind, qty_milli, \
         unit_cost_centimes, user_id, created_at) \
         VALUES (4, 1, 7, 'opening', 4000, 820, 1, '2026-09-09 08:00:00')",
    )
    .execute(&mut conn)
    .unwrap();

    use diesel_migrations::MigrationHarness;
    conn.run_pending_migrations(dzpos_core::db::MIGRATIONS)
        .unwrap();

    assert_eq!(
        on_delete_from_shops(&mut conn, "stock_movements"),
        "RESTRICT"
    );
    // The row came across whole and kept its id: an id that moved would make
    // every ledger export written before this migration name a different row.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM stock_movements WHERE id = 4 AND shop_id = 1 \
             AND product_id = 7 AND kind = 'opening' AND qty_milli = 4000 \
             AND unit_cost_centimes = 820 AND document_id IS NULL AND user_id = 1 \
             AND created_at = '2026-09-09 08:00:00'"
        ),
        1,
        "the movement did not survive the table recreation unchanged"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'index' \
             AND name = 'idx_stock_movements_shop_product'"
        ),
        1,
        "the index went with the dropped table and was not put back"
    );
    // AUTOINCREMENT counts from sqlite_sequence, and that row is keyed by
    // table name: a rename that lost it would hand the next movement id 5
    // over again the day row 4 is deleted.
    diesel::sql_query(
        "INSERT INTO stock_movements (shop_id, product_id, kind, qty_milli, \
         unit_cost_centimes, user_id) VALUES (1, 7, 'sale', -1000, 820, 1)",
    )
    .execute(&mut conn)
    .unwrap();
    assert_eq!(
        count(&mut conn, "SELECT MAX(id) AS n FROM stock_movements"),
        5,
        "the sequence did not follow the table through the rename"
    );
    // And the upgraded file still sells: the ledger is what a sale writes to.
    assert!(
        diesel::sql_query("DELETE FROM shops WHERE id = 1")
            .execute(&mut conn)
            .is_err(),
        "a shop with a ledger and no document was deleted"
    );
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM stock_movements"),
        2
    );
}

#[test]
fn a_database_at_the_third_migration_takes_the_fourth() {
    // architecture.md, Data: a migration ships with a test that opens a
    // database built by the previous ones and applies it. This one rebuilds
    // `documents` to give `customer_id` the table it has been waiting for, and
    // document_lines, document_tva and stock_movements all point at
    // `documents.id`, so what has to be proved is that every child row is
    // still attached to the same parent afterwards.
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_at_migration(3);
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'customers'"
        ),
        0,
        "migration 3 is not the version this test claims to start from"
    );
    diesel::sql_query(
        "INSERT INTO products (id, shop_id, name, unit, cost_centimes, selling_centimes, \
         qty_on_hand_milli, low_stock_at_milli, rate_bps) \
         VALUES (7, 1, 'Sucre', 'piece', 0, 0, 4000, 0, 1900)",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO documents (id, shop_id, kind, series, number, issued_at, user_id, \
         regime, payment_mode, seller_name, total_ht_centimes, discount_centimes, \
         subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes, \
         net_to_pay_centimes, status) \
         VALUES (3, 1, 'ticket', 'doc_ticket', 12, '2026-09-09 10:00:00', 1, 'reel', \
         'cash', 'Mon magasin', 1000, 0, 1000, 190, 1190, 0, 1190, 'issued')",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO document_lines (id, shop_id, document_id, position, name, qty_milli, \
         unit_price_centimes, line_discount_centimes, rate_bps, line_total_centimes) \
         VALUES (5, 1, 3, 0, 'Sucre', 1000, 1000, 0, 1900, 1000)",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO document_lines (id, shop_id, document_id, position, name, qty_milli, \
         unit_price_centimes, line_discount_centimes, rate_bps, line_total_centimes) \
         VALUES (8, 1, 3, 1, 'Café', 2000, 500, 0, 900, 1000)",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO document_tva (id, shop_id, document_id, rate_bps, base_centimes, \
         amount_centimes) VALUES (6, 1, 3, 1900, 1000, 190)",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO stock_movements (id, shop_id, product_id, kind, qty_milli, \
         unit_cost_centimes, document_id, user_id) VALUES (9, 1, 7, 'sale', -1000, 0, 3, 1)",
    )
    .execute(&mut conn)
    .unwrap();

    conn.run_pending_migrations(dzpos_core::db::MIGRATIONS)
        .unwrap();

    assert_eq!(
        orphan_rows(&mut conn),
        0,
        "the rebuilt file has a row pointing at a parent that is not there"
    );

    // The document came across whole, keeping the id and the number the paper
    // in the customer's hand carries.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 3 AND number = 12 \
             AND series = 'doc_ticket' AND net_to_pay_centimes = 1190 \
             AND buyer_name IS NULL AND old_balance_centimes IS NULL \
             AND ref_document_id IS NULL"
        ),
        1,
        "the document did not survive the table rebuild unchanged"
    );
    // Every child kept its own id and its parent. A cascade that fired while
    // the old table was dropped would show up as a zero here.
    for (table, id) in [
        ("document_lines", 5),
        ("document_lines", 8),
        ("document_tva", 6),
    ] {
        assert_eq!(
            count(
                &mut conn,
                &format!("SELECT COUNT(*) AS n FROM {table} WHERE id = {id} AND document_id = 3")
            ),
            1,
            "{table} lost its row when documents was rebuilt"
        );
    }
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM stock_movements WHERE id = 9 AND document_id = 3"
        ),
        1,
        "the stock movement lost the document that moved it"
    );
    // The two foreign keys the rebuild existed for.
    let keys: Vec<Name> = diesel::sql_query(
        "SELECT \"table\" || '.' || \"from\" || '.' || on_delete AS name \
         FROM pragma_foreign_key_list('documents') ORDER BY name",
    )
    .load(&mut conn)
    .unwrap();
    let keys: Vec<String> = keys.into_iter().map(|r| r.name).collect();
    assert!(
        keys.contains(&"customers.customer_id.RESTRICT".to_string()),
        "customer_id still has no foreign key: {keys:?}"
    );
    assert!(
        keys.contains(&"documents.ref_document_id.RESTRICT".to_string()),
        "ref_document_id still has no foreign key: {keys:?}"
    );
    // The `PRAGMA foreign_keys = OFF` the rebuild needs must not outlive it,
    // or every write after the upgrade would land unchecked.
    let pragma: Pragma = diesel::sql_query("PRAGMA foreign_keys")
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(
        pragma.foreign_keys, 1,
        "the migration left the foreign keys off"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'index' \
             AND name = 'idx_documents_shop_issued'"
        ),
        1,
        "the index went with the dropped table and was not put back"
    );
    // AUTOINCREMENT counts from sqlite_sequence, keyed by table name: a
    // rename that lost the row would hand number 3 out again.
    diesel::sql_query(
        "INSERT INTO documents (shop_id, kind, series, number, issued_at, user_id, \
         regime, payment_mode, seller_name, total_ht_centimes, discount_centimes, \
         subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes, \
         net_to_pay_centimes, status) \
         VALUES (1, 'ticket', 'doc_ticket', 13, '2026-09-09 10:01:00', 1, 'reel', \
         'cash', 'Mon magasin', 0, 0, 0, 0, 0, 0, 0, 'issued')",
    )
    .execute(&mut conn)
    .unwrap();
    assert_eq!(
        count(&mut conn, "SELECT MAX(id) AS n FROM documents"),
        4,
        "the sequence did not follow the table through the rename"
    );

    // And the upgraded file carries a customer with an opening debt, read
    // back as the balance a statement would print.
    diesel::sql_query(
        "INSERT INTO customers (id, shop_id, name, party_kind, credit_limit_centimes) \
         VALUES (1, 1, 'Ahmed Benali', 'company', 5000000)",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO debt_ledger (shop_id, customer_id, kind, debit_centimes, \
         credit_centimes, user_id, note) \
         VALUES (1, 1, 'opening', 250000, 0, 1, 'report ancien carnet')",
    )
    .execute(&mut conn)
    .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT SUM(debit_centimes - credit_centimes) AS n FROM debt_ledger \
             WHERE shop_id = 1 AND customer_id = 1"
        ),
        250000
    );
    // A document may now name that customer, which is what the rebuild was
    // for, and the customer cannot then be deleted out from under it.
    diesel::sql_query("UPDATE documents SET customer_id = 1 WHERE id = 3")
        .execute(&mut conn)
        .unwrap();
    assert!(
        diesel::sql_query("DELETE FROM customers WHERE id = 1")
            .execute(&mut conn)
            .is_err(),
        "a customer named on a document was deleted"
    );
}

#[test]
fn a_database_without_the_payment_mode_column_takes_the_migration_that_adds_it() {
    // architecture.md, Data: a migration ships with a test that opens a
    // database built by the previous ones and applies it. This one adds one
    // nullable column to a table that already holds movements, so what has to
    // be proved is that the movements are still there, still say what they
    // said, and read as no payment mode at all rather than as a made-up one.
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_before_migration("debt_payment_mode");
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('debt_ledger') \
             WHERE name = 'payment_mode'"
        ),
        0,
        "the file this starts from already carries the column"
    );
    diesel::sql_query(
        "INSERT INTO customers (id, shop_id, name, party_kind) VALUES (1, 1, 'Ahmed', 'consumer')",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO debt_ledger (id, shop_id, customer_id, kind, debit_centimes, \
         credit_centimes, user_id, note) \
         VALUES (5, 1, 1, 'opening', 250000, 0, 1, 'report ancien carnet')",
    )
    .execute(&mut conn)
    .unwrap();

    let pending = conn.pending_migrations(dzpos_core::db::MIGRATIONS).unwrap();
    conn.run_migration(&pending[0]).unwrap();

    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM debt_ledger WHERE id = 5 AND kind = 'opening' \
             AND debit_centimes = 250000 AND note = 'report ancien carnet' \
             AND payment_mode IS NULL"
        ),
        1,
        "the movement the file already carried did not survive the new column"
    );
    // The column takes the two ways a payment is taken and nothing else: a
    // 'credit' here would be a payment settled with more credit.
    for mode in ["cash", "card"] {
        assert_eq!(
            diesel::sql_query(format!(
                "INSERT INTO debt_ledger (shop_id, customer_id, kind, debit_centimes, \
                 credit_centimes, user_id, payment_mode) \
                 VALUES (1, 1, 'payment', 0, 1000, 1, '{mode}')"
            ))
            .execute(&mut conn)
            .unwrap(),
            1,
            "a payment in {mode} was refused"
        );
    }
    assert!(
        diesel::sql_query(
            "INSERT INTO debt_ledger (shop_id, customer_id, kind, debit_centimes, \
             credit_centimes, user_id, payment_mode) \
             VALUES (1, 1, 'payment', 0, 1000, 1, 'credit')"
        )
        .execute(&mut conn)
        .is_err(),
        "the column took a mode that is not a way of paying"
    );
    assert_eq!(orphan_rows(&mut conn), 0);
}

/// Customer 1, facture 4 made out to them with a buyer block and a balance
/// triple, and the ledger movement the sale wrote. Every figure here is read
/// back by the tests that revert the migration.
fn seed_a_facture_naming_a_customer(conn: &mut SqliteConnection) {
    diesel::sql_query(insert_with("customers", "name", "'Entreprise Benali'"))
        .execute(conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO documents (id, shop_id, kind, series, number, issued_at, user_id, \
         regime, payment_mode, seller_name, customer_id, buyer_name, buyer_party_kind, \
         buyer_rc, buyer_nif, total_ht_centimes, discount_centimes, subtotal_ht_centimes, \
         tva_centimes, total_ttc_centimes, stamp_centimes, net_to_pay_centimes, \
         old_balance_centimes, remaining_debt_centimes, total_debt_centimes) \
         VALUES (4, 1, 'facture', 'doc_facture', 7, '2026-09-09 10:00:00', 1, 'reel', \
         'credit', 'Mon magasin', 1, 'Entreprise Benali', 'company', \
         '16/00-7654321 B 22', '000216007654321', 100000, 0, 100000, 19000, 119000, 0, \
         119000, 250000, 119000, 369000)",
    )
    .execute(conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO debt_ledger (shop_id, customer_id, document_id, kind, debit_centimes, \
         credit_centimes, user_id) VALUES (1, 1, 4, 'sale', 119000, 0, 1)",
    )
    .execute(conn)
    .unwrap();
}

#[test]
fn a_database_without_the_cancellation_columns_takes_the_migration_that_adds_them() {
    // architecture.md, Data: a migration ships with a test that opens a
    // database built by the previous ones and applies it. This one adds four
    // nullable columns to a table that already holds documents, so what has
    // to be proved is that the documents are still there, still say what they
    // said, and read as never cancelled rather than as cancelled by nobody.
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_before_migration("cancellation");
    for column in [
        "cancelled_at",
        "cancelled_by",
        "cancel_reason",
        "cancel_avoir_document_id",
    ] {
        assert_eq!(
            count(
                &mut conn,
                &format!(
                    "SELECT COUNT(*) AS n FROM pragma_table_info('documents') \
                     WHERE name = '{column}'"
                )
            ),
            0,
            "the file this starts from already carries {column}"
        );
    }
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('document_lines') \
             WHERE name = 'ref_line_id'"
        ),
        0,
        "the file this starts from already carries ref_line_id"
    );
    seed_a_facture_naming_a_customer(&mut conn);
    diesel::sql_query(
        "INSERT INTO document_lines (id, shop_id, document_id, position, name, qty_milli, \
         unit_price_centimes, line_discount_centimes, rate_bps, line_total_centimes) \
         VALUES (9, 1, 4, 0, 'Ciment', 1000, 100000, 0, 1900, 100000)",
    )
    .execute(&mut conn)
    .unwrap();

    let pending = conn.pending_migrations(dzpos_core::db::MIGRATIONS).unwrap();
    conn.run_migration(&pending[0]).unwrap();

    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 4 AND number = 7 \
             AND net_to_pay_centimes = 119000 AND status = 'issued' \
             AND cancelled_at IS NULL AND cancelled_by IS NULL \
             AND cancel_reason IS NULL AND cancel_avoir_document_id IS NULL"
        ),
        1,
        "the facture the file already carried did not survive the new columns"
    );
    // A line the file already carried credits nothing, which is what every
    // line of a ticket, a facture and a proforma says.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM document_lines WHERE id = 9 AND qty_milli = 1000 \
             AND line_total_centimes = 100000 AND ref_line_id IS NULL"
        ),
        1,
        "the line the file already carried did not survive the new column"
    );
    // The facture line an avoir line credits is a line of this file.
    assert!(
        diesel::sql_query("UPDATE document_lines SET ref_line_id = 999 WHERE id = 9")
            .execute(&mut conn)
            .is_err(),
        "the credited-line reference took an id no line has"
    );

    // A cancellation is written whole: the day, the person and the reason
    // travel together, because a document marked annulée with nobody's name
    // on it is exactly what features.md §5 keeps a log against. Which half of
    // a half-written block is missing is the model's answer (`cancellation`
    // reads it back the way `buyer_block` and `balance_triple` do), so the
    // columns only have to take the whole of one.
    assert_eq!(
        diesel::sql_query(
            "UPDATE documents SET status = 'cancelled', cancelled_at = '2026-09-10 09:15:00', \
             cancelled_by = 1, cancel_reason = 'erreur de saisie' WHERE id = 4"
        )
        .execute(&mut conn)
        .unwrap(),
        1,
        "a whole cancellation was refused"
    );
    // The person who cancelled is a user of this file and the avoir a
    // cancellation issued is a document of it, so both columns name a row
    // rather than holding a number somebody typed.
    for bad in ["cancelled_by = 999", "cancel_avoir_document_id = 999"] {
        assert!(
            diesel::sql_query(format!("UPDATE documents SET {bad} WHERE id = 4"))
                .execute(&mut conn)
                .is_err(),
            "the cancellation block took an id no row has: {bad}"
        );
    }
    // Who cancelled is an id and never a name typed into the wrong box. A
    // STRICT table converts a number into text for the two TEXT columns, so
    // the integer one is where the refusal shows (STRICT tables,
    // architecture.md, Data).
    assert!(
        diesel::sql_query("UPDATE documents SET cancelled_by = 'Ahmed' WHERE id = 4")
            .execute(&mut conn)
            .is_err(),
        "the cancellation block took a name where it holds a user id"
    );
    assert_eq!(orphan_rows(&mut conn), 0);
}

#[test]
fn the_migration_reverts_and_reapplies() {
    // architecture.md, Data: a migration ships with a test that runs it.
    // Reverting all the way back to an empty file is what proves each
    // down.sql undoes its own up.sql and nothing else.
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_temp();
    // A revert on an empty file proves the tables move and says nothing about
    // the rows: the down.sql copies documents back the way the up.sql copied
    // them across, and a facture with a buyer, a balance and a debt behind it
    // is what that copy has to carry.
    seed_a_facture_naming_a_customer(&mut conn);

    conn.revert_last_migration(dzpos_core::db::MIGRATIONS)
        .unwrap();
    // The seventh one rebuilt the ledger to hang a two-column CHECK on it, so
    // its down rebuilds it once more without that CHECK. The column stays: it
    // belongs to the fifth migration and comes off with the fifth
    // migration's down, below.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' \
             AND name = 'debt_ledger' AND sql LIKE '%OR kind = ''payment''%'"
        ),
        0,
        "the check down.sql left the check behind"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('debt_ledger') \
             WHERE name = 'payment_mode'"
        ),
        1,
        "the check down.sql took the column the fifth migration added"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM debt_ledger WHERE customer_id = 1 AND kind = 'sale'"
        ),
        1,
        "the check down.sql took a movement with it"
    );

    conn.revert_last_migration(dzpos_core::db::MIGRATIONS)
        .unwrap();
    // The sixth one only adds columns, so its down takes the five and leaves
    // every document standing with the number a comptable reads and the buyer
    // block it was made out to.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('document_lines') \
             WHERE name = 'ref_line_id'"
        ),
        0,
        "the cancellation down.sql left ref_line_id behind"
    );
    for column in [
        "cancelled_at",
        "cancelled_by",
        "cancel_reason",
        "cancel_avoir_document_id",
    ] {
        assert_eq!(
            count(
                &mut conn,
                &format!(
                    "SELECT COUNT(*) AS n FROM pragma_table_info('documents') \
                     WHERE name = '{column}'"
                )
            ),
            0,
            "the cancellation down.sql left {column} behind"
        );
    }
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 4 AND number = 7 \
             AND series = 'doc_facture' AND net_to_pay_centimes = 119000 \
             AND customer_id = 1 AND buyer_name = 'Entreprise Benali'"
        ),
        1,
        "the cancellation down.sql took the facture or its buyer block with the columns"
    );

    conn.revert_last_migration(dzpos_core::db::MIGRATIONS)
        .unwrap();
    // The fifth one only adds a column, so its down takes the column and
    // leaves every movement standing: the ledger row the facture wrote is
    // still there with the id the allocations would name.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('debt_ledger') \
             WHERE name = 'payment_mode'"
        ),
        0,
        "the payment mode down.sql left its column behind"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM debt_ledger WHERE customer_id = 1 AND kind = 'sale'"
        ),
        1,
        "the payment mode down.sql took a movement with the column"
    );

    conn.revert_last_migration(dzpos_core::db::MIGRATIONS)
        .unwrap();
    // The facture is still there, keeping its id, its number and the totals a
    // comptable reads. The buyer block and the balance are gone with the
    // columns that held them, and `customer_id` is emptied on purpose: the
    // customers table goes with them, so a kept id would name nothing. A
    // downgrade loses which customer a document was made out to.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 4 AND number = 7 \
             AND series = 'doc_facture' AND net_to_pay_centimes = 119000 \
             AND customer_id IS NULL AND seller_name = 'Mon magasin'"
        ),
        1,
        "the facture did not survive the down copy"
    );
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM documents"),
        1,
        "the down copy left a document behind or wrote one twice"
    );
    // The fourth one rebuilds documents, so its down has to rebuild it again
    // and put the three tables it added away, without taking documents with
    // them.
    for table in ["customers", "debt_ledger", "debt_allocations"] {
        assert_eq!(
            count(
                &mut conn,
                &format!(
                    "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' \
                     AND name = '{table}'"
                )
            ),
            0,
            "the customers down.sql left {table} behind"
        );
    }
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('documents') \
             WHERE name = 'buyer_name'"
        ),
        0,
        "the customers down.sql left the buyer block on documents"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'documents'"
        ),
        1,
        "the customers down.sql took the second migration's tables with it"
    );
    let pragma: Pragma = diesel::sql_query("PRAGMA foreign_keys")
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(
        pragma.foreign_keys, 1,
        "the down.sql left the foreign keys off"
    );
    conn.revert_last_migration(dzpos_core::db::MIGRATIONS)
        .unwrap();
    // The third one only recreates stock_movements, so its down leaves the
    // second migration's tables standing and puts the CASCADE back.
    assert_eq!(
        on_delete_from_shops(&mut conn, "stock_movements"),
        "CASCADE",
        "the restrict down.sql did not put migration 2's foreign key back"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'documents'"
        ),
        1,
        "the restrict down.sql took the second migration's tables with it"
    );
    conn.revert_last_migration(dzpos_core::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'documents'"
        ),
        0,
        "the documents down.sql left its tables behind"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'products'"
        ),
        1,
        "the documents down.sql took the first migration's tables with it"
    );
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
    assert_eq!(
        orphan_rows(&mut conn),
        0,
        "the reapplied file has a row pointing at a parent that is not there"
    );
    assert_eq!(count(&mut conn, "SELECT COUNT(*) AS n FROM shops"), 1);
    assert_eq!(
        on_delete_from_shops(&mut conn, "stock_movements"),
        "RESTRICT",
        "the file reapplied to a schema the third migration had already fixed"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'customers'"
        ),
        1,
        "the fourth migration did not reapply"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('debt_ledger') \
             WHERE name = 'payment_mode'"
        ),
        1,
        "the fifth migration did not reapply"
    );
}

#[test]
fn every_ledger_table_restricts_the_shop_it_belongs_to() {
    // A shop that has sold, been audited or moved stock is never deleted:
    // décret 05-468 art. 10 for the series, ISO 27001 for the trail, and
    // features.md §1 for the ledger the stock on hand is only a cache of.
    // stock_movements shipped as CASCADE in migration 2 and migration 3 is
    // what put it with the others. The debt ledger and its allocations joined
    // them in migration 4, and the customers they name with them: a balance a
    // DELETE can empty is not a balance either.
    let (_dir, mut conn) = open_temp();
    for table in [
        "documents",
        "audit_log",
        "stock_movements",
        "customers",
        "debt_ledger",
        "debt_allocations",
    ] {
        assert_eq!(
            on_delete_from_shops(&mut conn, table),
            "RESTRICT",
            "{table} lets a shop delete take it"
        );
    }
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

#[test]
fn a_product_with_a_movement_cannot_be_deleted() {
    // The ledger is the truth about what left the shelf (features.md §1) and a
    // document line keeps its own snapshot of the product, so a delete that
    // took the movements with it would leave the sold lines standing and the
    // stock they came out of gone. M3 archives a product instead.
    let (_dir, mut conn) = open_temp();
    seed_for_probes(&mut conn);
    diesel::sql_query(insert_with("stock_movements", "kind", "'sale'"))
        .execute(&mut conn)
        .unwrap();
    let deleted = diesel::sql_query("DELETE FROM products WHERE id = 1").execute(&mut conn);
    assert!(
        deleted.is_err(),
        "a product with a movement was deleted and took its ledger with it"
    );
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM stock_movements"),
        1
    );
}

#[test]
fn a_shop_with_a_document_an_audit_entry_or_a_movement_cannot_be_deleted() {
    // A fiscal document, the audit trail and the stock ledger outlive the row
    // that points at them: décret 05-468 art. 10 wants an uninterrupted
    // series, and a series a DELETE can empty is not one. A shop whose only
    // rows are opening movements is the case the third migration is for: it
    // has no document yet and its ledger is still what the stock it reports
    // is made of. The second shop carries only the row under test, so each FK
    // is the reason its own delete fails.
    //
    // `is_err()` alone would stay green on a CHECK, a locked file or a typo in
    // the DELETE, so the reason is asserted and the row is counted after: the
    // point is not that the statement failed, it is that the ledger is still
    // there. diesel maps the bundled SQLite's foreign key error to
    // `DatabaseErrorKind::Unknown` rather than `ForeignKeyViolation`, so the
    // engine's own sentence is what names the reason here.
    use diesel::result::Error as DieselError;
    let (_dir, mut conn) = open_temp();
    seed_for_probes(&mut conn);
    for (what, table, insert) in [
        (
            "a document",
            "documents",
            "INSERT INTO documents (shop_id, kind, series, number, issued_at, user_id, \
             regime, payment_mode, seller_name, total_ht_centimes, discount_centimes, \
             subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes, \
             net_to_pay_centimes, status) \
             VALUES (2, 'ticket', 'doc_ticket', 1, '2026-09-09 10:00:00', 1, 'reel', \
             'cash', 'Autre magasin', 0, 0, 0, 0, 0, 0, 0, 'issued')",
        ),
        (
            "an audit entry",
            "audit_log",
            "INSERT INTO audit_log (shop_id, user_id, action, entity, entity_id) \
             VALUES (2, 1, 'update', 'product', 1)",
        ),
        (
            "an opening movement",
            "stock_movements",
            "INSERT INTO stock_movements (shop_id, product_id, kind, qty_milli, \
             unit_cost_centimes, user_id) VALUES (2, 1, 'opening', 1000, 0, 1)",
        ),
    ] {
        diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
            .execute(&mut conn)
            .unwrap();
        diesel::sql_query(insert).execute(&mut conn).unwrap();
        let deleted = diesel::sql_query("DELETE FROM shops WHERE id = 2").execute(&mut conn);
        let err = deleted
            .err()
            .unwrap_or_else(|| panic!("a shop was deleted and took {what} with it"));
        let reason = match &err {
            DieselError::DatabaseError(_, info) => info.message().to_string(),
            other => panic!("{what}: the delete failed outside the database: {other:?}"),
        };
        assert_eq!(
            reason, "FOREIGN KEY constraint failed",
            "{what}: the delete failed for something other than the foreign key"
        );
        assert_eq!(
            count(
                &mut conn,
                &format!("SELECT COUNT(*) AS n FROM {table} WHERE shop_id = 2")
            ),
            1,
            "{what} did not survive the refused delete"
        );
        assert_eq!(
            count(&mut conn, "SELECT COUNT(*) AS n FROM shops WHERE id = 2"),
            1,
            "the shop row went even though the delete was refused"
        );
        for cleanup in [
            "DELETE FROM stock_movements WHERE shop_id = 2",
            "DELETE FROM documents WHERE shop_id = 2",
            "DELETE FROM audit_log WHERE shop_id = 2",
            "DELETE FROM shops WHERE id = 2",
        ] {
            diesel::sql_query(cleanup).execute(&mut conn).unwrap();
        }
    }
}

#[test]
fn the_debt_tables_keep_what_they_name_and_lose_only_what_they_may() {
    // The four foreign key actions migration 4 chose, each read off the file
    // rather than off the schema: a customer with a history cannot be
    // deleted, a payment and a document an allocation settles cannot be
    // deleted, and a document a movement merely cites goes away leaving the
    // movement behind, because what the customer owes is not a fact about
    // the document that caused it.
    let (_dir, mut conn) = open_temp();
    seed_for_probes(&mut conn);
    diesel::sql_query(insert_with("customers", "name", "'Entreprise Benali'"))
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO debt_ledger (id, shop_id, customer_id, document_id, kind, \
         debit_centimes, credit_centimes, user_id) \
         VALUES (1, 1, 1, 1, 'sale', 100000, 0, 1)",
    )
    .execute(&mut conn)
    .unwrap();

    assert!(
        diesel::sql_query("DELETE FROM customers WHERE id = 1")
            .execute(&mut conn)
            .is_err(),
        "a customer with a ledger behind them was deleted"
    );

    // The sale is deleted; what the customer owes for it is not.
    diesel::sql_query("DELETE FROM documents WHERE id = 1")
        .execute(&mut conn)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM debt_ledger WHERE id = 1 AND document_id IS NULL \
             AND debit_centimes = 100000"
        ),
        1,
        "the movement went with the document instead of losing its id"
    );

    // A payment and the document it settled, this time with the allocation
    // that ties them together.
    diesel::sql_query(insert_with("documents", "number", "2"))
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO debt_ledger (id, shop_id, customer_id, kind, debit_centimes, \
         credit_centimes, user_id) VALUES (2, 1, 1, 'payment', 0, 50000, 1)",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO debt_allocations (shop_id, payment_ledger_id, document_id, \
         amount_centimes) VALUES (1, 2, 2, 50000)",
    )
    .execute(&mut conn)
    .unwrap();

    assert!(
        diesel::sql_query("DELETE FROM debt_ledger WHERE id = 2")
            .execute(&mut conn)
            .is_err(),
        "a payment an allocation settles with was deleted"
    );
    assert!(
        diesel::sql_query("DELETE FROM documents WHERE id = 2")
            .execute(&mut conn)
            .is_err(),
        "a document an allocation settled was deleted"
    );
    assert_eq!(orphan_rows(&mut conn), 0);
}

#[test]
fn a_facture_naming_a_customer_goes_down_and_up_without_orphaning_itself() {
    // The round trip above walks the file all the way back, which empties it
    // before anything is reapplied. This one reverts the fourth migration
    // only and puts it straight back, which is the move a developer makes and
    // the one where a kept `customer_id` would come back up pointing into a
    // `customers` table that was just recreated empty.
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_temp();
    seed_a_facture_naming_a_customer(&mut conn);

    // The fifth, sixth and seventh migrations sit on top of the fourth and
    // only touch tables the fourth left standing, so all three come off
    // first; the round trip above is where those steps are asserted.
    conn.revert_last_migration(dzpos_core::db::MIGRATIONS)
        .unwrap();
    conn.revert_last_migration(dzpos_core::db::MIGRATIONS)
        .unwrap();
    conn.revert_last_migration(dzpos_core::db::MIGRATIONS)
        .unwrap();
    conn.revert_last_migration(dzpos_core::db::MIGRATIONS)
        .unwrap();
    // Everything the fourth migration added to `documents` is off the table
    // again: the seven columns of the buyer block and the three of the
    // balance triple.
    for column in [
        "buyer_name",
        "buyer_party_kind",
        "buyer_rc",
        "buyer_nif",
        "buyer_nis",
        "buyer_ai",
        "buyer_address",
        "old_balance_centimes",
        "remaining_debt_centimes",
        "total_debt_centimes",
    ] {
        assert_eq!(
            count(
                &mut conn,
                &format!(
                    "SELECT COUNT(*) AS n FROM pragma_table_info('documents') \
                     WHERE name = '{column}'"
                )
            ),
            0,
            "the down.sql left {column} on documents"
        );
    }

    conn.run_pending_migrations(dzpos_core::db::MIGRATIONS)
        .unwrap();

    // The money came back untouched. The two copies are where a column could
    // quietly land in the wrong place, and a total off by a copy is a facture
    // that no longer matches the paper the customer holds.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 4 \
             AND total_ht_centimes = 100000 AND tva_centimes = 19000 \
             AND total_ttc_centimes = 119000 AND net_to_pay_centimes = 119000"
        ),
        1,
        "the totals are not what was seeded before the round trip"
    );
    assert_eq!(
        orphan_rows(&mut conn),
        0,
        "the reapplied file has a row pointing at a customer that is not there"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 4 AND number = 7 \
             AND customer_id IS NULL"
        ),
        1,
        "the facture kept a customer id the down.sql had no customer to keep"
    );
}

#[test]
fn a_database_whose_ledger_lets_any_movement_carry_a_mode_takes_the_check() {
    // architecture.md, Data: a migration ships with a test that opens a
    // database built by the previous ones and applies it. This one rebuilds a
    // table that already holds movements, so what has to be proved is that
    // every one of them is still there with the id it had, that the
    // allocations still name the same movements, and that the row migration
    // 4's comment forbids is now refused by the file itself.
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_before_migration("debt_payment_mode_check");
    diesel::sql_query(
        "INSERT INTO customers (id, shop_id, name, party_kind) VALUES (1, 1, 'Ahmed', 'consumer')",
    )
    .execute(&mut conn)
    .unwrap();
    // A document for the allocation to point at, written straight in: what
    // this asserts is the ledger, not how a facture gets issued.
    diesel::sql_query(
        "INSERT INTO documents (id, shop_id, kind, series, number, issued_at, user_id, \
         regime, payment_mode, seller_name, total_ht_centimes, discount_centimes, \
         subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes, \
         net_to_pay_centimes) VALUES (1, 1, 'facture', 'doc_facture', 1, \
         '2026-09-09 10:00:00', 1, 'reel', 'credit', 'Magasin', 100000, 0, 100000, \
         19000, 119000, 0, 119000)",
    )
    .execute(&mut conn)
    .unwrap();
    // The file this starts from takes the row the new CHECK refuses, which is
    // what makes the rebuild worth running: an opening balance stamped 'cash'
    // reads as money that was handed over and never was.
    assert_eq!(
        diesel::sql_query(
            "INSERT INTO debt_ledger (id, shop_id, customer_id, kind, debit_centimes, \
             credit_centimes, user_id, payment_mode) \
             VALUES (4, 1, 1, 'opening', 1000, 0, 1, 'cash')"
        )
        .execute(&mut conn)
        .unwrap(),
        1,
        "the file this starts from already refuses the row"
    );
    diesel::sql_query("DELETE FROM debt_ledger WHERE id = 4")
        .execute(&mut conn)
        .unwrap();
    // An opening with no mode, a payment with one, and an allocation naming
    // that payment by its id.
    diesel::sql_query(
        "INSERT INTO debt_ledger (id, shop_id, customer_id, kind, debit_centimes, \
         credit_centimes, user_id, note) \
         VALUES (5, 1, 1, 'opening', 250000, 0, 1, 'report ancien carnet')",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO debt_ledger (id, shop_id, customer_id, kind, debit_centimes, \
         credit_centimes, user_id, payment_mode) \
         VALUES (6, 1, 1, 'payment', 0, 100000, 1, 'card')",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO debt_allocations (shop_id, payment_ledger_id, document_id, amount_centimes) \
         VALUES (1, 6, 1, 100000)",
    )
    .execute(&mut conn)
    .unwrap();

    let pending = conn.pending_migrations(dzpos_core::db::MIGRATIONS).unwrap();
    conn.run_migration(&pending[0]).unwrap();

    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM debt_ledger WHERE id = 5 AND kind = 'opening' \
             AND debit_centimes = 250000 AND note = 'report ancien carnet' \
             AND payment_mode IS NULL"
        ),
        1,
        "the movement the file already carried did not survive the rebuild"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM debt_ledger WHERE id = 6 AND kind = 'payment' \
             AND credit_centimes = 100000 AND payment_mode = 'card'"
        ),
        1,
        "the payment did not survive the rebuild with its mode"
    );
    // The ids are what the allocations point at, so a rebuild that reassigned
    // them would say a facture had been settled by somebody else's payment.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM debt_allocations WHERE payment_ledger_id = 6"
        ),
        1,
        "the allocation lost the payment it names"
    );
    assert_eq!(orphan_rows(&mut conn), 0);
    // The index goes with the table when it is dropped, so it is written
    // again: a balance reads one customer's rows through it.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'index' \
             AND name = 'idx_debt_ledger_shop_customer'"
        ),
        1,
        "the rebuilt table lost its index"
    );

    // What the file now refuses on its own: a mode on a movement nobody
    // handed money over for.
    for kind in ["opening", "sale", "avoir", "adjustment"] {
        assert!(
            diesel::sql_query(format!(
                "INSERT INTO debt_ledger (shop_id, customer_id, kind, debit_centimes, \
                 credit_centimes, user_id, payment_mode) \
                 VALUES (1, 1, '{kind}', 1000, 0, 1, 'cash')"
            ))
            .execute(&mut conn)
            .is_err(),
            "a {kind} was stamped with a payment mode"
        );
    }
    // And what it still takes. A payment with the mode `pay` fills in, every
    // other kind without one, and a payment with none: money settled out of
    // credit the customer was already holding was handed over in nothing, and
    // the table does not make one up for it.
    for (kind, mode) in [
        ("payment", "'cash'"),
        ("payment", "'card'"),
        ("payment", "NULL"),
        ("sale", "NULL"),
        ("avoir", "NULL"),
    ] {
        assert_eq!(
            diesel::sql_query(format!(
                "INSERT INTO debt_ledger (shop_id, customer_id, kind, debit_centimes, \
                 credit_centimes, user_id, payment_mode) \
                 VALUES (1, 1, '{kind}', 1000, 0, 1, {mode})"
            ))
            .execute(&mut conn)
            .unwrap(),
            1,
            "a {kind} paid {mode} was refused"
        );
    }
    // The per-column check migration 4 wrote is still on the rebuilt table.
    assert!(
        diesel::sql_query(
            "INSERT INTO debt_ledger (shop_id, customer_id, kind, debit_centimes, \
             credit_centimes, user_id, payment_mode) \
             VALUES (1, 1, 'payment', 0, 1000, 1, 'credit')"
        )
        .execute(&mut conn)
        .is_err(),
        "the rebuilt table took a mode that is not a way of paying"
    );
}
