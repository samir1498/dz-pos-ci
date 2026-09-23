// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The first migration is the schema every later feature builds on, so the
//! test asserts the shape by name: the tables, `shop_id` on each of them,
//! the seeded shop with its owner and its dated régime fiscal.

use diesel::prelude::*;
use diesel::sql_types::{Integer, Text};

#[derive(QueryableByName)]
struct Pragma {
    #[diesel(sql_type = Integer)]
    foreign_keys: i32,
}

mod common;

use common::migrations::{
    count, insert_with, insert_with_all, on_delete_from_shops, open_at_migration,
    open_before_migration, orphan_rows, probe, Name,
};
use common::open_temp;

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
            "appointments",
            "audit_log",
            "cash_refunds",
            "categories",
            "counters",
            "customers",
            "debt_allocations",
            "debt_ledger",
            "document_lines",
            "document_tva",
            "documents",
            "expense_categories",
            "expenses",
            "jobs",
            "paired_devices",
            "pairing_tokens",
            "patients",
            "preferences",
            "products",
            "purchase_lines",
            "purchase_receipt_lines",
            "purchase_receipts",
            "purchases",
            "queue_entries",
            "sale_idempotency_keys",
            "sessions",
            "settings",
            "shifts",
            "shops",
            "stock_movements",
            "supplier_allocations",
            "supplier_ledger",
            "suppliers",
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
        "appointments",
        "audit_log",
        "categories",
        "counters",
        "customers",
        "debt_allocations",
        "debt_ledger",
        "document_lines",
        "document_tva",
        "documents",
        "expense_categories",
        "expenses",
        "jobs",
        "paired_devices",
        "pairing_tokens",
        "patients",
        "preferences",
        "products",
        "purchase_lines",
        "purchase_receipt_lines",
        "purchase_receipts",
        "purchases",
        "queue_entries",
        "sale_idempotency_keys",
        "sessions",
        "settings",
        "shifts",
        "stock_movements",
        "supplier_allocations",
        "supplier_ledger",
        "suppliers",
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
        "preferences",
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
        "suppliers",
        "purchases",
        "purchase_lines",
        "purchase_receipts",
        "purchase_receipt_lines",
        "supplier_ledger",
        "supplier_allocations",
        "expense_categories",
        "expenses",
        "jobs",
        "paired_devices",
        "pairing_tokens",
        "sale_idempotency_keys",
        "sessions",
        "shifts",
        "patients",
        "queue_entries",
        "appointments",
    ] {
        let strict = count(
            &mut conn,
            &format!("SELECT strict AS n FROM pragma_table_list WHERE name = '{table}'"),
        );
        assert_eq!(strict, 1, "{table} is not STRICT");
    }
}
#[test]
fn every_money_and_rate_column_refuses_a_real_a_text_and_a_negative() {
    // Rule 6: money is integer centimes. 19.99 was once read back as
    // Money(19). These are the `products` columns and the two counters; a
    // table with its own migration test probes its columns there instead.
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
        // The same marker on the supply side: the seeded supplier, purchase,
        // receipt and ledger row are what the probes above them point at and
        // each of those keys RESTRICTs, so a blanket DELETE would fail rather
        // than clear.
        "suppliers" => "DELETE FROM suppliers WHERE name <> 'seed'".to_string(),
        "purchases" => "DELETE FROM purchases WHERE note IS NULL".to_string(),
        "purchase_receipts" => "DELETE FROM purchase_receipts WHERE note IS NULL".to_string(),
        "supplier_ledger" => "DELETE FROM supplier_ledger WHERE note IS NULL".to_string(),
        // A purchase line carries no note to mark, and the seeded one is what
        // the receipt lines point at, so it is named by the id it was given.
        "purchase_lines" => "DELETE FROM purchase_lines WHERE id <> 1".to_string(),
        // The seven the migration seeds stay; a probe row is anything else.
        "expense_categories" => "DELETE FROM expense_categories WHERE key NOT IN \
             ('rent', 'electricity', 'water', 'salaries', 'transport', 'maintenance', 'other')"
            .to_string(),
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

/// The supply side of the same idea: a supplier, a purchase, one line, one
/// receipt and one ledger movement, each marked so `clear` can tell it from a
/// probe row. The product the line points at comes from `seed_for_probes`.
fn seed_for_supply_probes(conn: &mut SqliteConnection) {
    seed_for_probes(conn);
    for seed in [
        insert_with("suppliers", "name", "'seed'"),
        insert_with("purchases", "note", "'seed'"),
        insert_with("purchase_lines", "qty_ordered_milli", "1000"),
        insert_with("purchase_receipts", "note", "'seed'"),
        insert_with("supplier_ledger", "note", "'seed'"),
    ] {
        diesel::sql_query(seed).execute(conn).unwrap();
    }
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
    // A payment is not on this list because it cannot be written the way the
    // others are: migration 6 ties the mode to the kind, so a payment carries
    // one and nothing else may. It is probed below with its mode.
    for value in ["'opening'", "'sale'", "'avoir'", "'adjustment'"] {
        assert!(
            probe(&mut conn, "debt_ledger", "kind", value).is_ok(),
            "debt_ledger.kind refused {value}"
        );
    }
    assert!(
        diesel::sql_query(
            "INSERT INTO debt_ledger (shop_id, customer_id, kind, debit_centimes, \
             credit_centimes, user_id, payment_mode) VALUES (1, 1, 'payment', 0, 0, 1, 'cash')"
        )
        .execute(&mut conn)
        .is_ok(),
        "debt_ledger.kind refused 'payment'"
    );
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
fn every_money_and_quantity_column_of_the_supply_tables_refuses_a_real_a_text_and_a_negative() {
    // Rule 6 on the tables migration 8 adds. A landed cost read back as
    // something other than centimes is a margin computed on a price nobody
    // paid, and a quantity read back as a real is stock that never balances.
    let (_dir, mut conn) = open_temp();
    seed_for_supply_probes(&mut conn);
    let zero_is_fine = [
        ("purchases", "transport_centimes"),
        ("purchases", "extra_costs_centimes"),
        ("purchase_lines", "unit_cost_centimes"),
        ("purchase_lines", "landed_unit_cost_centimes"),
        ("purchase_lines", "qty_received_milli"),
        ("purchase_lines", "qty_returned_milli"),
        ("supplier_ledger", "debit_centimes"),
        ("supplier_ledger", "credit_centimes"),
        ("expense_categories", "sort_order"),
    ];
    for (table, column) in zero_is_fine {
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
    // A purchase of nothing, an allocation of nothing, a received line of
    // nothing and an expense of nothing each say a movement happened where
    // none did, so zero is refused where the columns above take it.
    let above_zero = [
        ("purchase_lines", "qty_ordered_milli"),
        ("purchase_receipt_lines", "qty_milli"),
        ("supplier_allocations", "amount_centimes"),
        ("expenses", "amount_centimes"),
    ];
    for (table, column) in above_zero {
        assert!(
            probe(&mut conn, table, column, "1000").is_ok(),
            "{table}.{column}: an integer was refused"
        );
        clear(&mut conn, table);
        for bad in ["0", "-1", "19.99", "'abc'"] {
            assert!(
                probe(&mut conn, table, column, bad).is_err(),
                "{table}.{column} accepted {bad}"
            );
        }
    }
    // A receipt takes its number from a counter the way a document does, and
    // the first number of a series is 1. The seeded receipt already holds
    // number 1 in this series, so the valid probe takes the next one.
    assert!(probe(&mut conn, "purchase_receipts", "number", "2").is_ok());
    clear(&mut conn, "purchase_receipts");
    for bad in ["0", "-1", "1.5", "'abc'"] {
        assert!(
            probe(&mut conn, "purchase_receipts", "number", bad).is_err(),
            "purchase_receipts.number accepted {bad}"
        );
    }
}

#[test]
fn a_purchase_line_refuses_a_received_quantity_above_the_ordered_one() {
    // The one CHECK that reads two columns of a line at once. A line that
    // says more arrived than was ordered is a purchase whose stock and whose
    // supplier debt disagree with the paper it came from. How much a single
    // receipt may add is a rule the service holds; this is the state the file
    // itself refuses to hold.
    let (_dir, mut conn) = open_temp();
    seed_for_supply_probes(&mut conn);
    assert!(
        diesel::sql_query(
            "INSERT INTO purchase_lines (shop_id, purchase_id, product_id, qty_ordered_milli, \
             unit_cost_centimes, landed_unit_cost_centimes, qty_received_milli) \
             VALUES (1, 1, 1, 1000, 0, 0, 1000)"
        )
        .execute(&mut conn)
        .is_ok(),
        "a line received in full was refused"
    );
    assert!(
        diesel::sql_query(
            "INSERT INTO purchase_lines (shop_id, purchase_id, product_id, qty_ordered_milli, \
             unit_cost_centimes, landed_unit_cost_centimes, qty_received_milli) \
             VALUES (1, 1, 1, 1000, 0, 0, 1001)"
        )
        .execute(&mut conn)
        .is_err(),
        "a line took more than was ordered"
    );
}

#[test]
fn a_supplier_ledger_row_carries_one_direction_and_a_mode_only_on_a_payment() {
    // The mirror of `a_ledger_row_carries_one_direction_…` on the supply
    // side. Debit is what the shop owes the supplier more of (an opening
    // balance, goods received) and credit is what takes it off (a payment, a
    // return), and the two never share a row.
    let (_dir, mut conn) = open_temp();
    seed_for_supply_probes(&mut conn);
    assert!(
        diesel::sql_query(
            "INSERT INTO supplier_ledger (shop_id, supplier_id, kind, debit_centimes, \
             credit_centimes, user_id) VALUES (1, 1, 'purchase', 1000, 1000, 1)"
        )
        .execute(&mut conn)
        .is_err(),
        "a supplier ledger row carried a debit and a credit at once"
    );
    for one_way in [("1000", "0"), ("0", "1000")] {
        diesel::sql_query(format!(
            "INSERT INTO supplier_ledger (shop_id, supplier_id, kind, debit_centimes, \
             credit_centimes, user_id) VALUES (1, 1, 'purchase', {}, {}, 1)",
            one_way.0, one_way.1
        ))
        .execute(&mut conn)
        .unwrap();
    }
    // The five kinds, and the payment written out with its mode because the
    // table ties the two together.
    for value in ["'opening'", "'purchase'", "'return'", "'adjustment'"] {
        assert!(
            probe(&mut conn, "supplier_ledger", "kind", value).is_ok(),
            "supplier_ledger.kind refused {value}"
        );
    }
    for (kind, mode) in [
        ("opening", "'cash'"),
        ("purchase", "'cash'"),
        ("return", "'cash'"),
        ("adjustment", "'cash'"),
        ("payment", "NULL"),
    ] {
        assert!(
            diesel::sql_query(format!(
                "INSERT INTO supplier_ledger (shop_id, supplier_id, kind, debit_centimes, \
                 credit_centimes, user_id, payment_mode) \
                 VALUES (1, 1, '{kind}', 1000, 0, 1, {mode})"
            ))
            .execute(&mut conn)
            .is_err(),
            "a {kind} with a mode of {mode} was taken"
        );
    }
    for mode in ["'cash'", "'card'"] {
        assert!(
            diesel::sql_query(format!(
                "INSERT INTO supplier_ledger (shop_id, supplier_id, kind, debit_centimes, \
                 credit_centimes, user_id, payment_mode) \
                 VALUES (1, 1, 'payment', 0, 1000, 1, {mode})"
            ))
            .execute(&mut conn)
            .is_ok(),
            "a payment made {mode} was refused"
        );
    }
    assert!(
        diesel::sql_query(
            "INSERT INTO supplier_ledger (shop_id, supplier_id, kind, debit_centimes, \
             credit_centimes, user_id, payment_mode) \
             VALUES (1, 1, 'payment', 0, 1000, 1, 'credit')"
        )
        .execute(&mut conn)
        .is_err(),
        "the supplier ledger took a mode that is not a way of paying"
    );
    assert!(probe(&mut conn, "supplier_ledger", "kind", "'sale'").is_err());
}

#[test]
fn a_purchase_only_takes_the_states_the_plan_names() {
    // A purchase is ordered, then received in part or in full, or it is
    // cancelled before anything arrived, or closed short when the rest never
    // will. A sixth word would be a state no screen and no query knows.
    let (_dir, mut conn) = open_temp();
    seed_for_supply_probes(&mut conn);
    for value in [
        "'ordered'",
        "'partially_received'",
        "'received'",
        "'cancelled'",
        "'closed_short'",
    ] {
        assert!(
            probe(&mut conn, "purchases", "status", value).is_ok(),
            "purchases.status refused {value}"
        );
        clear(&mut conn, "purchases");
    }
    for value in ["'issued'", "'open'", "''"] {
        assert!(
            probe(&mut conn, "purchases", "status", value).is_err(),
            "purchases.status accepted {value}"
        );
    }
}

#[test]
fn a_supplier_name_a_document_number_and_a_category_key_are_each_taken_once() {
    // features.md §1 names the supplier unique. The uniqueness is per shop
    // like every other one here, and it covers a deactivated fiche too: a
    // closed supplier keeps its ledger, and a second fiche under the same
    // name would read at the counter as one party with two balances.
    let (_dir, mut conn) = open_temp();
    seed_for_supply_probes(&mut conn);
    assert!(
        probe(&mut conn, "suppliers", "name", "'seed'").is_err(),
        "a supplier name was taken twice in one shop"
    );
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(&mut conn)
        .unwrap();
    assert!(
        diesel::sql_query("INSERT INTO suppliers (shop_id, name) VALUES (2, 'seed')")
            .execute(&mut conn)
            .is_ok(),
        "another shop could not use the same supplier name"
    );
    // The supplier's own number on the paper they sent: unique under that
    // supplier where it is written down, and absent as often as a shop
    // pleases, because a delivery note without a number is a thing suppliers
    // hand over.
    assert!(probe(
        &mut conn,
        "purchases",
        "supplier_document_number",
        "'BL-77'"
    )
    .is_ok());
    assert!(
        probe(
            &mut conn,
            "purchases",
            "supplier_document_number",
            "'BL-77'"
        )
        .is_err(),
        "one supplier's document number was taken twice"
    );
    assert!(probe(&mut conn, "purchases", "supplier_document_number", "NULL").is_ok());
    assert!(
        probe(&mut conn, "purchases", "supplier_document_number", "NULL").is_ok(),
        "two purchases without a supplier number collided"
    );
    // The seeded keys are the shop's own, one of each.
    assert!(
        probe(&mut conn, "expense_categories", "key", "'rent'").is_err(),
        "an expense category key was seeded twice in one shop"
    );
    assert!(probe(&mut conn, "jobs", "name", "'stock_rederive'").is_ok());
    assert!(
        probe(&mut conn, "jobs", "name", "'stock_rederive'").is_err(),
        "one shop carried the same job twice"
    );
    // A receipt's own number, the way a document's is: taken once inside its
    // series and free in another. The seeded receipt already holds number 1
    // of `reception:2026`.
    assert!(
        probe(&mut conn, "purchase_receipts", "number", "1").is_err(),
        "a receipt number was handed out twice inside one series"
    );
    assert!(probe(&mut conn, "purchase_receipts", "number", "2").is_ok());
    clear(&mut conn, "purchase_receipts");
    assert!(
        diesel::sql_query(insert_with_all(
            "purchase_receipts",
            &[("series", "'reception:2027'"), ("number", "1")],
        ))
        .execute(&mut conn)
        .is_ok(),
        "the same number was refused in another series"
    );
}

#[test]
fn an_active_flag_is_a_zero_or_a_one_and_nothing_else() {
    // The boolean is an INTEGER because a STRICT table has no boolean type,
    // so the CHECK is the only thing keeping a 2, a -1 or the word "true" out
    // of a column every list filters on.
    let (_dir, mut conn) = open_temp();
    seed_for_supply_probes(&mut conn);
    for (table, column) in [("suppliers", "active"), ("expense_categories", "active")] {
        for fine in ["0", "1"] {
            assert!(
                probe(&mut conn, table, column, fine).is_ok(),
                "{table}.{column} refused {fine}"
            );
            clear(&mut conn, table);
        }
        // Not "1.0" and not "'1'": STRICT converts a lossless real and a
        // numeric string into the integer the column declares, which is the
        // behaviour `a_lossless_real_is_stored_as_an_integer_by_strict_itself`
        // pins. What the CHECK is for is a number that is not a flag.
        for bad in ["2", "-1", "'true'", "'abc'"] {
            assert!(
                probe(&mut conn, table, column, bad).is_err(),
                "{table}.{column} accepted {bad}"
            );
        }
    }
}

#[test]
fn a_receipt_line_names_a_line_of_its_own_purchase_and_names_it_once() {
    // The receipt is against one purchase and the line belongs to one
    // purchase, and nothing tied the two together: a delivery on this order
    // could add stock against a line of somebody else's order and the debt
    // would land on the wrong paper. The composite keys are what tie them,
    // and they read the same fact from both ends.
    let (_dir, mut conn) = open_temp();
    seed_for_supply_probes(&mut conn);
    // A second purchase, from the same supplier, with a line of its own.
    diesel::sql_query(insert_with_all(
        "purchases",
        &[
            ("note", "'seconde commande'"),
            ("purchase_date", "'2026-09-11'"),
        ],
    ))
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(insert_with("purchase_lines", "purchase_id", "2"))
        .execute(&mut conn)
        .unwrap();
    // Receipt 1 is against purchase 1, and line 2 is a line of purchase 2.
    assert!(
        diesel::sql_query(insert_with_all(
            "purchase_receipt_lines",
            &[("purchase_line_id", "2")],
        ))
        .execute(&mut conn)
        .is_err(),
        "a delivery on one purchase took a line of another"
    );
    // And the purchase the receipt line carries has to be the receipt's own,
    // which is the same fact read from the other end.
    assert!(
        diesel::sql_query(insert_with_all(
            "purchase_receipt_lines",
            &[("purchase_id", "2"), ("purchase_line_id", "2")],
        ))
        .execute(&mut conn)
        .is_err(),
        "a receipt line named a purchase its receipt does not belong to"
    );
    // The line of its own purchase is taken, once.
    assert!(
        diesel::sql_query(insert_with("purchase_receipt_lines", "qty_milli", "400"))
            .execute(&mut conn)
            .is_ok(),
        "a delivery could not name a line of its own purchase"
    );
    assert!(
        diesel::sql_query(insert_with("purchase_receipt_lines", "qty_milli", "300"))
            .execute(&mut conn)
            .is_err(),
        "one line was written twice on one receipt"
    );
    // A second receipt against the same purchase may name it again: two
    // deliveries against one line is what a partial receipt is.
    diesel::sql_query(insert_with_all(
        "purchase_receipts",
        &[("number", "2"), ("note", "'seconde livraison'")],
    ))
    .execute(&mut conn)
    .unwrap();
    assert!(
        diesel::sql_query(insert_with_all(
            "purchase_receipt_lines",
            &[("receipt_id", "2"), ("qty_milli", "300")],
        ))
        .execute(&mut conn)
        .is_ok(),
        "a second delivery could not name the line the first one did"
    );
}

#[test]
fn a_purchase_line_refuses_a_returned_quantity_above_what_arrived() {
    // features.md §1 lists `return` among the stock movements, and what goes
    // back to a supplier is what came from them: a line returning more than
    // it received is stock the shop never had and a credit it is not owed.
    let (_dir, mut conn) = open_temp();
    seed_for_supply_probes(&mut conn);
    assert!(
        diesel::sql_query(insert_with_all(
            "purchase_lines",
            &[("qty_received_milli", "600"), ("qty_returned_milli", "600")],
        ))
        .execute(&mut conn)
        .is_ok(),
        "a line returning everything it received was refused"
    );
    assert!(
        diesel::sql_query(insert_with_all(
            "purchase_lines",
            &[("qty_received_milli", "600"), ("qty_returned_milli", "601")],
        ))
        .execute(&mut conn)
        .is_err(),
        "a line returned more than arrived"
    );
    // Nothing received is nothing to send back.
    assert!(
        diesel::sql_query(insert_with_all(
            "purchase_lines",
            &[("qty_received_milli", "0"), ("qty_returned_milli", "1")],
        ))
        .execute(&mut conn)
        .is_err(),
        "a line sent back goods it never took in"
    );
}

#[test]
fn the_supply_tables_keep_what_they_name_and_lose_only_what_they_may() {
    // The foreign key actions migration 8 chose, each read off the file
    // rather than off the schema: a supplier with an order or a movement
    // behind them is never deleted, nor is a product on a line or a category
    // an expense is filed under; a purchase takes its own lines with it, and
    // a purchase a movement merely cites goes away leaving what is owed
    // behind, because what the shop owes is not a fact about the order that
    // caused it.
    let (_dir, mut conn) = open_temp();
    seed_for_supply_probes(&mut conn);
    diesel::sql_query(
        "INSERT INTO supplier_ledger (id, shop_id, supplier_id, purchase_id, kind,          debit_centimes, credit_centimes, user_id, note)          VALUES (2, 1, 1, 1, 'purchase', 100000, 0, 1, 'seed')",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(insert_with("expenses", "amount_centimes", "300000"))
        .execute(&mut conn)
        .unwrap();

    for (what, sql) in [
        (
            "a supplier with an order behind them",
            "DELETE FROM suppliers WHERE id = 1",
        ),
        (
            "a product on a purchase line",
            "DELETE FROM products WHERE id = 1",
        ),
        (
            "a category an expense is filed under",
            "DELETE FROM expense_categories WHERE id = 1",
        ),
    ] {
        assert!(
            diesel::sql_query(sql).execute(&mut conn).is_err(),
            "{what} was deleted"
        );
    }

    // The receipt holds the order down, so it goes first; then the order
    // takes its own lines with it and leaves the ledger row standing without
    // the purchase it named.
    diesel::sql_query("DELETE FROM purchase_receipt_lines")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query("DELETE FROM purchase_receipts WHERE id = 1")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query("DELETE FROM purchases WHERE id = 1")
        .execute(&mut conn)
        .unwrap();
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM purchase_lines"),
        0,
        "the lines did not go with the order they belong to"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM supplier_ledger WHERE id = 2 AND purchase_id IS NULL              AND debit_centimes = 100000"
        ),
        1,
        "the movement went with the order instead of losing its id"
    );
    assert_eq!(orphan_rows(&mut conn), 0);

    // A payment and the purchase it settled, with the allocation that ties
    // them together: neither end goes.
    diesel::sql_query(insert_with("purchases", "note", "'a payer'"))
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO supplier_ledger (id, shop_id, supplier_id, kind, debit_centimes,          credit_centimes, user_id, payment_mode)          VALUES (3, 1, 1, 'payment', 0, 50000, 1, 'cash')",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO supplier_allocations (shop_id, payment_ledger_id, purchase_id,          amount_centimes) VALUES (1, 3, 2, 50000)",
    )
    .execute(&mut conn)
    .unwrap();
    assert!(
        diesel::sql_query("DELETE FROM supplier_ledger WHERE id = 3")
            .execute(&mut conn)
            .is_err(),
        "a payment an allocation settles with was deleted"
    );
    assert!(
        diesel::sql_query("DELETE FROM purchases WHERE id = 2")
            .execute(&mut conn)
            .is_err(),
        "a purchase an allocation settled was deleted"
    );
    assert_eq!(orphan_rows(&mut conn), 0);
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
    // Since the kind rules (migration 7) some of these values do not stand on
    // their own: an avoir names the facture it is written against, an annulée
    // document says when and by whom and why, and a document nobody paid cash
    // for holds neither an amount tendered nor change. The value under test is
    // still the one being probed; what travels with it is what the file now
    // insists on, and the row is otherwise the template's.
    let companions = |column: &str, value: &str| -> Vec<(&'static str, &'static str)> {
        match (column, value) {
            ("kind", "'avoir'") => vec![("ref_document_id", "1")],
            ("status", "'cancelled'") => vec![
                ("cancelled_at", "'2026-09-10 09:15:00'"),
                ("cancelled_by", "1"),
                ("cancel_reason", "'erreur de saisie'"),
            ],
            ("payment_mode", "'cash'") => Vec::new(),
            ("payment_mode", _) => vec![("tendered_centimes", "NULL"), ("change_centimes", "NULL")],
            _ => Vec::new(),
        }
    };
    for (column, good, bad) in sets {
        for value in good {
            let mut row = vec![(column, value)];
            row.extend(companions(column, value));
            assert!(
                diesel::sql_query(insert_with_all("documents", &row))
                    .execute(&mut conn)
                    .is_ok(),
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
    conn.run_pending_migrations(dzpos_retail::db::MIGRATIONS)
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
    use dzpos_retail::models::product::{NewProduct, Unit};
    use dzpos_retail::money::{Bps, Money, PaymentMode};
    use dzpos_retail::services::sales::{NewSale, NewSaleLine, SaleKind};
    use dzpos_retail::services::{products, sales};
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
    conn.run_pending_migrations(dzpos_retail::db::MIGRATIONS)
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

    conn.run_pending_migrations(dzpos_retail::db::MIGRATIONS)
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
            // The series carries the year of issue once the ninth migration
            // has run (features.md §4, Numbering); the number the paper in
            // the customer's hand carries is untouched by that.
            "SELECT COUNT(*) AS n FROM documents WHERE id = 3 AND number = 12 \
             AND series = 'doc_ticket:2026' AND series_year = 2026 \
             AND net_to_pay_centimes = 1190 \
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

    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
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

    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
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

/// Three tickets and the counter that handed their numbers out, written the
/// way the app wrote them before a series carried a year. `issued_at` is on
/// the shop's calendar (`services::clock::now`), so the comment beside each
/// row is the UTC instant the till was at when it wrote it.
fn seed_three_tickets_across_the_new_year(conn: &mut SqliteConnection) {
    for (id, number, issued_at) in [
        // 2025-12-31T22:30:00Z, still 31 December in Algiers.
        (30, 1, "2025-12-31 23:30:00"),
        // 2025-12-31T23:30:00Z, already 1 January in Algiers.
        (31, 2, "2026-01-01 00:30:00"),
        // 2026-01-01T00:30:00Z.
        (32, 3, "2026-01-01 01:30:00"),
    ] {
        diesel::sql_query(format!(
            "INSERT INTO documents (id, shop_id, kind, series, number, issued_at, user_id, \
             regime, payment_mode, seller_name, total_ht_centimes, discount_centimes, \
             subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes, \
             net_to_pay_centimes) \
             VALUES ({id}, 1, 'ticket', 'doc_ticket', {number}, '{issued_at}', 1, 'reel', \
             'cash', 'Mon magasin', 10000, 0, 10000, 1900, 11900, 0, 11900)"
        ))
        .execute(conn)
        .unwrap();
    }
    // The counter as the till left it: the next ticket would have been 4.
    diesel::sql_query(
        "INSERT INTO counters (shop_id, name, next_value) VALUES (1, 'doc_ticket', 4)",
    )
    .execute(conn)
    .unwrap();
    // And a series the shop has taken a number in without keeping a document,
    // which is what a proforma the operator deleted the file of looks like.
    // There is no year to name it after.
    diesel::sql_query(
        "INSERT INTO counters (shop_id, name, next_value) VALUES (1, 'doc_proforma', 2)",
    )
    .execute(conn)
    .unwrap();
}

#[test]
fn a_database_without_the_series_year_takes_the_migration_that_adds_it() {
    // architecture.md, Data: a migration ships with a test that opens a
    // database built by the previous ones and applies it. This one adds the
    // year a series counts in (features.md §4, Numbering), so what has to be
    // proved is that the year comes off the shop's calendar and not the
    // machine's, and that the counter carries on rather than starting again
    // beside a number already printed.
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_before_migration("series_year");
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('documents') \
             WHERE name = 'series_year'"
        ),
        0,
        "the file this starts from already carries series_year"
    );
    seed_three_tickets_across_the_new_year(&mut conn);

    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    conn.run_migration(&pending[0]).unwrap();

    // The hour between 23:00 UTC and midnight is the whole of the question. A
    // backfill off `created_at`, which is the column default's UTC, would put
    // ticket 31 in 2025 and hand its number out again in the new year.
    for (id, year) in [(30, 2025), (31, 2026), (32, 2026)] {
        assert_eq!(
            count(
                &mut conn,
                &format!(
                    "SELECT COUNT(*) AS n FROM documents WHERE id = {id} \
                     AND series_year = {year} AND series = 'doc_ticket:{year}'"
                )
            ),
            1,
            "ticket {id} did not land in {year}"
        );
    }
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE number IN (1, 2, 3) \
             AND net_to_pay_centimes = 11900"
        ),
        3,
        "the tickets the file already carried did not survive the new column"
    );
    // The counter carries on under the key the code now asks for, at the
    // number it had reached. A counter starting again at 1 would print
    // TK-2026-000002 twice.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM counters WHERE shop_id = 1 \
             AND name = 'doc_ticket:2026' AND next_value = 4"
        ),
        1,
        "the ticket counter did not carry on into the year it had reached"
    );
    // A series no document has used names no year, and the barcode series is
    // not a document series at all (features.md §1): neither is renamed.
    for name in ["doc_proforma", "in_store_barcode"] {
        assert_eq!(
            count(
                &mut conn,
                &format!("SELECT COUNT(*) AS n FROM counters WHERE name = '{name}'")
            ),
            1,
            "{name} was renamed and it names no year"
        );
    }
    assert_eq!(orphan_rows(&mut conn), 0);
}

