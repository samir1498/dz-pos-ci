use crate::models::*;
use crate::repos::product_repo;
use diesel::prelude::*;
use diesel::SqliteConnection;

pub fn create(conn: &mut SqliteConnection, product: &NewProduct) -> QueryResult<Product> {
    product_repo::insert_product(conn, product)
}

pub fn find_all(conn: &mut SqliteConnection) -> QueryResult<Vec<Product>> {
    product_repo::find_all_products(conn)
}

pub fn find_paginated(
    conn: &mut SqliteConnection,
    page: i64,
    per_page: i64,
) -> QueryResult<Vec<Product>> {
    product_repo::find_products_paginated(conn, page, per_page)
}

pub fn count_all(conn: &mut SqliteConnection) -> QueryResult<i64> {
    product_repo::count_products(conn)
}

pub fn find_by_id(conn: &mut SqliteConnection, product_id: i32) -> QueryResult<Option<Product>> {
    product_repo::find_product_by_id(conn, product_id)
}

pub fn find_by_barcode(conn: &mut SqliteConnection, barcode: &str) -> QueryResult<Option<Product>> {
    product_repo::find_product_by_barcode(conn, barcode)
}

pub fn update(
    conn: &mut SqliteConnection,
    product_id: i32,
    product: &NewProduct,
) -> QueryResult<()> {
    product_repo::update_product(conn, product_id, product)
}

pub fn delete(conn: &mut SqliteConnection, product_id: i32) -> QueryResult<bool> {
    product_repo::delete_product(conn, product_id)
}
