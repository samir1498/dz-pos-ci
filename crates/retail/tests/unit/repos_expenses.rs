// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::repos::testdb::{open, OWNER, SHOP};

/// A moment on the shop's clock, so nothing below depends on the wall
/// clock. 00:30 is the hour the column's own UTC default used to store as
/// 23:30 the day before, which migration 000017 put right.
fn filed_at() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 9, 11)
        .and_then(|d| d.and_hms_opt(0, 30, 0))
        .unwrap()
}

#[test]
fn the_shop_starts_with_the_seven_categories_the_spec_names_in_their_order() {
    // The migration seeds them per shop and the row carries the i18n key,
    // not a label: the desktop reads the three languages by that key.
    let (_dir, mut conn) = open();
    let keys: Vec<String> = categories(&mut conn, SHOP)
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
            "other"
        ]
    );
}

#[test]
fn a_category_added_by_the_shop_takes_its_place_in_the_list() {
    let (_dir, mut conn) = open();
    insert_category(
        &mut conn,
        &ExpenseCategoryRowWrite {
            shop_id: SHOP,
            key: "internet".to_string(),
            sort_order: 8,
            active: true,
        },
    )
    .unwrap();
    let listed = categories(&mut conn, SHOP).unwrap();
    assert_eq!(listed.len(), 8);
    assert_eq!(listed[7].key, "internet");
    // A key is the shop's own and is taken once.
    assert!(insert_category(
        &mut conn,
        &ExpenseCategoryRowWrite {
            shop_id: SHOP,
            key: "internet".to_string(),
            sort_order: 9,
            active: true,
        },
    )
    .is_err());
}

#[test]
fn an_expense_is_read_back_with_its_amount_its_day_and_its_category() {
    let (_dir, mut conn) = open();
    let rent = categories(&mut conn, SHOP).unwrap()[0].id;
    let made = insert(
        &mut conn,
        &ExpenseRowWrite {
            shop_id: SHOP,
            category_id: rent,
            amount_centimes: 3_000_000,
            expense_date: "2026-09-10".to_string(),
            note: Some("loyer septembre".to_string()),
            user_id: OWNER,
            created_at: filed_at(),
        },
    )
    .unwrap();
    let read = get(&mut conn, SHOP, made.id).unwrap();
    assert_eq!(read, made);
    // The moment the caller stamped, unchanged. Written by the service
    // from the shop's clock, never left to the column's UTC default.
    assert_eq!(read.created_at, filed_at());
    assert_eq!(read.amount, crate::money::Money::centimes(3_000_000));
    assert_eq!(read.category_id, rent);
    assert_eq!(read.note.as_deref(), Some("loyer septembre"));
}

#[test]
fn the_list_is_newest_first_and_another_shops_expenses_are_not_in_it() {
    let (_dir, mut conn) = open();
    let rent = categories(&mut conn, SHOP).unwrap()[0].id;
    let mut ids = Vec::new();
    for day in ["2026-09-01", "2026-09-10"] {
        ids.push(
            insert(
                &mut conn,
                &ExpenseRowWrite {
                    shop_id: SHOP,
                    category_id: rent,
                    amount_centimes: 1_000,
                    expense_date: day.to_string(),
                    note: None,
                    user_id: OWNER,
                    created_at: filed_at(),
                },
            )
            .unwrap()
            .id,
        );
    }
    ids.reverse();
    let read: Vec<i32> = list_between(&mut conn, SHOP, "2026-09-01", "2026-09-30")
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(read, ids);
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(&mut conn)
        .unwrap();
    assert!(list_between(&mut conn, 2, "2026-09-01", "2026-09-30")
        .unwrap()
        .is_empty());
    assert!(get(&mut conn, 2, ids[0]).is_err());
    // The seed runs once, over the shops on the file at the time, so a
    // shop the migration never saw carries none of its own.
    assert!(categories(&mut conn, 2).unwrap().is_empty());
}

#[test]
fn the_total_covers_the_days_asked_for_and_a_stretch_with_nothing_in_it_is_zero() {
    let (_dir, mut conn) = open();
    let rent = categories(&mut conn, SHOP).unwrap()[0].id;
    for (day, centimes) in [
        ("2026-08-31", 700_i64),
        ("2026-09-01", 1_000),
        ("2026-09-30", 2_000),
        ("2026-10-01", 900),
    ] {
        insert(
            &mut conn,
            &ExpenseRowWrite {
                shop_id: SHOP,
                category_id: rent,
                amount_centimes: centimes,
                expense_date: day.to_string(),
                note: None,
                user_id: OWNER,
                created_at: filed_at(),
            },
        )
        .unwrap();
    }
    // Both ends are inside: the first and the last day of a month are
    // days the shop spent money on like any other.
    assert_eq!(
        total_between(&mut conn, SHOP, "2026-09-01", "2026-09-30").unwrap(),
        crate::money::Money::centimes(3_000)
    );
    assert_eq!(
        total_between(&mut conn, SHOP, "2026-07-01", "2026-07-31").unwrap(),
        crate::money::Money::ZERO
    );
}