/// The audit log is the one table whose moments came from the column's own
/// default, which is SQLite's UTC, while every other date in the file is on
/// the shop's calendar. A shop that has been running keeps rows written that
/// way, so the migration has to move what is already there and not only
/// change what is written next. The row at 23:30 is the one that mattered on
/// a screen: it was written at half past midnight in Algiers and printed on
/// the day before.
#[test]
fn the_audit_log_moves_onto_the_shop_clock_with_the_rows_already_in_it() {
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_before_migration("audit_log_shop_clock");
    diesel::sql_query(
        "INSERT INTO audit_log (id, shop_id, user_id, action, entity, created_at) VALUES \
         (1, 1, 1, 'product.update', 'product', '2026-09-11 23:30:00'), \
         (2, 1, 1, 'price.update', 'product', '2026-09-11 10:00:00')",
    )
    .execute(&mut conn)
    .unwrap();

    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    conn.run_migration(&pending[0]).unwrap();

    for (id, moment) in [(1, "2026-09-12 00:30:00"), (2, "2026-09-11 11:00:00")] {
        assert_eq!(
            count(
                &mut conn,
                &format!(
                    "SELECT COUNT(*) AS n FROM audit_log WHERE id = {id} \
                     AND created_at = '{moment}'"
                )
            ),
            1,
            "row {id} is not on the shop's clock"
        );
    }
    assert_eq!(orphan_rows(&mut conn), 0);
}

