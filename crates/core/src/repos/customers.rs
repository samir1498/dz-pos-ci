//! The only place customers touch diesel. Every query is scoped by `shop_id`
//! (rule 3); a function that forgets it is the bug this layer exists to make
//! visible.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::customer::{Customer, CustomerRow, CustomerRowWrite};
use crate::schema::customers;

/// Alphabetical, the order the customer list and the picker read in. Two
/// customers may share a name, so the id breaks the tie and the order is
/// stable between two calls.
pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<Customer>, CoreError> {
    let rows: Vec<CustomerRow> = customers::table
        .filter(customers::shop_id.eq(shop_id))
        .order((customers::name.asc(), customers::id.asc()))
        .select(CustomerRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(Customer::from).collect())
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Customer, CoreError> {
    let row: CustomerRow = customers::table
        .filter(customers::shop_id.eq(shop_id))
        .filter(customers::id.eq(id))
        .select(CustomerRow::as_select())
        .first(conn)
        .optional()?
        .ok_or(CoreError::NotFound {
            entity: "customer",
            id,
        })?;
    Ok(Customer::from(row))
}

pub fn insert(
    conn: &mut SqliteConnection,
    write: &CustomerRowWrite,
) -> Result<Customer, CoreError> {
    let row: CustomerRow = diesel::insert_into(customers::table)
        .values(write)
        .returning(CustomerRow::as_returning())
        .get_result(conn)?;
    Ok(Customer::from(row))
}

pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    write: &CustomerRowWrite,
) -> Result<Customer, CoreError> {
    let changed = diesel::update(
        customers::table
            .filter(customers::shop_id.eq(shop_id))
            .filter(customers::id.eq(id)),
    )
    .set(write)
    .execute(conn)?;
    if changed == 0 {
        return Err(CoreError::NotFound {
            entity: "customer",
            id,
        });
    }
    get(conn, shop_id, id)
}

/// Whether the customer is one of this shop's. A document and a ledger row
/// both point at a customer by id, and the foreign key alone would let
/// another shop's row through.
pub fn belongs_to_shop(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<bool, CoreError> {
    let found: Option<i32> = customers::table
        .filter(customers::shop_id.eq(shop_id))
        .filter(customers::id.eq(customer_id))
        .select(customers::id)
        .first(conn)
        .optional()?;
    Ok(found.is_some())
}
