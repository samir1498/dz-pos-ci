use crate::models::*;
use crate::repos::{expense_repo, product_repo, transaction_repo};
use diesel::prelude::*;
use diesel::SqliteConnection;

#[derive(Debug, Clone, Default)]
pub struct DailyStats {
    pub sales_total: f64,
    pub expenses_total: f64,
    pub transaction_count: i64,
}

pub fn daily_stats(conn: &mut SqliteConnection, date: &str) -> QueryResult<DailyStats> {
    let sales_total = transaction_repo::daily_sales_total(conn, date)?;
    let expenses_total = expense_repo::expense_total_by_range(conn, date, date)?;
    let transaction_count = transaction_repo::transaction_count_by_date(conn, date)?;

    Ok(DailyStats {
        sales_total,
        expenses_total,
        transaction_count,
    })
}

pub fn low_stock_products(
    conn: &mut SqliteConnection,
    threshold: i32,
) -> QueryResult<Vec<Product>> {
    product_repo::find_low_stock_products(conn, threshold)
}

pub fn profit(sales_total: f64, cost_price_total: f64, expenses_total: f64) -> f64 {
    sales_total - cost_price_total - expenses_total
}
