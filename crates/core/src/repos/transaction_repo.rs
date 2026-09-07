use crate::models::*;
use crate::schema::{products, transaction_items, transactions};
use diesel::prelude::*;
use diesel::SqliteConnection;

pub fn insert_transaction(
    conn: &mut SqliteConnection,
    t: &NewTransaction,
) -> QueryResult<Transaction> {
    diesel::insert_into(transactions::table)
        .values(t)
        .returning(Transaction::as_returning())
        .get_result(conn)
}

pub fn insert_transaction_items(
    conn: &mut SqliteConnection,
    items: &[NewTransactionItem],
) -> QueryResult<Vec<TransactionItem>> {
    conn.transaction::<Vec<TransactionItem>, diesel::result::Error, _>(|conn| {
        let mut result = Vec::with_capacity(items.len());
        for item in items {
            let line_total = item.quantity as f64 * item.selling_price;
            let ti = diesel::insert_into(transaction_items::table)
                .values((
                    transaction_items::transaction_id.eq(item.transaction_id),
                    transaction_items::product_id.eq(item.product_id),
                    transaction_items::product_name.eq(&item.product_name),
                    transaction_items::quantity.eq(item.quantity),
                    transaction_items::selling_price.eq(item.selling_price),
                    transaction_items::cost_price.eq(item.cost_price),
                    transaction_items::line_total.eq(line_total),
                ))
                .returning(TransactionItem::as_returning())
                .get_result(conn)?;

            let current_stock = products::table
                .find(item.product_id)
                .select(products::quantity_on_hand)
                .first::<i32>(conn)
                .optional()?
                .unwrap_or(0);

            if current_stock < item.quantity {
                return Err(diesel::result::Error::RollbackTransaction);
            }

            diesel::update(products::table.find(item.product_id))
                .set(products::quantity_on_hand.eq(products::quantity_on_hand - item.quantity))
                .execute(conn)?;

            result.push(ti);
        }
        Ok(result)
    })
}

pub fn find_transactions_by_date(
    conn: &mut SqliteConnection,
    date: &str,
) -> QueryResult<Vec<Transaction>> {
    transactions::table
        .filter(transactions::timestamp.like(format!("{}%", date)))
        .order(transactions::timestamp.desc())
        .select(Transaction::as_select())
        .load(conn)
}

pub fn find_transactions_paginated(
    conn: &mut SqliteConnection,
    date: &str,
    page: i64,
    per_page: i64,
) -> QueryResult<Vec<Transaction>> {
    transactions::table
        .filter(transactions::timestamp.like(format!("{}%", date)))
        .order(transactions::timestamp.desc())
        .select(Transaction::as_select())
        .offset((page - 1).max(0) * per_page)
        .limit(per_page)
        .load(conn)
}

pub fn count_transactions_by_date(conn: &mut SqliteConnection, date: &str) -> QueryResult<i64> {
    use diesel::dsl::count;
    transactions::table
        .filter(transactions::timestamp.like(format!("{}%", date)))
        .select(count(transactions::id))
        .first::<i64>(conn)
}

pub fn find_items_by_transaction(
    conn: &mut SqliteConnection,
    transaction_id: i32,
) -> QueryResult<Vec<TransactionItem>> {
    transaction_items::table
        .filter(transaction_items::transaction_id.eq(transaction_id))
        .select(TransactionItem::as_select())
        .load(conn)
}

pub fn daily_sales_total(conn: &mut SqliteConnection, date: &str) -> QueryResult<f64> {
    use diesel::dsl::sum;
    transactions::table
        .filter(transactions::timestamp.like(format!("{}%", date)))
        .select(sum(transactions::total))
        .first(conn)
        .map(|v: Option<f64>| v.unwrap_or(0.0))
}

pub fn daily_cost_total(conn: &mut SqliteConnection, date: &str) -> QueryResult<f64> {
    let items = transaction_items::table
        .inner_join(transactions::table)
        .filter(transactions::timestamp.like(format!("{}%", date)))
        .select((transaction_items::cost_price, transaction_items::quantity))
        .load::<(f64, i32)>(conn)?;

    Ok(items.iter().map(|(price, qty)| price * *qty as f64).sum())
}

pub fn transaction_count_by_date(conn: &mut SqliteConnection, date: &str) -> QueryResult<i64> {
    use diesel::dsl::count;
    transactions::table
        .filter(transactions::timestamp.like(format!("{}%", date)))
        .select(count(transactions::id))
        .first::<i64>(conn)
}

pub fn update_transaction_total(
    conn: &mut SqliteConnection,
    transaction_id: i32,
    total: f64,
) -> QueryResult<()> {
    diesel::update(transactions::table.find(transaction_id))
        .set(transactions::total.eq(total))
        .execute(conn)
        .map(|_| ())
}