#[test]
fn the_migration_reverts_and_reapplies() {
    // architecture.md, Data: a migration ships with a test that runs it.
    // Reverting all the way back to an empty file is what proves each
    // down.sql undoes its own up.sql and nothing else.
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_temp();
    // An empty file proves the tables move, not the rows: the down.sql copies
    // documents back, and a facture with a buyer and a debt is what it carries.
    seed_a_facture_naming_a_customer(&mut conn);

    // The seeder writes the row the way the file below the ninth migration
    // spells it, because the cancellation test uses it on a file that has no
    // `series_year` column at all. Numbered here, so the revert below has a
    // year to take back out; without this the down's `substr` would be
    // asserted against a string it never touches.
    assert_eq!(
        diesel::sql_query(
            "UPDATE documents SET series = 'doc_facture:2026', series_year = 2026 WHERE id = 4"
        )
        .execute(&mut conn)
        .unwrap(),
        1
    );

    // Eight migrations (21 down to 14) sit on the audit clock (13), which moves
    // data and adds no table (the seventeenth's expense half is in
    // `migration_shifts.rs`): nine turns, both directions read off one row.
    assert_eq!(
        diesel::sql_query(
            "INSERT INTO audit_log (shop_id, user_id, action, entity, created_at) \
             VALUES (1, 1, 'test.shift', 'test', '2026-09-11 23:30:00')"
        )
        .execute(&mut conn)
        .unwrap(),
        1
    );
    for _ in 0..9 {
        conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
            .unwrap();
    }
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM audit_log \
             WHERE action = 'test.shift' AND created_at = '2026-09-11 22:30:00'"
        ),
        1,
        "the audit clock down.sql did not put the hour back"
    );
    conn.run_pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM audit_log \
             WHERE action = 'test.shift' AND created_at = '2026-09-11 23:30:00'"
        ),
        1,
        "the audit clock up.sql did not take the hour back"
    );
    // Those eight (21 down to 14) and the audit clock (13) itself: all nine.
    for _ in 0..9 {
        conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
            .unwrap();
    }

    // The twelfth: the sessions table. It adds a table and two indexes and
    // nothing else, so its down drops all three and touches no user and no
    // document.
    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master \
             WHERE name IN ('sessions', 'idx_sessions_token_hash', 'idx_sessions_shop_user')"
        ),
        0,
        "the sessions down.sql left the table or one of its indexes behind"
    );

    // The eleventh: the columns signing in needs. It adds five to `users` and
    // an index over them, and its down takes exactly those off and leaves the
    // row, its id and its role standing.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('users') \
             WHERE name IN ('password_hash', 'active', 'pin_failures', \
                            'locked_until', 'updated_at')"
        ),
        5,
        "the migrated file does not carry the columns this asserts are removed"
    );
    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('users') \
             WHERE name IN ('password_hash', 'active', 'pin_failures', \
                            'locked_until', 'updated_at')"
        ),
        0,
        "the users auth down.sql left a column behind"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'index' \
             AND name = 'idx_users_shop_name'"
        ),
        0,
        "the users auth down.sql left its index behind"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM users WHERE id = 1 AND role = 'owner'"
        ),
        1,
        "the users auth down.sql took the seeded owner with it"
    );

    // The tenth: the preferences table. It adds a table and nothing else, so
    // its down drops it and touches no document.
    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master \
             WHERE type = 'table' AND name = 'preferences'"
        ),
        0,
        "the preferences down.sql left the table behind"
    );

    // The ninth: the year a series counts in. Its down puts the series string
    // back the way the file below it spells it and takes the column off,
    // which is what the kind rules underneath it are asserted against.
    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('documents') \
             WHERE name = 'series_year'"
        ),
        0,
        "the series year down.sql left the column behind"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 4 AND number = 7 \
             AND series = 'doc_facture'"
        ),
        1,
        "the series year down.sql left the year in the series string"
    );

    // The suppliers migration adds ten tables and touches none of the ones
    // already there, so its down takes exactly those ten away and leaves the
    // customer side standing. Named by table rather than by ordinal: another
    // branch's migration can land in front of this one and shift every number
    // after it.
    let supply_tables = [
        "suppliers",
        "supplier_ledger",
        "supplier_allocations",
        "purchases",
        "purchase_lines",
        "purchase_receipts",
        "purchase_receipt_lines",
        "expense_categories",
        "expenses",
        "jobs",
    ];
    for table in supply_tables {
        assert_eq!(
            count(
                &mut conn,
                &format!(
                    "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' \
                     AND name = '{table}'"
                )
            ),
            1,
            "the migrated file does not carry {table}, which this asserts is removed"
        );
    }
    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    for table in supply_tables {
        assert_eq!(
            count(
                &mut conn,
                &format!(
                    "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' \
                     AND name = '{table}'"
                )
            ),
            0,
            "the suppliers down.sql left {table} behind"
        );
    }
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM debt_ledger WHERE customer_id = 1 AND kind = 'sale'"
        ),
        1,
        "the suppliers down.sql took the customer side with it"
    );

    // Read before the revert as well as after: an assertion that only ever
    // says "not there" would go on passing if the pattern below stopped
    // matching the CHECK the migration actually writes.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' \
             AND name = 'documents' AND sql LIKE '%kind <> ''avoir''%'"
        ),
        1,
        "the migrated file does not carry the kind rules this asserts are removed"
    );
    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    // The eighth one rebuilt `documents` to hang the kind rules on it, so its
    // down rebuilds it once more without them. Every column stays: they
    // belong to the migrations below and come off with their own downs.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' \
             AND name = 'documents' AND sql LIKE '%kind <> ''avoir''%'"
        ),
        0,
        "the kind rules down.sql left them behind"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 4 AND number = 7 \
             AND series = 'doc_facture' AND net_to_pay_centimes = 119000 \
             AND customer_id = 1 AND buyer_name = 'Entreprise Benali'"
        ),
        1,
        "the kind rules down.sql took the facture or its buyer block with them"
    );

    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' \
             AND name = 'debt_ledger' AND sql LIKE '%kind <> ''payment''%'"
        ),
        1,
        "the migrated file does not carry the check this asserts is removed"
    );
    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    // The seventh one rebuilt the ledger to hang a two-column CHECK on it, so
    // its down rebuilds it once more without that CHECK. The column stays: it
    // belongs to the fifth migration and comes off with the fifth
    // migration's down, below.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' \
             AND name = 'debt_ledger' AND sql LIKE '%kind <> ''payment''%'"
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

    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
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

    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
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

    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
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
    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
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
    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
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
    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'products'"
        ),
        0,
        "down.sql left products behind"
    );
    conn.run_pending_migrations(dzpos_retail::db::MIGRATIONS)
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
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM expense_categories WHERE shop_id = 1"
        ),
        7,
        "the suppliers migration did not reapply with its seeded categories"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' \
             AND name = 'sale_idempotency_keys'"
        ),
        1,
        "sale_idempotency_keys did not reapply"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master \
             WHERE name IN ('shifts', 'idx_shifts_one_open_per_user', \
                            'idx_shifts_shop_opened_at')"
        ),
        3,
        "the shifts table or one of its indexes did not reapply"
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
        // Migration 8 puts the supply side beside them for the same reason:
        // what the shop owes a supplier, what it bought and what it spent are
        // all records a DELETE must not be able to empty.
        "suppliers",
        "supplier_ledger",
        "supplier_allocations",
        "purchases",
        "purchase_receipts",
        "expenses",
        // Migration 17 puts the till drawer beside them: what a person
        // counted at a close is a money record like the rest.
        "shifts",
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
    // stock they came out of gone. The product is archived instead.
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
         credit_centimes, user_id, payment_mode) \
         VALUES (2, 1, 1, 'payment', 0, 50000, 1, 'cash')",
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

    // The migrations sitting on top of the fourth only touch tables the
    // fourth left standing, so they all come off first; the round trip above
    // is where those steps are asserted. The stop is what the fourth's own
    // down does, `customers` going away, rather than a count of reverts: a
    // migration written on another branch lands in between and shifts every
    // ordinal after it.
    loop {
        conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
            .unwrap();
        if count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'customers'",
        ) == 0
        {
            break;
        }
    }
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

    conn.run_pending_migrations(dzpos_retail::db::MIGRATIONS)
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

    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
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

    // What the file now refuses on its own, both ways round: a mode on a
    // movement nobody handed money over for, and a payment that does not say
    // what it was handed over in.
    for (kind, mode) in [
        ("opening", "'cash'"),
        ("sale", "'cash'"),
        ("avoir", "'cash'"),
        ("adjustment", "'cash'"),
        ("payment", "NULL"),
    ] {
        assert!(
            diesel::sql_query(format!(
                "INSERT INTO debt_ledger (shop_id, customer_id, kind, debit_centimes, \
                 credit_centimes, user_id, payment_mode) \
                 VALUES (1, 1, '{kind}', 1000, 0, 1, {mode})"
            ))
            .execute(&mut conn)
            .is_err(),
            "a {kind} with a mode of {mode} was taken"
        );
    }
    // And what it still takes: a payment with the mode `pay` fills in, and
    // every other kind without one.
    for (kind, mode) in [
        ("payment", "'cash'"),
        ("payment", "'card'"),
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

/// A document of `kind` with `overrides` applied to the probe template, as the
/// file answers it. The kind rules read two and three columns at once, so
/// every one of them is asked with a whole row rather than a substitution.
fn kind_row(
    conn: &mut SqliteConnection,
    kind: &str,
    overrides: &[(&str, &str)],
) -> QueryResult<usize> {
    let mut row = vec![("kind", kind)];
    row.extend_from_slice(overrides);
    diesel::sql_query(insert_with_all("documents", &row)).execute(conn)
}

#[test]
fn only_an_avoir_names_the_document_it_is_written_against() {
    // features.md §3: an avoir is written against the facture it credits, and
    // `avoir::issue` is the one writer that fills the column in. Every other
    // paper the till writes stands on its own, so a ticket, a facture or a
    // proforma pointing at another document is a row no service could have
    // written and the table says so.
    let (_dir, mut conn) = open_temp();
    seed_for_probes(&mut conn);

    assert!(
        kind_row(&mut conn, "'avoir'", &[]).is_err(),
        "an avoir crediting nothing was taken"
    );
    for kind in ["'ticket'", "'facture'", "'proforma'"] {
        assert!(
            kind_row(&mut conn, kind, &[("ref_document_id", "1")]).is_err(),
            "a {kind} written against another document was taken"
        );
    }

    assert_eq!(
        kind_row(&mut conn, "'avoir'", &[("ref_document_id", "1")]).unwrap(),
        1,
        "an avoir naming the facture it credits was refused"
    );
    clear(&mut conn, "documents");
    assert_eq!(
        kind_row(&mut conn, "'facture'", &[]).unwrap(),
        1,
        "a facture that names no other document was refused"
    );
}

#[test]
fn tendered_and_change_are_a_cash_document_and_travel_together() {
    // features.md §3: the two columns are what the customer handed over and
    // what went back over the counter, and `sales::settle` fills them in on a
    // cash sale and refuses them anywhere else. A card facture holding change
    // is money the shop never gave back.
    let (_dir, mut conn) = open_temp();
    seed_for_probes(&mut conn);

    for mode in ["'card'", "'credit'", "'cheque'", "'transfer'"] {
        assert!(
            kind_row(&mut conn, "'facture'", &[("payment_mode", mode)]).is_err(),
            "a {mode} document held an amount tendered and change"
        );
    }
    // Half the pair is the other half of the rule: what was handed over and
    // what went back are one fact about one payment, and a row holding one of
    // them is a write that got half way.
    for (tendered, change) in [("5000", "NULL"), ("NULL", "0")] {
        assert!(
            kind_row(
                &mut conn,
                "'ticket'",
                &[("tendered_centimes", tendered), ("change_centimes", change)]
            )
            .is_err(),
            "a cash ticket held {tendered} tendered and {change} change"
        );
    }

    assert_eq!(
        kind_row(&mut conn, "'ticket'", &[]).unwrap(),
        1,
        "a cash ticket with both columns was refused"
    );
    clear(&mut conn, "documents");
    assert_eq!(
        kind_row(
            &mut conn,
            "'facture'",
            &[
                ("payment_mode", "'credit'"),
                ("tendered_centimes", "NULL"),
                ("change_centimes", "NULL"),
            ]
        )
        .unwrap(),
        1,
        "a credit facture with neither column was refused"
    );
}

#[test]
fn a_document_is_annulee_exactly_when_it_says_when_by_whom_and_why() {
    // features.md §5 and the `cancellation` reader in models::document: a
    // document marked annulée with nobody's name on it is what the log is
    // kept against, and a document carrying a cancellation while it still
    // says it stands is the same write from the other side. The reader
    // refuses both; since migration 7 so does the table.
    let (_dir, mut conn) = open_temp();
    seed_for_probes(&mut conn);
    let block = [
        ("cancelled_at", "'2026-09-10 09:15:00'"),
        ("cancelled_by", "1"),
        ("cancel_reason", "'erreur de saisie'"),
    ];

    assert!(
        kind_row(&mut conn, "'facture'", &[("status", "'cancelled'")]).is_err(),
        "an annulée facture said nothing about when or by whom"
    );
    assert!(
        kind_row(&mut conn, "'facture'", &block).is_err(),
        "a facture carrying a cancellation still said it stands"
    );
    for missing in ["cancelled_at", "cancelled_by", "cancel_reason"] {
        let mut row: Vec<(&str, &str)> = vec![("status", "'cancelled'")];
        row.extend(block.iter().filter(|(c, _)| *c != missing).copied());
        row.push((missing, "NULL"));
        assert!(
            kind_row(&mut conn, "'facture'", &row).is_err(),
            "a cancellation was written without its {missing}"
        );
    }
    // The avoir a cancellation issued exists only where the cancellation
    // does: `cancellation::cancel` writes the four together and nothing else
    // writes the column at all.
    assert!(
        kind_row(&mut conn, "'facture'", &[("cancel_avoir_document_id", "1")]).is_err(),
        "a document that still stands named the avoir that annulled it"
    );

    let mut whole: Vec<(&str, &str)> = vec![("status", "'cancelled'")];
    whole.extend(block);
    assert_eq!(
        kind_row(&mut conn, "'facture'", &whole).unwrap(),
        1,
        "a whole cancellation was refused"
    );
    clear(&mut conn, "documents");
    whole.push(("cancel_avoir_document_id", "1"));
    assert_eq!(
        kind_row(&mut conn, "'facture'", &whole).unwrap(),
        1,
        "a cancellation naming the avoir it issued was refused"
    );
}

#[test]
fn a_proforma_says_nothing_is_owed() {
    // features.md §3: a quotation moves no goods and no money. `proforma.rs`
    // writes the triple as three zeros rather than leaving it out, so the
    // paper says what a proforma changes, which is nothing; what it must
    // never say is that this quotation put something on an account.
    let (_dir, mut conn) = open_temp();
    seed_for_probes(&mut conn);

    for column in [
        "old_balance_centimes",
        "remaining_debt_centimes",
        "total_debt_centimes",
    ] {
        assert!(
            kind_row(&mut conn, "'proforma'", &[(column, "1000")]).is_err(),
            "a proforma carried {column} of its own"
        );
    }

    assert_eq!(
        kind_row(
            &mut conn,
            "'proforma'",
            &[
                ("old_balance_centimes", "0"),
                ("remaining_debt_centimes", "0"),
                ("total_debt_centimes", "0"),
            ]
        )
        .unwrap(),
        1,
        "the triple of three zeros a proforma is written with was refused"
    );
    clear(&mut conn, "documents");
    // And a facture still carries what the customer owed, owes and will owe.
    assert_eq!(
        kind_row(
            &mut conn,
            "'facture'",
            &[
                ("old_balance_centimes", "1000"),
                ("remaining_debt_centimes", "11900"),
                ("total_debt_centimes", "12900"),
            ]
        )
        .unwrap(),
        1,
        "a facture was refused its balance triple"
    );
}

#[test]
fn a_database_whose_documents_take_any_block_on_any_kind_takes_the_kind_rules() {
    // architecture.md, Data: a migration ships with a test that opens a
    // database built by the previous ones and applies it. This one rebuilds
    // the table holding every paper the shop has issued, so what has to be
    // proved is that each of them is still there with the id it had, that the
    // rows pointing at those ids still point at them, and that the row the
    // services already refuse is now refused by the file.
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_before_migration("document_kind_rules");
    diesel::sql_query(
        "INSERT INTO customers (id, shop_id, name, party_kind) \
         VALUES (1, 1, 'Entreprise Benali', 'company')",
    )
    .execute(&mut conn)
    .unwrap();
    let paid = |id: i32, kind: &str, number: i64, mode: &str, columns: &str, values: &str| {
        format!(
            "INSERT INTO documents (id, shop_id, kind, series, number, issued_at, user_id, \
             regime, payment_mode, seller_name, total_ht_centimes, discount_centimes, \
             subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes, \
             net_to_pay_centimes{columns}) VALUES ({id}, 1, '{kind}', 'doc_{kind}', {number}, \
             '2026-09-09 10:00:00', 1, 'reel', '{mode}', 'Mon magasin', 100000, 0, 100000, \
             19000, 119000, 0, 119000{values})"
        )
    };
    let document = |id: i32, kind: &str, number: i64, columns: &str, values: &str| -> String {
        paid(id, kind, number, "cash", columns, values)
    };
    // A cash ticket with what was handed over, a credit facture on a
    // customer's account, an annulée facture with the block a cancellation
    // writes, the quotation with its three zeros, and the avoir that credited
    // the facture, which copies the facture's cash mode and still holds
    // neither an amount tendered nor change.
    for sql in [
        document(
            1,
            "ticket",
            1,
            ", tendered_centimes, change_centimes",
            ", 120000, 1000",
        ),
        // Every column of both party blocks carries a value of its own, and
        // no two are the same: the rebuild copies forty-three columns by
        // name, and two of them transposed would put the buyer's RC in the
        // seller's box on every reprint without any count changing.
        paid(
            2,
            "facture",
            1,
            "credit",
            ", customer_id, old_balance_centimes, remaining_debt_centimes, total_debt_centimes, \
             seller_rc, seller_nif, seller_nis, seller_ai, seller_address, seller_phone, \
             buyer_name, buyer_party_kind, buyer_rc, buyer_nif, buyer_nis, buyer_ai, \
             buyer_address",
            ", 1, 0, 119000, 119000, \
             '16/00-1111111 B 21', '000216001111111', '000216002222222', '00021600333333', \
             'Rue Didouche Mourad, Alger', '021 11 22 33', \
             'Entreprise Benali', 'company', '16/00-4444444 B 22', '000216004444444', \
             '000216005555555', '00021600666666', 'Cité 1200 Logements, Oran'",
        ),
        document(
            3,
            "facture",
            2,
            ", status, cancelled_at, cancelled_by, cancel_reason",
            ", 'cancelled', '2026-09-10 09:15:00', 1, 'erreur de saisie'",
        ),
        document(
            4,
            "proforma",
            1,
            ", customer_id, old_balance_centimes, remaining_debt_centimes, total_debt_centimes",
            ", 1, 0, 0, 0",
        ),
        document(5, "avoir", 1, ", ref_document_id", ", 2"),
    ] {
        diesel::sql_query(sql).execute(&mut conn).unwrap();
    }
    diesel::sql_query(
        "INSERT INTO debt_ledger (id, shop_id, customer_id, document_id, kind, debit_centimes, \
         credit_centimes, user_id) VALUES (1, 1, 1, 2, 'sale', 119000, 0, 1)",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO debt_ledger (id, shop_id, customer_id, document_id, kind, debit_centimes, \
         credit_centimes, user_id, payment_mode) \
         VALUES (2, 1, 1, 2, 'payment', 0, 19000, 1, 'card')",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO debt_allocations (shop_id, payment_ledger_id, document_id, amount_centimes) \
         VALUES (1, 2, 2, 19000)",
    )
    .execute(&mut conn)
    .unwrap();
    // The file this starts from takes the row the kind rules refuse, which is
    // what makes the rebuild worth running: a facture marked annulée that
    // says nothing about when or by whom.
    assert_eq!(
        diesel::sql_query(document(6, "facture", 3, ", status", ", 'cancelled'"))
            .execute(&mut conn)
            .unwrap(),
        1,
        "the file this starts from already refuses the row"
    );
    diesel::sql_query("DELETE FROM documents WHERE id = 6")
        .execute(&mut conn)
        .unwrap();

    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    conn.run_migration(&pending[0]).unwrap();

    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 1 AND kind = 'ticket' \
             AND tendered_centimes = 120000 AND change_centimes = 1000"
        ),
        1,
        "the cash ticket did not survive the rebuild with what was handed over"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 2 AND customer_id = 1 \
             AND payment_mode = 'credit' AND remaining_debt_centimes = 119000"
        ),
        1,
        "the credit facture lost its customer or its triple"
    );
    // Column by column, so a pair swapped in the copy goes red here and not
    // on a reprint a year later.
    for (column, value) in [
        ("seller_name", "Mon magasin"),
        ("seller_rc", "16/00-1111111 B 21"),
        ("seller_nif", "000216001111111"),
        ("seller_nis", "000216002222222"),
        ("seller_ai", "00021600333333"),
        ("seller_address", "Rue Didouche Mourad, Alger"),
        ("seller_phone", "021 11 22 33"),
        ("buyer_name", "Entreprise Benali"),
        ("buyer_party_kind", "company"),
        ("buyer_rc", "16/00-4444444 B 22"),
        ("buyer_nif", "000216004444444"),
        ("buyer_nis", "000216005555555"),
        ("buyer_ai", "00021600666666"),
        ("buyer_address", "Cité 1200 Logements, Oran"),
    ] {
        assert_eq!(
            count(
                &mut conn,
                &format!(
                    "SELECT COUNT(*) AS n FROM documents WHERE id = 2 AND {column} = '{value}'"
                )
            ),
            1,
            "the rebuild did not bring {column} across as it was"
        );
    }
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 3 AND status = 'cancelled' \
             AND cancelled_by = 1 AND cancel_reason = 'erreur de saisie'"
        ),
        1,
        "the annulée facture lost the block the cancellation wrote"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 4 AND kind = 'proforma' \
             AND old_balance_centimes = 0 AND total_debt_centimes = 0"
        ),
        1,
        "the quotation lost the triple of three zeros it is written with"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 5 AND kind = 'avoir' \
             AND ref_document_id = 2"
        ),
        1,
        "the avoir lost the facture it was written against"
    );
    // The ids are what the ledger, the allocations and the avoir point at, so
    // a rebuild that reassigned them would say a facture had been settled by
    // somebody else's payment or credited by somebody else's avoir.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM debt_allocations WHERE document_id = 2"
        ),
        1,
        "the allocation lost the facture it names"
    );
    assert_eq!(orphan_rows(&mut conn), 0);
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'index' \
             AND name = 'idx_documents_shop_issued'"
        ),
        1,
        "the rebuilt table lost its index"
    );
    // What the file now refuses on its own.
    assert!(
        diesel::sql_query(document(6, "facture", 3, ", status", ", 'cancelled'"))
            .execute(&mut conn)
            .is_err(),
        "a facture marked annulée with nobody's name on it was taken"
    );
    // And the per-column checks migration 2 wrote are still on the rebuilt
    // table: a copy that quietly relaxed one would be a hole this file opened.
    assert!(
        diesel::sql_query(document(7, "recu", 4, "", ""))
            .execute(&mut conn)
            .is_err(),
        "the rebuilt table took a kind the spec does not name"
    );
    assert!(
        diesel::sql_query(
            document(8, "ticket", 5, "", "").replace("119000, 0, 119000", "119000, 0, -1")
        )
        .execute(&mut conn)
        .is_err(),
        "the rebuilt table took a negative net to pay"
    );
}

