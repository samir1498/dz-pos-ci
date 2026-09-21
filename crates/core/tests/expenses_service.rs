// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Money out that is not stock (features.md §1, Expense): what the service
//! refuses, what it stores, and what one month of it comes to.

use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::money::Money;
use dzpos_core::services::clock::Month;
use dzpos_core::services::expenses::{self, NewExpense};

mod common;
use common::open_temp;

const SHOP: i32 = 1;
const OWNER: i32 = 1;

fn day(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
}

fn september() -> Month {
    Month::new(2026, 9).unwrap()
}

fn category(conn: &mut SqliteConnection, key: &str) -> i32 {
    expenses::categories(conn, SHOP)
        .unwrap()
        .into_iter()
        .find(|c| c.key == key)
        .unwrap()
        .id
}

fn spend(conn: &mut SqliteConnection, category_id: i32, centimes: i64, on: &str) -> i32 {
    expenses::create(
        conn,
        SHOP,
        OWNER,
        NewExpense {
            category_id,
            amount: Money::centimes(centimes),
            expense_date: day(on),
            note: None,
        },
    )
    .unwrap()
    .id
}

#[test]
fn an_expense_is_stamped_on_the_shop_clock_and_never_left_to_sqlite() {
    // The column carries `DEFAULT (CURRENT_TIMESTAMP)`, which SQLite answers
    // in UTC while Algiers is an hour ahead all year, so a row that took the
    // default was an hour early and an expense filed at 00:30 read as 23:30
    // the day before. Migration 000017 moved the rows written up to then;
    // this is the stamp that keeps the next one right.
    //
    // Asserted against the two UTC readings that bracket the call rather than
    // against a fixed moment, because the service reads the wall clock: what
    // is being pinned is which clock, and that answer is the same at every
    // hour of the day.
    let (_dir, mut conn) = open_temp();
    let rent = category(&mut conn, "rent");
    let before = chrono::Utc::now().naive_utc();
    let made = expenses::create(
        &mut conn,
        SHOP,
        OWNER,
        NewExpense {
            category_id: rent,
            amount: Money::centimes(3_000_000),
            expense_date: day("2026-09-10"),
            note: None,
        },
    )
    .unwrap();
    let after = chrono::Utc::now().naive_utc();
    // Bracketed by the two UTC readings rather than by a tolerance: the stamp
    // is `Utc::now() + 3600`, so it cannot land past `after + 3600`, and a
    // tolerance around 3600 would have to be wider than the slowest this test
    // ever runs. `before` is taken above the `create` call for the same reason.
    assert!(
        made.created_at >= before + chrono::Duration::seconds(3_600)
            && made.created_at <= after + chrono::Duration::seconds(3_600),
        "the expense was stamped {}, outside the shop-clock window {} to {}",
        made.created_at,
        before + chrono::Duration::seconds(3_600),
        after + chrono::Duration::seconds(3_600)
    );
}

