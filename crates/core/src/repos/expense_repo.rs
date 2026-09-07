use crate::models::*;
use crate::schema::expenses;
use diesel::prelude::*;
use diesel::SqliteConnection;

pub fn insert_expense(conn: &mut SqliteConnection, e: &NewExpense) -> QueryResult<Expense> {
    diesel::insert_into(expenses::table)
        .values(e)
        .returning(Expense::as_returning())
        .get_result(conn)
}

pub fn find_expenses_by_date_range(
    conn: &mut SqliteConnection,
    start: &str,
    end: &str,
) -> QueryResult<Vec<Expense>> {
    expenses::table
        .filter(expenses::date.ge(start))
        .filter(expenses::date.le(end))
        .order(expenses::date.desc())
        .select(Expense::as_select())
        .load(conn)
}

pub fn find_expenses_paginated(
    conn: &mut SqliteConnection,
    start: &str,
    end: &str,
    page: i64,
    per_page: i64,
) -> QueryResult<Vec<Expense>> {
    expenses::table
        .filter(expenses::date.ge(start))
        .filter(expenses::date.le(end))
        .order(expenses::date.desc())
        .select(Expense::as_select())
        .offset((page - 1).max(0) * per_page)
        .limit(per_page)
        .load(conn)
}

pub fn count_expenses_by_range(
    conn: &mut SqliteConnection,
    start: &str,
    end: &str,
) -> QueryResult<i64> {
    use diesel::dsl::count;
    expenses::table
        .filter(expenses::date.ge(start))
        .filter(expenses::date.le(end))
        .select(count(expenses::id))
        .first::<i64>(conn)
}

pub fn find_expenses_by_category(
    conn: &mut SqliteConnection,
    category: &str,
    start: &str,
    end: &str,
) -> QueryResult<Vec<Expense>> {
    expenses::table
        .filter(expenses::category.eq(category))
        .filter(expenses::date.ge(start))
        .filter(expenses::date.le(end))
        .order(expenses::date.desc())
        .select(Expense::as_select())
        .load(conn)
}

pub fn delete_expense(conn: &mut SqliteConnection, expense_id: i32) -> QueryResult<bool> {
    let rows = diesel::delete(expenses::table.find(expense_id)).execute(conn)?;
    Ok(rows > 0)
}

pub fn expense_total_by_range(
    conn: &mut SqliteConnection,
    start: &str,
    end: &str,
) -> QueryResult<f64> {
    use diesel::dsl::sum;
    expenses::table
        .filter(expenses::date.ge(start))
        .filter(expenses::date.le(end))
        .select(sum(expenses::amount))
        .first(conn)
        .map(|v: Option<f64>| v.unwrap_or(0.0))
}