#[test]
fn only_a_ticket_and_a_facture_are_annulled() {
    // `cancellation::cancel` (services/cancellation.rs) names the two kinds it
    // knows how to undo and refuses every other by not being on the list, so
    // that a kind added later is refused until somebody decides what undoing
    // it means. An annulée proforma or an annulée avoir is a row no service
    // could have written: a quotation moved nothing to put back, and an avoir
    // is the instrument that undoes a facture rather than something undone in
    // turn.
    let (_dir, mut conn) = open_temp();
    seed_for_probes(&mut conn);
    let block = [
        ("status", "'cancelled'"),
        ("cancelled_at", "'2026-09-10 09:15:00'"),
        ("cancelled_by", "1"),
        ("cancel_reason", "'erreur de saisie'"),
    ];

    for kind in [
        "'proforma'",
        "'avoir'",
        "'quittance'",
        "'bon_de_livraison'",
        "'bon_de_reception'",
    ] {
        let mut row: Vec<(&str, &str)> = block.to_vec();
        if kind == "'avoir'" {
            row.push(("ref_document_id", "1"));
        }
        assert!(
            kind_row(&mut conn, kind, &row).is_err(),
            "an annulée {kind} was taken"
        );
    }
    for kind in ["'ticket'", "'facture'"] {
        assert_eq!(
            kind_row(&mut conn, kind, &block).unwrap(),
            1,
            "an annulée {kind} was refused"
        );
        clear(&mut conn, "documents");
    }
}

