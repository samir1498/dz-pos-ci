// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Migration 000029, a purchase's own number (`purchases.series_year`,
//! `purchases.number`, the `purchase:<year>` counters), and migration 000030,
//! a product's pack size (`products.contenance_milli`, `contenance_unit`).
//!
//! In its own file beside `migration_shifts.rs` and not in `migration.rs`,
//! which is at the length `scripts/file-sizes.json` pins it to.

use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

mod common;

use common::migrations::{count, insert_with_all, open_before_migration, probe};

/// Four orders already on the file when the migration runs, written in this
/// id order: 2026, 2025, 2026, 2025. Numbered by hand below.
fn seed_four_orders(conn: &mut SqliteConnection) {
    assert_eq!(
        diesel::sql_query(insert_with_all("suppliers", &[]))
            .execute(conn)
            .unwrap(),
        1
    );
    for day in [
        "'2026-01-05'",
        "'2025-12-30'",
        "'2026-02-01'",
        "'2025-12-31'",
    ] {
        assert_eq!(
            diesel::sql_query(insert_with_all("purchases", &[("purchase_date", day)]))
                .execute(conn)
                .unwrap(),
            1
        );
    }
}

/// `(id, series_year, number)` for every order, by id.
fn numbers(conn: &mut SqliteConnection) -> Vec<(i32, i32, i64)> {
    use dzpos_retail::schema::purchases::dsl::*;
    purchases
        .select((id, series_year, number))
        .order(id.asc())
        .load(conn)
        .unwrap()
}

fn next_value(conn: &mut SqliteConnection, name: &str) -> i32 {
    count(
        conn,
        &format!("SELECT next_value AS n FROM counters WHERE shop_id = 1 AND name = '{name}'"),
    )
}

#[test]
fn the_orders_on_the_file_are_numbered_in_id_order_inside_their_year_and_the_revert_takes_it_back()
{
    let (_dir, mut conn) = open_before_migration("purchase_series");
    seed_four_orders(&mut conn);
    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    conn.run_migration(&pending[0]).unwrap();

    // 2026: ids 1 and 3 are its first and second; 2025: ids 2 and 4.
    let expected = vec![(1, 2026, 1), (2, 2025, 1), (3, 2026, 2), (4, 2025, 2)];
    assert_eq!(numbers(&mut conn), expected);
    // Each year's counter carries on after its last order, so the next order
    // of 2026 is number 3 and not a second number 1.
    assert_eq!(next_value(&mut conn, "purchase:2026"), 3);
    assert_eq!(next_value(&mut conn, "purchase:2025"), 3);

    // Two orders of one year cannot hold the same number.
    let clash =
        diesel::sql_query("UPDATE purchases SET number = 1 WHERE id = 3").execute(&mut conn);
    assert!(clash.is_err(), "two orders of 2026 both held number 1");

    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('purchases') \
             WHERE name IN ('series_year', 'number')"
        ),
        0,
        "the down.sql left a column behind"
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM counters WHERE name LIKE 'purchase:%'"
        ),
        0,
        "the down.sql left a counter behind"
    );
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM purchases"),
        4,
        "the revert took an order with it"
    );

    // And up again: the same file gives the same numbers.
    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    conn.run_migration(&pending[0]).unwrap();
    assert_eq!(numbers(&mut conn), expected);
}

#[test]
fn a_pack_size_is_both_columns_or_neither_and_the_revert_drops_them() {
    let (_dir, mut conn) = open_before_migration("product_contenance");
    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    conn.run_migration(&pending[0]).unwrap();

    // Written out rather than through the shared template, which names the
    // columns every migration before this one had.
    let product = |milli: &str, unit: &str| {
        format!(
            "INSERT INTO products (shop_id, name, unit, cost_centimes, selling_centimes, \
             qty_on_hand_milli, low_stock_at_milli, rate_bps, contenance_milli, \
             contenance_unit) VALUES (1, 'Huile', 'piece', 0, 0, 0, 0, 1900, {milli}, {unit})"
        )
    };
    // Neither, and both with each of the four units.
    for (milli, unit) in [
        ("NULL", "NULL"),
        ("1500", "'l'"),
        ("250", "'g'"),
        ("1000", "'kg'"),
        ("330", "'ml'"),
    ] {
        assert!(
            diesel::sql_query(product(milli, unit))
                .execute(&mut conn)
                .is_ok(),
            "{milli} {unit} was refused"
        );
    }
    // Half a figure, a unit the shop does not weigh in, and a pack of
    // nothing or less, or not an integer.
    for (milli, unit) in [
        ("1500", "NULL"),
        ("NULL", "'l'"),
        ("1500", "'cl'"),
        ("0", "'l'"),
        ("-1", "'l'"),
        ("1.5", "'l'"),
        ("'abc'", "'l'"),
    ] {
        assert!(
            diesel::sql_query(product(milli, unit))
                .execute(&mut conn)
                .is_err(),
            "{milli} {unit} was taken"
        );
    }
    // The template row still goes in untouched: the two columns are optional.
    assert!(probe(&mut conn, "products", "name", "'Sucre 1kg'").is_ok());

    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_table_info('products') \
             WHERE name LIKE 'contenance%'"
        ),
        0,
        "the down.sql left a column behind"
    );
}
