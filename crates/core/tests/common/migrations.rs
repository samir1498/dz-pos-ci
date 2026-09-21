//! What a test that reads the file itself needs: a database stopped at a
//! chosen migration, the counts and pragmas SQLite answers about its own
//! shape, and one valid INSERT per table for a probe to spoil one column of.
//!
//! Lifted out of `tests/migration.rs` on 2026-09-21, unchanged, when the
//! shifts migration needed the same helpers from a second file and that one
//! was at the length `scripts/file-sizes.json` pins it to. Nothing here
//! decides anything: a helper that judged a row would hide the very thing
//! the test is about.

use diesel::prelude::*;
use diesel::sql_types::{Integer, Text};

#[derive(QueryableByName)]
pub struct Count {
    #[diesel(sql_type = Integer)]
    pub n: i32,
}

#[derive(QueryableByName)]
pub struct Name {
    #[diesel(sql_type = Text)]
    pub name: String,
}

/// A database carrying the first `n` migrations and nothing after them, so
/// migration `n + 1` can be applied to a file that already holds a shop's
/// data. Every migration ships with a test that opens one of these
/// (architecture.md, Data).
pub fn open_at_migration(n: usize) -> (tempfile::TempDir, SqliteConnection) {
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
pub fn open_before_migration(marker: &str) -> (tempfile::TempDir, SqliteConnection) {
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
pub fn on_delete_from_shops(conn: &mut SqliteConnection, table: &str) -> String {
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
pub fn orphan_rows(conn: &mut SqliteConnection) -> i32 {
    count(conn, "SELECT COUNT(*) AS n FROM pragma_foreign_key_check")
}

pub fn count(conn: &mut SqliteConnection, sql: &str) -> i32 {
    let row: Count = diesel::sql_query(sql).get_result(conn).unwrap();
    row.n
}
/// One INSERT per table with every constrained column at a valid value, so a
/// probe can swap a single column for a bad literal.
pub fn insert_with(table: &str, column: &str, literal: &str) -> String {
    insert_with_all(table, &[(column, literal)])
}

/// The same row with several columns swapped at once. The kind rules of
/// migration 7 read two and three columns together, so a row that means
/// anything to them cannot be built one substitution at a time: an annulée
/// document has to say when, by whom and why in the same INSERT.
pub fn insert_with_all(table: &str, overrides: &[(&str, &str)]) -> String {
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
             remaining_debt_centimes, total_debt_centimes, buyer_party_kind, status, \
             ref_document_id, cancelled_at, cancelled_by, cancel_reason, \
             cancel_avoir_document_id",
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
                "NULL",
                "NULL",
                "NULL",
                "NULL",
                "NULL",
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
        "suppliers" => ("shop_id, name, active", &["1", "'Sarl Amrani'", "1"]),
        // The ledger probes swap one column at a time, so the template row is
        // the one kind that carries no payment mode: a mode on anything else
        // is refused by the table, and the payment is probed with its mode
        // written out in full.
        "supplier_ledger" => (
            "shop_id, supplier_id, purchase_id, kind, debit_centimes, credit_centimes, \
             user_id, note",
            &["1", "1", "NULL", "'purchase'", "0", "0", "1", "NULL"],
        ),
        "supplier_allocations" => (
            "shop_id, payment_ledger_id, purchase_id, amount_centimes",
            &["1", "1", "1", "1"],
        ),
        "purchases" => (
            "shop_id, supplier_id, supplier_document_number, purchase_date, due_date, \
             transport_centimes, extra_costs_centimes, status, user_id, note",
            &[
                "1",
                "1",
                "NULL",
                "'2026-09-10'",
                "NULL",
                "0",
                "0",
                "'ordered'",
                "1",
                "NULL",
            ],
        ),
        "purchase_lines" => (
            "shop_id, purchase_id, product_id, qty_ordered_milli, unit_cost_centimes, \
             landed_unit_cost_centimes, qty_received_milli, qty_returned_milli",
            &["1", "1", "1", "1000", "0", "0", "0", "0"],
        ),
        "purchase_receipts" => (
            "shop_id, purchase_id, series, number, received_at, user_id, note",
            &[
                "1",
                "1",
                "'reception:2026'",
                "1",
                "'2026-09-10 09:30:00'",
                "1",
                "NULL",
            ],
        ),
        "purchase_receipt_lines" => (
            "shop_id, purchase_id, receipt_id, purchase_line_id, qty_milli",
            &["1", "1", "1", "1", "1000"],
        ),
        "expense_categories" => (
            "shop_id, key, sort_order, active",
            &["1", "'probe'", "0", "1"],
        ),
        "expenses" => (
            "shop_id, category_id, amount_centimes, expense_date, note, user_id",
            &["1", "1", "1", "'2026-09-10'", "NULL", "1"],
        ),
        "jobs" => (
            "shop_id, name, last_run_day",
            &["1", "'stock_rederive'", "NULL"],
        ),
        // An open drawer: the four close columns are null together, which is
        // the arm `shifts_close_is_whole` lets through with nothing else set.
        // A probe that wants a closed one writes all four at once, the way
        // the annulée document above does, because the two CHECKs on this
        // table read three and four columns together.
        "shifts" => (
            "shop_id, opened_by, opened_at, opening_cash_centimes, closed_at, closed_by, \
             counted_centimes, expected_at_close_centimes, note",
            &[
                "1",
                "1",
                "'2026-09-21 09:00:00'",
                "500000",
                "NULL",
                "NULL",
                "NULL",
                "NULL",
                "NULL",
            ],
        ),
        // The document id is 1 because every case that writes a refund
        // seeds its document first, out of the `documents` template above,
        // into an empty file.
        "cash_refunds" => (
            "shop_id, document_id, user_id, amount_centimes, refunded_at",
            &["1", "1", "1", "300000", "'2026-09-21 12:00:00'"],
        ),
        other => panic!("no insert template for {other}"),
    };
    let names: Vec<&str> = columns.split(',').map(str::trim).collect();
    assert_eq!(names.len(), values.len(), "template for {table} is uneven");
    let mut row: Vec<&str> = values.to_vec();
    for (column, literal) in overrides {
        let at = names
            .iter()
            .position(|n| n == column)
            .unwrap_or_else(|| panic!("{table} template has no column {column}"));
        row[at] = literal;
    }
    format!(
        "INSERT INTO {table} ({}) VALUES ({})",
        names.join(", "),
        row.join(", ")
    )
}

pub fn probe(
    conn: &mut SqliteConnection,
    table: &str,
    column: &str,
    literal: &str,
) -> QueryResult<usize> {
    diesel::sql_query(insert_with(table, column, literal)).execute(conn)
}