#[test]
fn a_database_with_a_shops_m2_history_takes_the_suppliers_and_purchases_tables() {
    // architecture.md, Data: a migration ships with a test that opens a
    // database built by the previous ones and applies it. This one adds
    // tables rather than rebuilding any, so what has to be proved is that the
    // file it lands on keeps every row and every id it had, that the seven
    // expense categories are written once per shop already on the file, and
    // that nothing is left pointing at a parent that is not there.
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_before_migration("suppliers_purchases_expenses");
    // A second shop, so "seeded per existing shop" is read as more than "the
    // shop the first migration made".
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(&mut conn)
        .unwrap();
    // The customer-and-credit rows a shop actually carries: a product, a
    // customer, a facture on credit, the sale movement of the ledger, a
    // payment against it and what that payment settled.
    diesel::sql_query(
        "INSERT INTO products (id, shop_id, name, unit, cost_centimes, selling_centimes, \
         qty_on_hand_milli, low_stock_at_milli, rate_bps) \
         VALUES (1, 1, 'Farine 5kg', 'piece', 20000, 26000, 0, 0, 1900)",
    )
    .execute(&mut conn)
    .unwrap();
    seed_a_facture_naming_a_customer(&mut conn);
    diesel::sql_query(
        "INSERT INTO debt_ledger (id, shop_id, customer_id, kind, debit_centimes, \
         credit_centimes, user_id, payment_mode) \
         VALUES (9, 1, 1, 'payment', 0, 19000, 1, 'cash')",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO debt_allocations (id, shop_id, payment_ledger_id, document_id, \
         amount_centimes) VALUES (3, 1, 9, 4, 19000)",
    )
    .execute(&mut conn)
    .unwrap();

    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    conn.run_migration(&pending[0]).unwrap();

    // Nothing the file already held moved.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE id = 4 AND kind = 'facture' \
             AND series = 'doc_facture' AND number = 7 AND customer_id = 1 \
             AND net_to_pay_centimes = 119000"
        ),
        1,
        "the facture did not survive the migration with its number"
    );
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM customers"),
        1,
        "the customer count changed"
    );
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM debt_ledger"),
        2,
        "the ledger count changed"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM debt_allocations WHERE id = 3 \
             AND payment_ledger_id = 9 AND document_id = 4 AND amount_centimes = 19000"
        ),
        1,
        "the allocation lost the payment or the document it names"
    );

    // Seven categories for each of the two shops, in the order the screen
    // lists them. The label is not stored: the desktop reads it from the i18n
    // file by key, so a shop switching language does not rewrite its rows.
    for shop in [1, 2] {
        assert_eq!(
            count(
                &mut conn,
                &format!("SELECT COUNT(*) AS n FROM expense_categories WHERE shop_id = {shop}")
            ),
            7,
            "shop {shop} was not seeded with the seven categories"
        );
        for (position, key) in [
            "rent",
            "electricity",
            "water",
            "salaries",
            "transport",
            "maintenance",
            "other",
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                count(
                    &mut conn,
                    &format!(
                        "SELECT COUNT(*) AS n FROM expense_categories WHERE shop_id = {shop} \
                         AND key = '{key}' AND sort_order = {} AND active = 1",
                        position + 1
                    )
                ),
                1,
                "shop {shop} is missing the {key} category at its place in the list"
            );
        }
    }

    assert_eq!(
        orphan_rows(&mut conn),
        0,
        "the migrated file has a row pointing at a parent that is not there"
    );
    // The index a supplier's balance and statement read through, the shape
    // the debt ledger's own carries: the id closes it because two movements
    // can land inside one second.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'index' \
             AND name = 'idx_supplier_ledger_shop_supplier'"
        ),
        1,
        "the supplier ledger has no index to read a balance through"
    );
    // And the tables are usable on the file as it stands: a supplier, a
    // purchase against it, a line, a receipt of part of that line and the
    // ledger row the goods put on the account.
    for insert in [
        "INSERT INTO suppliers (id, shop_id, name, phone) VALUES (1, 1, 'Sarl Amrani', '0550')",
        "INSERT INTO purchases (id, shop_id, supplier_id, supplier_document_number, \
         purchase_date, transport_centimes, extra_costs_centimes, status, user_id) \
         VALUES (1, 1, 1, 'BL-77', '2026-09-10', 50000, 0, 'partially_received', 1)",
        "INSERT INTO purchase_lines (id, shop_id, purchase_id, product_id, qty_ordered_milli, \
         unit_cost_centimes, landed_unit_cost_centimes, qty_received_milli) \
         VALUES (1, 1, 1, 1, 10000, 20000, 25000, 4000)",
        "INSERT INTO purchase_receipts (id, shop_id, purchase_id, series, number, received_at, \
         user_id) VALUES (1, 1, 1, 'reception:2026', 1, '2026-09-10 09:30:00', 1)",
        "INSERT INTO purchase_receipt_lines (shop_id, purchase_id, receipt_id, \
         purchase_line_id, qty_milli) VALUES (1, 1, 1, 1, 4000)",
        "INSERT INTO supplier_ledger (id, shop_id, supplier_id, purchase_id, kind, \
         debit_centimes, credit_centimes, user_id) VALUES (1, 1, 1, 1, 'purchase', 100000, 0, 1)",
        "INSERT INTO supplier_ledger (id, shop_id, supplier_id, kind, debit_centimes, \
         credit_centimes, user_id, payment_mode) VALUES (2, 1, 1, 'payment', 0, 60000, 1, 'cash')",
        "INSERT INTO supplier_allocations (shop_id, payment_ledger_id, purchase_id, \
         amount_centimes) VALUES (1, 2, 1, 60000)",
        "INSERT INTO expenses (shop_id, category_id, amount_centimes, expense_date, user_id) \
         VALUES (1, 1, 300000, '2026-09-10', 1)",
        "INSERT INTO jobs (shop_id, name, last_run_day) VALUES (1, 'stock_rederive', NULL)",
    ] {
        diesel::sql_query(insert).execute(&mut conn).unwrap();
    }
    assert_eq!(
        count(
            &mut conn,
            "SELECT COALESCE(SUM(debit_centimes) - SUM(credit_centimes), 0) AS n \
             FROM supplier_ledger WHERE shop_id = 1 AND supplier_id = 1"
        ),
        40000,
        "the supplier ledger does not add up to what is still owed"
    );
    assert_eq!(orphan_rows(&mut conn), 0);
    // A purchase whose receipt names it is not deleted out from under the
    // stock that came in on it.
    assert!(
        diesel::sql_query("DELETE FROM purchases WHERE id = 1")
            .execute(&mut conn)
            .is_err(),
        "a purchase with a receipt was deleted"
    );
}

