use crate::models::*;
use crate::schema::{products, purchase_order_items, purchase_orders};
use diesel::prelude::*;
use diesel::SqliteConnection;

pub fn insert_purchase_order(
    conn: &mut SqliteConnection,
    po: &NewPurchaseOrder,
) -> QueryResult<PurchaseOrder> {
    diesel::insert_into(purchase_orders::table)
        .values(po)
        .returning(PurchaseOrder::as_returning())
        .get_result(conn)
}

pub fn insert_purchase_order_items(
    conn: &mut SqliteConnection,
    items: &[NewPurchaseOrderItem],
) -> QueryResult<Vec<PurchaseOrderItem>> {
    conn.transaction::<Vec<PurchaseOrderItem>, diesel::result::Error, _>(|conn| {
        let mut result = Vec::with_capacity(items.len());
        for item in items {
            let line_total = item.quantity as f64 * item.unit_cost;
            let item = diesel::insert_into(purchase_order_items::table)
                .values((
                    purchase_order_items::purchase_order_id.eq(item.purchase_order_id),
                    purchase_order_items::product_name.eq(&item.product_name),
                    purchase_order_items::quantity.eq(item.quantity),
                    purchase_order_items::unit_cost.eq(item.unit_cost),
                    purchase_order_items::line_total.eq(line_total),
                ))
                .returning(PurchaseOrderItem::as_returning())
                .get_result(conn)?;
            result.push(item);
        }
        Ok(result)
    })
}

pub fn find_all_purchase_orders(conn: &mut SqliteConnection) -> QueryResult<Vec<PurchaseOrder>> {
    purchase_orders::table
        .order(purchase_orders::created_at.desc())
        .select(PurchaseOrder::as_select())
        .load(conn)
}

pub fn find_purchase_orders_paginated(
    conn: &mut SqliteConnection,
    page: i64,
    per_page: i64,
) -> QueryResult<Vec<PurchaseOrder>> {
    purchase_orders::table
        .order(purchase_orders::created_at.desc())
        .select(PurchaseOrder::as_select())
        .offset((page - 1).max(0) * per_page)
        .limit(per_page)
        .load(conn)
}

pub fn count_purchase_orders(conn: &mut SqliteConnection) -> QueryResult<i64> {
    use diesel::dsl::count;
    purchase_orders::table
        .select(count(purchase_orders::id))
        .first::<i64>(conn)
}

pub fn find_purchase_order_by_id(
    conn: &mut SqliteConnection,
    purchase_order_id: i32,
) -> QueryResult<Option<PurchaseOrder>> {
    purchase_orders::table
        .find(purchase_order_id)
        .select(PurchaseOrder::as_select())
        .first(conn)
        .optional()
}

pub fn find_items_by_purchase_order(
    conn: &mut SqliteConnection,
    purchase_order_id: i32,
) -> QueryResult<Vec<PurchaseOrderItem>> {
    purchase_order_items::table
        .filter(purchase_order_items::purchase_order_id.eq(purchase_order_id))
        .select(PurchaseOrderItem::as_select())
        .load(conn)
}

pub fn receive_purchase_order(
    conn: &mut SqliteConnection,
    purchase_order_id: i32,
) -> QueryResult<()> {
    conn.transaction::<(), diesel::result::Error, _>(|conn| {
        diesel::update(purchase_orders::table.find(purchase_order_id))
            .set(purchase_orders::status.eq("Received"))
            .execute(conn)?;

        let items = find_items_by_purchase_order(conn, purchase_order_id)?;
        for item in &items {
            let existing = products::table
                .filter(products::name.eq(&item.product_name))
                .select(Product::as_select())
                .first(conn)
                .optional()?;

            if existing.is_some() {
                diesel::update(products::table.filter(products::name.eq(&item.product_name)))
                    .set((
                        products::cost_price.eq(item.unit_cost),
                        products::quantity_on_hand.eq(products::quantity_on_hand + item.quantity),
                    ))
                    .execute(conn)?;
            } else {
                diesel::insert_into(products::table)
                    .values((
                        products::name.eq(&item.product_name),
                        products::cost_price.eq(item.unit_cost),
                        products::selling_price.eq(0.0),
                        products::quantity_on_hand.eq(item.quantity),
                    ))
                    .execute(conn)?;
            }
        }
        Ok(())
    })
}

pub fn delete_purchase_order(
    conn: &mut SqliteConnection,
    purchase_order_id: i32,
) -> QueryResult<bool> {
    let rows = diesel::delete(purchase_orders::table.find(purchase_order_id)).execute(conn)?;
    Ok(rows > 0)
}

pub fn purchase_order_count_today(
    conn: &mut SqliteConnection,
    today_prefix: &str,
) -> QueryResult<i64> {
    use diesel::dsl::count;
    purchase_orders::table
        .filter(purchase_orders::purchase_order_number.like(format!("{}%", today_prefix)))
        .select(count(purchase_orders::id))
        .first::<i64>(conn)
}

pub fn purchase_order_supplier_name(
    conn: &mut SqliteConnection,
    supplier_id: Option<i32>,
) -> Option<String> {
    let supplier_id = supplier_id?;
    use crate::schema::suppliers::dsl::*;
    suppliers
        .find(supplier_id)
        .select(crate::schema::suppliers::name)
        .first::<String>(conn)
        .ok()
}

pub fn update_purchase_order_total(
    conn: &mut SqliteConnection,
    purchase_order_id: i32,
    total: f64,
) -> QueryResult<()> {
    diesel::update(purchase_orders::table.find(purchase_order_id))
        .set(purchase_orders::total.eq(total))
        .execute(conn)
        .map(|_| ())
}