#[test]
fn the_shop_starts_with_the_seven_seeded_categories_in_their_order() {
    let (_dir, mut conn) = open_temp();
    let keys: Vec<String> = expenses::categories(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .map(|c| c.key)
        .collect();
    assert_eq!(
        keys,
        vec![
            "rent",
            "electricity",
            "water",
            "salaries",
            "transport",
            "maintenance",
            "other",
        ]
    );
}

#[test]
fn an_expense_is_stored_with_the_amount_the_day_and_the_note_it_was_given() {
    let (_dir, mut conn) = open_temp();
    let rent = category(&mut conn, "rent");
    let made = expenses::create(
        &mut conn,
        SHOP,
        OWNER,
        NewExpense {
            category_id: rent,
            amount: Money::centimes(3_000_000),
            expense_date: day("2026-09-10"),
            note: Some("  loyer septembre  ".to_string()),
        },
    )
    .unwrap();
    assert_eq!(made.amount, Money::centimes(3_000_000));
    assert_eq!(made.expense_date, "2026-09-10");
    // Trimmed, like every other stored field.
    assert_eq!(made.note.as_deref(), Some("loyer septembre"));
    assert_eq!(made.user_id, OWNER);
}

#[test]
fn the_month_holds_its_own_days_and_nobody_elses_and_totals_them() {
    let (_dir, mut conn) = open_temp();
    let rent = category(&mut conn, "rent");
    let water = category(&mut conn, "water");
    let inside = spend(&mut conn, rent, 3_000_000, "2026-09-01");
    let last_day = spend(&mut conn, water, 150_000, "2026-09-30");
    spend(&mut conn, rent, 999_999, "2026-08-31");
    spend(&mut conn, rent, 999_999, "2026-10-01");

    let listed = expenses::list(&mut conn, SHOP, september()).unwrap();
    let ids: Vec<i32> = listed.iter().map(|e| e.id).collect();
    // Newest first: the day the shop reads its list in.
    assert_eq!(ids, vec![last_day, inside]);
    assert_eq!(
        expenses::total(&mut conn, SHOP, september()).unwrap(),
        Money::centimes(3_150_000)
    );
}

#[test]
fn an_empty_month_lists_nothing_and_comes_to_zero() {
    let (_dir, mut conn) = open_temp();
    assert!(expenses::list(&mut conn, SHOP, september())
        .unwrap()
        .is_empty());
    assert_eq!(
        expenses::total(&mut conn, SHOP, september()).unwrap(),
        Money::ZERO
    );
}

#[test]
fn an_amount_of_nothing_or_below_is_refused() {
    let (_dir, mut conn) = open_temp();
    let rent = category(&mut conn, "rent");
    for centimes in [0, -1] {
        let refused = expenses::create(
            &mut conn,
            SHOP,
            OWNER,
            NewExpense {
                category_id: rent,
                amount: Money::centimes(centimes),
                expense_date: day("2026-09-10"),
                note: None,
            },
        );
        assert_eq!(field_of(refused), Some("amount".to_string()));
    }
}

#[test]
fn a_category_of_another_shop_or_a_retired_one_is_refused() {
    let (_dir, mut conn) = open_temp();
    let rent = category(&mut conn, "rent");
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO expense_categories (id, shop_id, key, sort_order, active) \
         VALUES (99, 2, 'rent', 1, 1)",
    )
    .execute(&mut conn)
    .unwrap();
    let elsewhere = expenses::create(
        &mut conn,
        SHOP,
        OWNER,
        NewExpense {
            category_id: 99,
            amount: Money::centimes(1_000),
            expense_date: day("2026-09-10"),
            note: None,
        },
    );
    assert_eq!(field_of(elsewhere), Some("category_id".to_string()));

    diesel::sql_query(format!(
        "UPDATE expense_categories SET active = 0 WHERE id = {rent}"
    ))
    .execute(&mut conn)
    .unwrap();
    let retired = expenses::create(
        &mut conn,
        SHOP,
        OWNER,
        NewExpense {
            category_id: rent,
            amount: Money::centimes(1_000),
            expense_date: day("2026-09-10"),
            note: None,
        },
    );
    assert_eq!(field_of(retired), Some("category_id".to_string()));
    // The expenses already filed under it stay readable.
    assert!(expenses::categories(&mut conn, SHOP)
        .unwrap()
        .iter()
        .any(|c| c.id == rent && !c.active));
}

#[test]
fn writing_an_expense_leaves_the_audit_entry_its_own_change_writes() {
    let (_dir, mut conn) = open_temp();
    let rent = category(&mut conn, "rent");
    let made = spend(&mut conn, rent, 3_000_000, "2026-09-10");
    let entry = dzpos_core::services::audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .find(|e| e.entity == "expense")
        .unwrap();
    assert_eq!(entry.action, "expense.create");
    assert_eq!(entry.entity_id, Some(made));
    assert_eq!(entry.before, None);
    let after: serde_json::Value = serde_json::from_str(&entry.after.unwrap()).unwrap();
    assert_eq!(after["amount_centimes"], serde_json::json!(3_000_000));
    assert_eq!(after["expense_date"], serde_json::json!("2026-09-10"));
    assert_eq!(after["category_key"], serde_json::json!("rent"));
}

#[test]
fn a_month_outside_the_calendar_is_refused_rather_than_read_as_january() {
    assert!(Month::new(2026, 0).is_err());
    assert!(Month::new(2026, 13).is_err());
    let december = Month::new(2026, 12).unwrap();
    assert_eq!(december.last_day(), day("2026-12-31"));
    assert_eq!(december.first_day(), day("2026-12-01"));
}

fn field_of<T>(result: Result<T, dzpos_core::error::CoreError>) -> Option<String> {
    match result {
        Err(dzpos_core::error::CoreError::Validation { field, .. }) => Some(field),
        _ => None,
    }
}
