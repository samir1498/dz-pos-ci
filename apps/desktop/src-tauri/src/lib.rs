mod commands;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // DB path same as egui desktop
            let db_path = db_path();
            if let Some(parent) = std::path::Path::new(&db_path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            dzpos_backup::backup_if_needed(&db_path);
            let _conn = dzpos_core::db::open(&db_path).expect("Failed to open database");
            app.manage(commands::DbState { db_path });
            Ok(())
        })
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::get_db_path,
            // Products
            commands::list_products,
            commands::get_product,
            commands::create_product,
            commands::update_product,
            commands::delete_product,
            // Sales
            commands::record_sale,
            commands::list_sales,
            // Purchase Orders
            commands::list_purchase_orders,
            commands::create_purchase_order,
            commands::receive_purchase_order,
            // Expenses
            commands::list_expenses,
            commands::create_expense,
            // Dashboard
            commands::get_dashboard_data,
            commands::get_low_stock_products,
            // Suppliers
            commands::list_suppliers,
            commands::create_supplier,
            commands::update_supplier,
            commands::delete_supplier,
            commands::delete_expense,
            commands::delete_purchase_order,
            // Expense Categories
            commands::list_expense_categories,
            // Backup
            commands::get_db_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn db_path() -> String {
    let base = dirs::data_dir().expect("Cannot find data directory");
    base.join("dzpos")
        .join("dzpos.db")
        .to_string_lossy()
        .to_string()
}
