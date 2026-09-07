use diesel::prelude::*;
use diesel::SqliteConnection;
use dzpos_core::models::*;
use dzpos_core::schema::products;
use dzpos_core::services::product_service;

pub fn import_products(conn: &mut SqliteConnection, backup_path: &str) -> Result<usize, String> {
    let backup_conn = &mut dzpos_core::db::open(backup_path)
        .map_err(|e| format!("Failed to open backup: {}", e))?;

    let source_products: Vec<Product> = products::table
        .select(Product::as_select())
        .load(backup_conn)
        .map_err(|e| format!("Failed to read backup: {}", e))?;

    let mut imported = 0;
    for p in source_products {
        let new = NewProduct {
            name: p.name,
            barcode: p.barcode,
            cost_price: p.cost_price,
            selling_price: p.selling_price,
            quantity_on_hand: p.quantity_on_hand,
        };
        if product_service::create(conn, &new).is_ok() {
            imported += 1;
        }
    }

    Ok(imported)
}
