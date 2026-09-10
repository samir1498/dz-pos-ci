// The callers are T2 (suppliers), T3 (purchases), T4 (expenses) and T5 (the
// re-derive job): the tables land here before the services that read them, and
// `repos` is crate-internal on purpose (architecture.md: nothing outside this
// crate touches diesel), so a plain build sees no use of these yet.
#![allow(dead_code)]

//! The only place expenses and their categories touch diesel. Every query is
//! scoped by `shop_id` (rule 3).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::expense::{
    Expense, ExpenseCategory, ExpenseCategoryRow, ExpenseCategoryRowWrite, ExpenseRow,
    ExpenseRowWrite,
};
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

/// The shop's expenses, newest first. `expense_date` is a day and several
/// land on one, so the id breaks the tie.
pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<Expense>, CoreError> {
    let rows: Vec<ExpenseRow> = expenses::table
        .filter(expenses::shop_id.eq(shop_id))
        .order((expenses::expense_date.desc(), expenses::id.desc()))
        .select(ExpenseRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(Expense::from).collect())
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
        let read: Vec<i32> = list(&mut conn, SHOP)
            .unwrap()
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(read, ids);
        diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
            .execute(&mut conn)
            .unwrap();
        assert!(list(&mut conn, 2).unwrap().is_empty());
        assert!(get(&mut conn, 2, ids[0]).is_err());
        // A shop created after the migration is seeded by the service that
        // creates it, so it starts with none of its own.
        assert!(categories(&mut conn, 2).unwrap().is_empty());
    }
}
