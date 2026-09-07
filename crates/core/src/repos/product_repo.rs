use crate::models::*;
use crate::schema::products;
use diesel::prelude::*;
use diesel::SqliteConnection;

pub fn insert_product(conn: &mut SqliteConnection, p: &NewProduct) -> QueryResult<Product> {
    diesel::insert_into(products::table)
        .values(p)
        .returning(Product::as_returning())
        .get_result(conn)
}

pub fn find_all_products(conn: &mut SqliteConnection) -> QueryResult<Vec<Product>> {
    products::table
        .order(products::name.asc())
        .select(Product::as_select())
        .load(conn)
}

pub fn find_products_paginated(
    conn: &mut SqliteConnection,
    page: i64,
    per_page: i64,
) -> QueryResult<Vec<Product>> {
    products::table
        .order(products::name.asc())
        .select(Product::as_select())
        .offset((page - 1).max(0) * per_page)
        .limit(per_page)
        .load(conn)
}

pub fn count_products(conn: &mut SqliteConnection) -> QueryResult<i64> {
    use diesel::dsl::count;
    products::table
        .select(count(products::id))
        .first::<i64>(conn)
}

pub fn find_product_by_id(
    conn: &mut SqliteConnection,
    product_id: i32,
) -> QueryResult<Option<Product>> {
    products::table
        .find(product_id)
        .select(Product::as_select())
        .first(conn)
        .optional()
}

pub fn find_product_by_barcode(
    conn: &mut SqliteConnection,
    barcode: &str,
) -> QueryResult<Option<Product>> {
    products::table
        .filter(products::barcode.eq(barcode))
        .select(Product::as_select())
        .first(conn)
        .optional()
}

pub fn update_product(
    conn: &mut SqliteConnection,
    product_id: i32,
    p: &NewProduct,
) -> QueryResult<()> {
    diesel::update(products::table.find(product_id))
        .set((
            products::name.eq(&p.name),
            products::barcode.eq(&p.barcode),
            products::cost_price.eq(p.cost_price),
            products::selling_price.eq(p.selling_price),
        ))
        .execute(conn)
        .map(|_| ())
}

pub fn delete_product(conn: &mut SqliteConnection, product_id: i32) -> QueryResult<bool> {
    let rows = diesel::delete(products::table.find(product_id)).execute(conn)?;
    Ok(rows > 0)
}

pub fn find_low_stock_products(
    conn: &mut SqliteConnection,
    threshold: i32,
) -> QueryResult<Vec<Product>> {
    products::table
        .filter(products::quantity_on_hand.le(threshold))
        .order(products::quantity_on_hand.asc())
        .select(Product::as_select())
        .load(conn)
}
