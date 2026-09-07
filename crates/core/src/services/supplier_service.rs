use crate::models::*;
use crate::repos::supplier_repo;
use diesel::prelude::*;
use diesel::SqliteConnection;

pub fn find_all(conn: &mut SqliteConnection) -> QueryResult<Vec<Supplier>> {
    supplier_repo::find_all_suppliers(conn)
}

pub fn find_by_id(conn: &mut SqliteConnection, supplier_id: i32) -> QueryResult<Option<Supplier>> {
    supplier_repo::find_supplier_by_id(conn, supplier_id)
}

pub fn create(conn: &mut SqliteConnection, supplier: &NewSupplier) -> QueryResult<Supplier> {
    supplier_repo::insert_supplier(conn, supplier)
}

pub fn update(
    conn: &mut SqliteConnection,
    supplier_id: i32,
    supplier: &NewSupplier,
) -> QueryResult<()> {
    supplier_repo::update_supplier(conn, supplier_id, supplier)
}

pub fn delete(conn: &mut SqliteConnection, supplier_id: i32) -> QueryResult<bool> {
    supplier_repo::delete_supplier(conn, supplier_id)
}
