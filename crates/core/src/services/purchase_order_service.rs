use crate::models::*;
use crate::repos::purchase_order_repo;
use crate::services::supplier_service;
use diesel::prelude::*;
use diesel::SqliteConnection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderInput {
    pub purchase_order_number: String,
    pub supplier_id: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderItemInput {
    pub product_name: String,
    pub quantity: i32,
    pub unit_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderResult {
    pub order: PurchaseOrder,
    pub items: Vec<PurchaseOrderItem>,
}

pub fn create_purchase_order(
    conn: &mut SqliteConnection,
    po: &PurchaseOrderInput,
    items: &[PurchaseOrderItemInput],
) -> QueryResult<PurchaseOrderResult> {
    conn.transaction::<PurchaseOrderResult, diesel::result::Error, _>(|conn| {
        let order = purchase_order_repo::insert_purchase_order(
            conn,
            &NewPurchaseOrder {
                purchase_order_number: po.purchase_order_number.clone(),
                supplier_id: po.supplier_id,
                status: "Draft".to_string(),
                notes: po.notes.clone(),
                total: 0.0,
            },
        )?;

        let new_items: Vec<NewPurchaseOrderItem> = items
            .iter()
            .map(|item| NewPurchaseOrderItem {
                purchase_order_id: order.id,
                product_name: item.product_name.clone(),
                quantity: item.quantity,
                unit_cost: item.unit_cost,
                line_total: 0.0,
            })
            .collect();

        let result_items = purchase_order_repo::insert_purchase_order_items(conn, &new_items)?;

        let computed_total: f64 = result_items.iter().map(|i| i.line_total).sum();

        purchase_order_repo::update_purchase_order_total(conn, order.id, computed_total)?;

        let updated_order = purchase_order_repo::find_purchase_order_by_id(conn, order.id)?
            .expect("order should exist after insert");

        Ok(PurchaseOrderResult {
            order: updated_order,
            items: result_items,
        })
    })
}

pub fn receive_purchase_order(
    conn: &mut SqliteConnection,
    purchase_order_id: i32,
) -> QueryResult<()> {
    purchase_order_repo::receive_purchase_order(conn, purchase_order_id)
}

pub fn find_paginated(
    conn: &mut SqliteConnection,
    page: i64,
    per_page: i64,
) -> QueryResult<Vec<PurchaseOrder>> {
    purchase_order_repo::find_purchase_orders_paginated(conn, page, per_page)
}

pub fn count_all(conn: &mut SqliteConnection) -> QueryResult<i64> {
    purchase_order_repo::count_purchase_orders(conn)
}

pub fn delete(conn: &mut SqliteConnection, purchase_order_id: i32) -> QueryResult<bool> {
    purchase_order_repo::delete_purchase_order(conn, purchase_order_id)
}

pub fn supplier_name(conn: &mut SqliteConnection, supplier_id: Option<i32>) -> Option<String> {
    let id = supplier_id?;
    supplier_service::find_by_id(conn, id).ok()?.map(|s| s.name)
}
