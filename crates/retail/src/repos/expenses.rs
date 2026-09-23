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
#[path = "../../tests/unit/repos_expenses.rs"]
mod tests;