/// The preferences table lands on a file that already holds a shop, and it
/// takes one row per (shop, key) rather than a series: writing the theme twice
/// leaves one row, which is the whole difference between this table and
/// `settings` beside it.
#[test]
fn a_database_without_preferences_takes_the_migration_that_adds_them() {
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_before_migration("preferences");
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master \
             WHERE type = 'table' AND name = 'preferences'"
        ),
        0,
        "the file this starts from already carries preferences"
    );

    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    conn.run_migration(&pending[0]).unwrap();

    diesel::sql_query(
        "INSERT INTO preferences (shop_id, key, value) VALUES (1, 'theme', 'registre')",
    )
    .execute(&mut conn)
    .unwrap();
    // The upsert the repo does, spelled here so the UNIQUE is what proves it:
    // a second theme for the same shop replaces the first, it does not stack.
    diesel::sql_query(
        "INSERT INTO preferences (shop_id, key, value) VALUES (1, 'theme', 'observe') \
         ON CONFLICT (shop_id, key) DO UPDATE SET value = excluded.value",
    )
    .execute(&mut conn)
    .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM preferences WHERE shop_id = 1 AND key = 'theme'"
        ),
        1,
        "the second theme was appended instead of replacing the first"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM preferences \
             WHERE shop_id = 1 AND key = 'theme' AND value = 'observe'"
        ),
        1,
        "the row does not hold the theme written last"
    );

    // Scoped by shop like every other table (rule 3): a second shop's theme is
    // its own row, not a conflict with the first.
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième')")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO preferences (shop_id, key, value) VALUES (2, 'theme', 'comptoir')",
    )
    .execute(&mut conn)
    .unwrap();
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM preferences"),
        2,
        "two shops did not get one theme each"
    );

    // The shop goes and its preferences go with it.
    diesel::sql_query("DELETE FROM shops WHERE id = 2")
        .execute(&mut conn)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM preferences WHERE shop_id = 2"
        ),
        0,
        "a deleted shop left its preferences behind"
    );
    assert_eq!(orphan_rows(&mut conn), 0);
}

