use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = crate::schema::transactions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Transaction {
    pub id: i32,
    pub timestamp: String,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = crate::schema::transactions)]
pub struct NewTransaction {
    pub timestamp: String,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = crate::schema::transaction_items)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TransactionItem {
    pub id: i32,
    pub transaction_id: i32,
    pub product_id: i32,
    pub product_name: String,
    pub quantity: i32,
    pub selling_price: f64,
    pub cost_price: f64,
    pub line_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = crate::schema::transaction_items)]
pub struct NewTransactionItem {
    pub transaction_id: i32,
    pub product_id: i32,
    pub product_name: String,
    pub quantity: i32,
    pub selling_price: f64,
    pub cost_price: f64,
    pub line_total: f64,
}
