use crate::models::*;
use crate::schema::expense_categories;
use diesel::prelude::*;
use diesel::SqliteConnection;

pub fn insert_category(
    conn: &mut SqliteConnection,
    c: &NewExpenseCategory,
) -> QueryResult<ExpenseCategory> {
    diesel::insert_into(expense_categories::table)
        .values(c)
        .returning(ExpenseCategory::as_returning())
        .get_result(conn)
}

pub fn find_all_categories(conn: &mut SqliteConnection) -> QueryResult<Vec<ExpenseCategory>> {
    expense_categories::table
        .order(expense_categories::name.asc())
        .select(ExpenseCategory::as_select())
        .load(conn)
}

pub fn delete_category(conn: &mut SqliteConnection, category_id: i32) -> QueryResult<bool> {
    let rows = diesel::delete(expense_categories::table.find(category_id)).execute(conn)?;
    Ok(rows > 0)
}
