use crate::models::*;
use crate::repos::expense_category_repo;
use diesel::prelude::*;
use diesel::SqliteConnection;

pub fn find_all(conn: &mut SqliteConnection) -> QueryResult<Vec<ExpenseCategory>> {
    expense_category_repo::find_all_categories(conn)
}

pub fn create(
    conn: &mut SqliteConnection,
    category: &NewExpenseCategory,
) -> QueryResult<ExpenseCategory> {
    expense_category_repo::insert_category(conn, category)
}

pub fn delete(conn: &mut SqliteConnection, category_id: i32) -> QueryResult<bool> {
    expense_category_repo::delete_category(conn, category_id)
}
