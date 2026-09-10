//! The only place customers touch diesel. Every query is scoped by `shop_id`
//! (rule 3); a function that forgets it is the bug this layer exists to make
//! visible.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::customer::{Customer, CustomerRow, CustomerRowWrite};
use crate::repos::contains_pattern;
use crate::schema::customers;

/// The ones the shop still deals with first, then alphabetical inside each
/// group: the list is read by somebody looking for a customer to serve, and a
/// deactivated fiche is kept for its ledger rather than for that. Two
/// customers may share a name, so the id breaks the tie and the order is
/// stable between two calls.
///
/// `search` matches a piece of the name or of the phone. It is a substring
/// somebody typed into a box, so the wildcards SQLite reads in a LIKE pattern
/// are escaped into characters to match: a `%` typed by accident used to
/// answer the whole list.
pub fn list(
    conn: &mut SqliteConnection,
    shop_id: i32,
    search: Option<&str>,
) -> Result<Vec<Customer>, CoreError> {
    let mut query = customers::table
        .filter(customers::shop_id.eq(shop_id))
        .into_boxed();
    if let Some(text) = search {
        let pattern = contains_pattern(text);
        query = query.filter(
            customers::name
                .like(pattern.clone())
                .escape('\\')
                // A fiche with no phone is not a match, and `NULL LIKE …` is
                // NULL, which an OR treats as no match. `assume_not_null`
                // only says so to the type system.
                .or(customers::phone
                    .like(pattern)
                    .escape('\\')
                    .assume_not_null()),
        );
    }
    let rows: Vec<CustomerRow> = query
        .order((
            customers::active.desc(),
            customers::name.asc(),
            customers::id.asc(),
        ))
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
