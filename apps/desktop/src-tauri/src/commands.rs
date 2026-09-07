use diesel::SqliteConnection;
use dzpos_core::models::*;
use dzpos_core::services;
use serde::{Deserialize, Serialize};
use tauri::State;

pub struct DbState {
    pub db_path: String,
}

fn conn(state: &DbState) -> SqliteConnection {
    dzpos_core::db::open(&state.db_path).expect("Failed to open database")
}

// ── Shared types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub sales_total: f64,
    pub expenses_total: f64,
    pub transaction_count: i64,
    pub cost_total: f64,
    pub profit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleInput {
    pub timestamp: String,
    pub items: Vec<SaleItemInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleItemInput {
    pub product_id: i32,
    pub product_name: String,
    pub quantity: i32,
    pub selling_price: f64,
    pub cost_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePurchaseOrderInput {
    pub purchase_order_number: String,
    pub supplier_id: Option<i32>,
    pub notes: Option<String>,
    pub items: Vec<PurchaseOrderItemInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderItemInput {
    pub product_name: String,
    pub quantity: i32,
    pub unit_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExpenseInput {
    pub date: String,
    pub category: String,
    pub description: String,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductInput {
    pub name: String,
    pub barcode: Option<String>,
    pub cost_price: f64,
    pub selling_price: f64,
    pub quantity_on_hand: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierInput {
    pub name: String,
    pub phone: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbInfo {
    pub db_path: String,
    pub product_count: i64,
}

// ── Commands ──

#[tauri::command]
pub fn get_db_path(state: State<DbState>) -> String {
    state.db_path.clone()
}

// ── Products ──

#[tauri::command]
pub fn list_products(
    state: State<DbState>,
    page: Option<i64>,
    per_page: Option<i64>,
) -> Result<PaginatedResult<Product>, String> {
    let mut conn = conn(&state);
    let p = page.unwrap_or(1);
    let pp = per_page.unwrap_or(20);
    let total = services::product_service::count_all(&mut conn).map_err(|e| e.to_string())?;
    let items =
        services::product_service::find_paginated(&mut conn, p, pp).map_err(|e| e.to_string())?;
    Ok(PaginatedResult {
        items,
        total,
        page: p,
        per_page: pp,
    })
}

#[tauri::command]
pub fn get_product(state: State<DbState>, id: i32) -> Result<Option<Product>, String> {
    let mut conn = conn(&state);
    services::product_service::find_by_id(&mut conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_product(state: State<DbState>, input: ProductInput) -> Result<Product, String> {
    let mut conn = conn(&state);
    let new = NewProduct {
        name: input.name,
        barcode: input.barcode,
        cost_price: input.cost_price,
        selling_price: input.selling_price,
        quantity_on_hand: input.quantity_on_hand,
    };
    services::product_service::create(&mut conn, &new).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_product(state: State<DbState>, id: i32, input: ProductInput) -> Result<(), String> {
    let mut conn = conn(&state);
    let new = NewProduct {
        name: input.name,
        barcode: input.barcode,
        cost_price: input.cost_price,
        selling_price: input.selling_price,
        quantity_on_hand: input.quantity_on_hand,
    };
    services::product_service::update(&mut conn, id, &new).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_product(state: State<DbState>, id: i32) -> Result<bool, String> {
    let mut conn = conn(&state);
    services::product_service::delete(&mut conn, id).map_err(|e| e.to_string())
}

// ── Sales ──

#[tauri::command]
pub fn record_sale(
    state: State<DbState>,
    input: SaleInput,
) -> Result<services::sale_service::SaleResult, String> {
    let mut conn = conn(&state);
    let items: Vec<services::sale_service::SaleItemInput> = input
        .items
        .iter()
        .map(|i| services::sale_service::SaleItemInput {
            product_id: i.product_id,
            product_name: i.product_name.clone(),
            quantity: i.quantity,
            selling_price: i.selling_price,
            cost_price: i.cost_price,
        })
        .collect();
    services::sale_service::record_sale(&mut conn, &input.timestamp, &items)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_sales(
    state: State<DbState>,
    date: String,
    page: Option<i64>,
    per_page: Option<i64>,
) -> Result<PaginatedResult<Transaction>, String> {
    let mut conn = conn(&state);
    let p = page.unwrap_or(1);
    let pp = per_page.unwrap_or(20);
    let total = services::sale_service::count_transactions_by_date(&mut conn, &date)
        .map_err(|e| e.to_string())?;
    let items = services::sale_service::find_transactions_paginated(&mut conn, &date, p, pp)
        .map_err(|e| e.to_string())?;
    Ok(PaginatedResult {
        items,
        total,
        page: p,
        per_page: pp,
    })
}

// ── Purchase Orders ──

#[tauri::command]
pub fn list_purchase_orders(
    state: State<DbState>,
    page: Option<i64>,
    per_page: Option<i64>,
) -> Result<PaginatedResult<PurchaseOrder>, String> {
    let mut conn = conn(&state);
    let p = page.unwrap_or(1);
    let pp = per_page.unwrap_or(20);
    let total =
        services::purchase_order_service::count_all(&mut conn).map_err(|e| e.to_string())?;
    let items = services::purchase_order_service::find_paginated(&mut conn, p, pp)
        .map_err(|e| e.to_string())?;
    Ok(PaginatedResult {
        items,
        total,
        page: p,
        per_page: pp,
    })
}

#[tauri::command]
pub fn create_purchase_order(
    state: State<DbState>,
    input: CreatePurchaseOrderInput,
) -> Result<services::purchase_order_service::PurchaseOrderResult, String> {
    let mut conn = conn(&state);
    let po_input = services::purchase_order_service::PurchaseOrderInput {
        purchase_order_number: input.purchase_order_number,
        supplier_id: input.supplier_id,
        notes: input.notes,
    };
    let items: Vec<services::purchase_order_service::PurchaseOrderItemInput> = input
        .items
        .iter()
        .map(
            |i| services::purchase_order_service::PurchaseOrderItemInput {
                product_name: i.product_name.clone(),
                quantity: i.quantity,
                unit_cost: i.unit_cost,
            },
        )
        .collect();
    services::purchase_order_service::create_purchase_order(&mut conn, &po_input, &items)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn receive_purchase_order(state: State<DbState>, id: i32) -> Result<(), String> {
    let mut conn = conn(&state);
    services::purchase_order_service::receive_purchase_order(&mut conn, id)
        .map_err(|e| e.to_string())
}

// ── Expenses ──

#[tauri::command]
pub fn list_expenses(
    state: State<DbState>,
    start_date: String,
    end_date: String,
    page: Option<i64>,
    per_page: Option<i64>,
) -> Result<PaginatedResult<Expense>, String> {
    let mut conn = conn(&state);
    let p = page.unwrap_or(1);
    let pp = per_page.unwrap_or(20);
    let total = services::expense_service::count_by_date_range(&mut conn, &start_date, &end_date)
        .map_err(|e| e.to_string())?;
    let items = services::expense_service::find_paginated(&mut conn, &start_date, &end_date, p, pp)
        .map_err(|e| e.to_string())?;
    Ok(PaginatedResult {
        items,
        total,
        page: p,
        per_page: pp,
    })
}

#[tauri::command]
pub fn create_expense(state: State<DbState>, input: CreateExpenseInput) -> Result<Expense, String> {
    let mut conn = conn(&state);
    let new = NewExpense {
        date: input.date,
        category: input.category,
        description: Some(input.description),
        amount: input.amount,
    };
    services::expense_service::create(&mut conn, &new).map_err(|e| e.to_string())
}

// ── Dashboard ──

#[tauri::command]
pub fn get_dashboard_data(state: State<DbState>, date: String) -> Result<DashboardData, String> {
    let mut conn = conn(&state);
    let stats =
        services::dashboard_service::daily_stats(&mut conn, &date).map_err(|e| e.to_string())?;
    let cost_total =
        services::sale_service::daily_cost_total(&mut conn, &date).map_err(|e| e.to_string())?;
    let profit =
        services::dashboard_service::profit(stats.sales_total, cost_total, stats.expenses_total);
    Ok(DashboardData {
        sales_total: stats.sales_total,
        expenses_total: stats.expenses_total,
        transaction_count: stats.transaction_count,
        cost_total,
        profit,
    })
}

#[tauri::command]
pub fn get_low_stock_products(
    state: State<DbState>,
    threshold: Option<i32>,
) -> Result<Vec<Product>, String> {
    let mut conn = conn(&state);
    services::dashboard_service::low_stock_products(&mut conn, threshold.unwrap_or(10))
        .map_err(|e| e.to_string())
}

// ── Suppliers ──

#[tauri::command]
pub fn list_suppliers(state: State<DbState>) -> Result<Vec<Supplier>, String> {
    let mut conn = conn(&state);
    services::supplier_service::find_all(&mut conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_supplier(state: State<DbState>, input: SupplierInput) -> Result<Supplier, String> {
    let mut conn = conn(&state);
    let new = NewSupplier {
        name: input.name,
        phone: input.phone,
        notes: input.notes,
    };
    services::supplier_service::create(&mut conn, &new).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_supplier(state: State<DbState>, id: i32, input: SupplierInput) -> Result<(), String> {
    let mut conn = conn(&state);
    let new = NewSupplier {
        name: input.name,
        phone: input.phone,
        notes: input.notes,
    };
    services::supplier_service::update(&mut conn, id, &new).map_err(|e| e.to_string())
}

// ── Expense Categories ──

#[tauri::command]
pub fn list_expense_categories(state: State<DbState>) -> Result<Vec<ExpenseCategory>, String> {
    let mut conn = conn(&state);
    services::expense_category_service::find_all(&mut conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_supplier(state: State<DbState>, id: i32) -> Result<bool, String> {
    let mut conn = conn(&state);
    services::supplier_service::delete(&mut conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_expense(state: State<DbState>, id: i32) -> Result<bool, String> {
    let mut conn = conn(&state);
    services::expense_service::delete(&mut conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_purchase_order(state: State<DbState>, id: i32) -> Result<bool, String> {
    let mut conn = conn(&state);
    services::purchase_order_service::delete(&mut conn, id).map_err(|e| e.to_string())
}

// ── DB Info ──

#[tauri::command]
pub fn get_db_info(state: State<DbState>) -> Result<DbInfo, String> {
    let mut conn = conn(&state);
    let count = services::product_service::count_all(&mut conn).map_err(|e| e.to_string())?;
    Ok(DbInfo {
        db_path: state.db_path.clone(),
        product_count: count,
    })
}
