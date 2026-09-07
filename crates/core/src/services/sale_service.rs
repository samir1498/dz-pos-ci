use crate::models::*;
use crate::repos::transaction_repo;
use diesel::prelude::*;
use diesel::SqliteConnection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleItemInput {
    pub product_id: i32,
    pub product_name: String,
    pub quantity: i32,
    pub selling_price: f64,
    pub cost_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleResult {
    pub transaction: Transaction,
    pub items: Vec<TransactionItem>,
}

pub fn record_sale(
    conn: &mut SqliteConnection,
    timestamp: &str,
    items: &[SaleItemInput],
) -> QueryResult<SaleResult> {
    conn.transaction::<SaleResult, diesel::result::Error, _>(|conn| {
        let tx = transaction_repo::insert_transaction(
            conn,
            &NewTransaction {
                timestamp: timestamp.to_string(),
                total: 0.0,
            },
        )?;

        let new_items: Vec<NewTransactionItem> = items
            .iter()
            .map(|item| NewTransactionItem {
                transaction_id: tx.id,
                product_id: item.product_id,
                product_name: item.product_name.clone(),
                quantity: item.quantity,
                selling_price: item.selling_price,
                cost_price: item.cost_price,
                line_total: 0.0,
            })
            .collect();

        let result_items = transaction_repo::insert_transaction_items(conn, &new_items)?;
        let computed_total: f64 = result_items.iter().map(|i| i.line_total).sum();

        transaction_repo::update_transaction_total(conn, tx.id, computed_total)?;

        Ok(SaleResult {
            transaction: Transaction {
                total: computed_total,
                ..tx
            },
            items: result_items,
        })
    })
}

pub fn find_transactions_by_date(
    conn: &mut SqliteConnection,
    date: &str,
) -> QueryResult<Vec<Transaction>> {
    transaction_repo::find_transactions_by_date(conn, date)
}

pub fn find_transactions_paginated(
    conn: &mut SqliteConnection,
    date: &str,
    page: i64,
    per_page: i64,
) -> QueryResult<Vec<Transaction>> {
    transaction_repo::find_transactions_paginated(conn, date, page, per_page)
}

pub fn count_transactions_by_date(conn: &mut SqliteConnection, date: &str) -> QueryResult<i64> {
    transaction_repo::count_transactions_by_date(conn, date)
}

pub fn find_items_by_transaction(
    conn: &mut SqliteConnection,
    transaction_id: i32,
) -> QueryResult<Vec<TransactionItem>> {
    transaction_repo::find_items_by_transaction(conn, transaction_id)
}

pub fn daily_sales_total(conn: &mut SqliteConnection, date: &str) -> QueryResult<f64> {
    transaction_repo::daily_sales_total(conn, date)
}

pub fn daily_cost_total(conn: &mut SqliteConnection, date: &str) -> QueryResult<f64> {
    transaction_repo::daily_cost_total(conn, date)
}

pub fn transaction_count_by_date(conn: &mut SqliteConnection, date: &str) -> QueryResult<i64> {
    transaction_repo::transaction_count_by_date(conn, date)
}
