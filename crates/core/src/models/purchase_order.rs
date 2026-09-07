use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = crate::schema::purchase_orders)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PurchaseOrder {
    pub id: i32,
    pub purchase_order_number: String,
    pub supplier_id: Option<i32>,
    pub status: String,
    pub notes: Option<String>,
    pub total: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = crate::schema::purchase_orders)]
pub struct NewPurchaseOrder {
    pub purchase_order_number: String,
    pub supplier_id: Option<i32>,
    pub status: String,
    pub notes: Option<String>,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = crate::schema::purchase_order_items)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PurchaseOrderItem {
    pub id: i32,
    pub purchase_order_id: i32,
    pub product_name: String,
    pub quantity: i32,
    pub unit_cost: f64,
    pub line_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = crate::schema::purchase_order_items)]
pub struct NewPurchaseOrderItem {
    pub purchase_order_id: i32,
    pub product_name: String,
    pub quantity: i32,
    pub unit_cost: f64,
    pub line_total: f64,
}

#[derive(Debug, Clone)]
pub struct PurchaseOrderWithItems {
    pub order: PurchaseOrder,
    pub items: Vec<PurchaseOrderItem>,
    pub supplier_name: Option<String>,
}