/// The columns signing in needs land on a file that already holds a shop, its
/// owner and everything they have written. Nothing is backfilled: `users` and
/// row 1 of it have existed since the first migration, so what this asserts is
/// that the owner every document already points at reads back as a row with a
/// role, switched on, with no credential set yet.
#[test]
fn a_database_without_the_sign_in_columns_takes_the_migration_that_adds_them() {
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_before_migration("users_auth");
    seed_a_facture_naming_a_customer(&mut conn);
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('users') \
             WHERE name IN ('password_hash', 'active', 'pin_failures', \
                            'locked_until', 'updated_at')"
        ),
        0,
        "the file this starts from already carries the sign-in columns"
    );
    // The paper the owner has already signed, on the file as it stands.
    let documents_before = count(
        &mut conn,
        "SELECT COUNT(*) AS n FROM documents WHERE user_id = 1",
    );
    assert!(
        documents_before > 0,
        "the seeder wrote nothing to carry over"
    );

    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    conn.run_migration(&pending[0]).unwrap();

    // The seeded owner, read back as a row rather than as an id nobody can
    // resolve. `active` defaults on, the counter starts at nothing, and the
    // PIN is still the sentinel the first migration wrote: a row nobody can
    // sign in as, which is the state the next task's login screen inherits.
    let owner: Vec<Name> = diesel::sql_query(
        "SELECT name || '|' || role || '|' || active || '|' || pin_failures \
         || '|' || pin_hash || '|' || (updated_at = created_at) \
         || '|' || (password_hash IS NULL) || '|' || (locked_until IS NULL) AS name \
         FROM users WHERE id = 1",
    )
    .load(&mut conn)
    .unwrap();
    assert_eq!(
        owner.first().map(|r| r.name.as_str()),
        Some("Propriétaire|owner|1|0|!unset|1|1|1"),
        "the seeded owner did not come through the migration as a usable row"
    );
    // And every document it signed still points at it.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM documents WHERE user_id = 1"
        ),
        documents_before,
        "the migration moved the owner out from under the paper it signed"
    );
    assert_eq!(orphan_rows(&mut conn), 0);

    // One name per shop, and only per shop: a second shop's Karim is its own
    // row (rule 3).
    diesel::sql_query("INSERT INTO users (shop_id, name, role) VALUES (1, 'Karim', 'cashier')")
        .execute(&mut conn)
        .unwrap();
    assert!(
        diesel::sql_query("INSERT INTO users (shop_id, name, role) VALUES (1, 'Karim', 'manager')")
            .execute(&mut conn)
            .is_err(),
        "two users of one shop took the same name"
    );
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième')")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query("INSERT INTO users (shop_id, name, role) VALUES (2, 'Karim', 'cashier')")
        .execute(&mut conn)
        .unwrap();

    // The file refuses what the service refuses: a role it does not know
    // (the CHECK is the first migration's and still stands) and an `active`
    // that is neither on nor off.
    assert!(
        diesel::sql_query("INSERT INTO users (shop_id, name, role) VALUES (1, 'Nabil', 'patron')")
            .execute(&mut conn)
            .is_err(),
        "a role nobody defined was accepted"
    );
    assert!(
        diesel::sql_query(
            "INSERT INTO users (shop_id, name, role, active) VALUES (1, 'Nabil', 'cashier', 2)"
        )
        .execute(&mut conn)
        .is_err(),
        "a user was neither active nor inactive"
    );
    assert!(
        diesel::sql_query("UPDATE users SET pin_failures = -1 WHERE id = 1")
            .execute(&mut conn)
            .is_err(),
        "a negative count of wrong PINs was accepted"
    );

    // A deleted shop takes its users with it, the way it takes its
    // preferences: the cascade is the first migration's and the new columns
    // did not change it.
    diesel::sql_query("DELETE FROM users WHERE shop_id = 2")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query("DELETE FROM shops WHERE id = 2")
        .execute(&mut conn)
        .unwrap();
    assert_eq!(orphan_rows(&mut conn), 0);
}

