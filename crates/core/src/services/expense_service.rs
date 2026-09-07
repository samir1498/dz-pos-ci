use crate::models::*;
use crate::repos::expense_repo;
use diesel::prelude::*;
use diesel::SqliteConnection;

pub fn create(conn: &mut SqliteConnection, expense: &NewExpense) -> QueryResult<Expense> {
    expense_repo::insert_expense(conn, expense)
}

pub fn find_by_date_range(
    conn: &mut SqliteConnection,
    start_date: &str,
    end_date: &str,
) -> QueryResult<Vec<Expense>> {
    expense_repo::find_expenses_by_date_range(conn, start_date, end_date)
}

pub fn find_paginated(
    conn: &mut SqliteConnection,
    start_date: &str,
    end_date: &str,
    page: i64,
    per_page: i64,
) -> QueryResult<Vec<Expense>> {
    expense_repo::find_expenses_paginated(conn, start_date, end_date, page, per_page)
}

pub fn count_by_date_range(
    conn: &mut SqliteConnection,
    start_date: &str,
    end_date: &str,
) -> QueryResult<i64> {
    expense_repo::count_expenses_by_range(conn, start_date, end_date)
}

pub fn total_by_range(
    conn: &mut SqliteConnection,
    start_date: &str,
    end_date: &str,
) -> QueryResult<f64> {
    expense_repo::expense_total_by_range(conn, start_date, end_date)
}

pub fn delete(conn: &mut SqliteConnection, expense_id: i32) -> QueryResult<bool> {
    expense_repo::delete_expense(conn, expense_id)
}
