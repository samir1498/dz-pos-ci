//! The only place expenses and their categories touch diesel. Every query is
//! scoped by `shop_id` (rule 3).

use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Nullable};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::expense::{
    Expense, ExpenseCategory, ExpenseCategoryRow, ExpenseCategoryRowWrite, ExpenseRow,
    ExpenseRowWrite,
};
use crate::money::Money;
use crate::schema::{expense_categories, expenses};

/// The shop's categories in the order the screen lists them. The id closes
/// the order because two categories may share a place in it, and a list that
/// reordered itself between two reads is one an operator cannot point at.
pub fn categories(
    conn: &mut SqliteConnection,
    shop_id: i32,
) -> Result<Vec<ExpenseCategory>, CoreError> {
    let rows: Vec<ExpenseCategoryRow> = expense_categories::table
        .filter(expense_categories::shop_id.eq(shop_id))
        .order((
            expense_categories::sort_order.asc(),
            expense_categories::id.asc(),
        ))
        .select(ExpenseCategoryRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(ExpenseCategory::from).collect())
}

/// Nothing ships a caller for this yet: a shop cannot add a category of its
/// own today (the seven seeded keys are the list, and the desktop holds their
/// labels in the three languages by key), and the table allows one so a later
/// screen adds a row rather than a migration. The round trip is tested here
/// so the query is known good the day that screen exists.
#[cfg_attr(not(test), allow(dead_code))]
pub fn insert_category(
    conn: &mut SqliteConnection,
    write: &ExpenseCategoryRowWrite,
) -> Result<ExpenseCategory, CoreError> {
    let row: ExpenseCategoryRow = diesel::insert_into(expense_categories::table)
        .values(write)
        .returning(ExpenseCategoryRow::as_returning())
        .get_result(conn)?;
    Ok(ExpenseCategory::from(row))
}

pub fn insert(conn: &mut SqliteConnection, write: &ExpenseRowWrite) -> Result<Expense, CoreError> {
    let row: ExpenseRow = diesel::insert_into(expenses::table)
        .values(write)
        .returning(ExpenseRow::as_returning())
        .get_result(conn)?;
    Ok(Expense::from(row))
}

/// One expense on its own. No screen opens one today: an expense is never
/// edited and never deleted, so the list is the whole of what is read. Kept
/// because the tests below read a row back the way a later detail panel
/// would.
#[cfg_attr(not(test), allow(dead_code))]
pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Expense, CoreError> {
    let row: ExpenseRow = expenses::table
        .filter(expenses::shop_id.eq(shop_id))
        .filter(expenses::id.eq(id))
        .select(ExpenseRow::as_select())
        .first(conn)
        .optional()?
        .ok_or(CoreError::NotFound {
            entity: "expense",
            id,
        })?;
    Ok(Expense::from(row))
}

/// The shop's expenses over a stretch of days, both ends included, newest
/// first. `expense_date` is a day written `YYYY-MM-DD`, so the two bounds are
/// compared as text and the index on (shop_id, expense_date) is the one the
/// query walks.
pub fn list_between(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: &str,
    to: &str,
) -> Result<Vec<Expense>, CoreError> {
    let rows: Vec<ExpenseRow> = expenses::table
        .filter(expenses::shop_id.eq(shop_id))
        .filter(expenses::expense_date.between(from, to))
        .order((expenses::expense_date.desc(), expenses::id.desc()))
        .select(ExpenseRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(Expense::from).collect())
}

/// What the shop spent over those days. Summed by SQLite rather than by
/// adding a list up in Rust, because the cash position asks this of a month
/// it never lists; the SUM is coalesced, so a stretch with no expense in it
/// answers zero rather than nothing.
pub fn total_between(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: &str,
    to: &str,
) -> Result<Money, CoreError> {
    let total: Option<i64> = expenses::table
        .filter(expenses::shop_id.eq(shop_id))
        .filter(expenses::expense_date.between(from, to))
        .select(sql::<Nullable<BigInt>>("SUM(amount_centimes)"))
        .first(conn)?;
    Ok(Money::centimes(total.unwrap_or(0)))
}

#[cfg(test)]
mod tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::repos::testdb::{open, OWNER, SHOP};

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
            },
        )
        .unwrap();
        let read = get(&mut conn, SHOP, made.id).unwrap();
        assert_eq!(read, made);
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
}