/// The sessions table lands on a file that already holds a shop, its users
/// and the paper they signed. Nothing is backfilled: a session is a live
/// thing and a file that has never had one starts with none.
#[test]
fn a_database_without_sessions_takes_the_migration_that_adds_them() {
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_before_migration("sessions");
    seed_a_facture_naming_a_customer(&mut conn);
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'sessions'"
        ),
        0,
        "the file this starts from already carries the sessions table"
    );

    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    conn.run_migration(&pending[0]).unwrap();

    // Empty, and STRICT like every table since the first one.
    assert_eq!(count(&mut conn, "SELECT COUNT(*) AS n FROM sessions"), 0);
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_list('sessions') WHERE strict = 1"
        ),
        1,
        "the sessions table is not STRICT"
    );
    // Rule 3: the shop is on the row, and so is the user the session acts as.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('sessions') \
             WHERE name IN ('shop_id', 'user_id', 'token_hash', 'created_at', \
                            'last_seen_at', 'ended_at')"
        ),
        6
    );

    diesel::sql_query(
        "INSERT INTO sessions (shop_id, user_id, token_hash, created_at, last_seen_at) \
         VALUES (1, 1, 'aa', '2026-09-11 12:00:00', '2026-09-11 12:00:00')",
    )
    .execute(&mut conn)
    .unwrap();

    // One row per token hash: two under one hash would be a SHA-256 collision
    // or a bug, and the file refuses both.
    assert!(
        diesel::sql_query(
            "INSERT INTO sessions (shop_id, user_id, token_hash, created_at, last_seen_at) \
             VALUES (1, 1, 'aa', '2026-09-11 12:00:00', '2026-09-11 12:00:00')"
        )
        .execute(&mut conn)
        .is_err(),
        "two sessions took the same token hash"
    );
    // A session of a user nobody has, or of a shop nobody has.
    assert!(
        diesel::sql_query(
            "INSERT INTO sessions (shop_id, user_id, token_hash, created_at, last_seen_at) \
             VALUES (1, 999, 'bb', '2026-09-11 12:00:00', '2026-09-11 12:00:00')"
        )
        .execute(&mut conn)
        .is_err(),
        "a session named a user that is not there"
    );
    assert!(
        diesel::sql_query(
            "INSERT INTO sessions (shop_id, user_id, token_hash, created_at, last_seen_at) \
             VALUES (999, 1, 'cc', '2026-09-11 12:00:00', '2026-09-11 12:00:00')"
        )
        .execute(&mut conn)
        .is_err(),
        "a session named a shop that is not there"
    );
    assert_eq!(orphan_rows(&mut conn), 0);

    // A deleted shop takes its sessions with it, the way it takes its
    // preferences; the user it names is what keeps the row from outliving the
    // person, and the FK on users has no cascade because a user is never
    // deleted.
    assert_eq!(on_delete_from_shops(&mut conn, "sessions"), "CASCADE");
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième')")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO users (id, shop_id, name, role) VALUES (7, 2, 'Karim', 'cashier')",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO sessions (shop_id, user_id, token_hash, created_at, last_seen_at) \
         VALUES (2, 7, 'dd', '2026-09-11 12:00:00', '2026-09-11 12:00:00')",
    )
    .execute(&mut conn)
    .unwrap();
    diesel::sql_query("DELETE FROM users WHERE shop_id = 2")
        .execute(&mut conn)
        .unwrap_err();
    diesel::sql_query("DELETE FROM shops WHERE id = 2")
        .execute(&mut conn)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sessions WHERE shop_id = 2"
        ),
        0,
        "a deleted shop left its sessions behind"
    );
    assert_eq!(orphan_rows(&mut conn), 0);
}

#[test]
fn the_canonical_barcode_migration_folds_a_upc_and_leaves_a_collision_alone() {
    // architecture.md, Data: a migration ships with a test that opens a
    // database built by the previous ones and applies it.
    //
    // Three articles sit in the file before it runs: one filed under the
    // twelve digits printed on the box, one already thirteen, and a pair
    // that is the same article under both of its numbers. The first is
    // folded, the second is untouched, and the pair is left exactly as it
    // is: folding it would collide, and which of the two rows is the real
    // article is not something SQL can know.
    use diesel::connection::SimpleConnection;
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_at_migration(15);
    conn.batch_execute(
        "INSERT INTO products (shop_id, name, barcode, unit, cost_centimes, selling_centimes, \
           qty_on_hand_milli, low_stock_at_milli, rate_bps, active) VALUES \
           (1, 'Thé sur la boîte', '613000900127', 'piece', 0, 12000, 0, 0, 1900, 1), \
           (1, 'Café déjà long', '6130009000110', 'piece', 0, 12000, 0, 0, 1900, 1), \
           (1, 'Sucre court', '613000900134', 'piece', 0, 12000, 0, 0, 1900, 1), \
           (1, 'Sucre long', '0613000900134', 'piece', 0, 12000, 0, 0, 1900, 1), \
           (1, 'Sel EAN-8', '61300001', 'piece', 0, 12000, 0, 0, 1900, 1)",
    )
    .unwrap();

    conn.run_pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();

    let mut barcode = |name: &str| -> String {
        #[derive(QueryableByName)]
        struct Row {
            #[diesel(sql_type = Text)]
            barcode: String,
        }
        diesel::sql_query(format!(
            "SELECT barcode FROM products WHERE name = '{name}'"
        ))
        .load::<Row>(&mut conn)
        .unwrap()
        .remove(0)
        .barcode
    };

    assert_eq!(
        barcode("Thé sur la boîte"),
        "0613000900127",
        "the twelve digits on the box are the EAN-13 they are short for"
    );
    assert_eq!(barcode("Café déjà long"), "6130009000110");
    assert_eq!(
        barcode("Sel EAN-8"),
        "61300001",
        "an EAN-8 is its own number"
    );
    assert_eq!(
        barcode("Sucre court"),
        "613000900134",
        "folding this one would collide with the row that already holds it"
    );
    assert_eq!(barcode("Sucre long"), "0613000900134");
}
