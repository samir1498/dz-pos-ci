//! The only place products touch diesel. Every query is scoped by
//! `shop_id` (rule 3); a function that forgets it is the bug this layer
//! exists to make visible.

use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel::sqlite::SqliteConnection;

use crate::error::{CoreError, RetailError};
use crate::models::product::{Contenance, Product, ProductRow, ProductRowWrite};
use crate::schema::products;

/// A unique-index violation on `(shop_id, barcode)` is the only constraint
/// a caller can trip, so it becomes the domain error rather than a SQL one.
fn map_write(err: DieselError, barcode: Option<&str>) -> RetailError {
    match (&err, barcode) {
        (DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _), Some(code)) => {
            RetailError::DuplicateBarcode(code.to_string())
        }
        _ => CoreError::Query(err).into(),
    }
}

pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<Product>, CoreError> {
    products::table
        .filter(products::shop_id.eq(shop_id))
        .order(products::name.asc())
        .select(ProductRow::as_select())
        .load(conn)?
        .into_iter()
        .map(Product::try_from)
        .collect()
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Product, CoreError> {
    let row = products::table
        .filter(products::shop_id.eq(shop_id))
        .filter(products::id.eq(id))
        .select(ProductRow::as_select())
        .first(conn)
        .optional()?
        .ok_or(CoreError::NotFound {
            entity: "product",
            id,
        })?;
    Product::try_from(row)
}

pub fn insert(
    conn: &mut SqliteConnection,
    write: &ProductRowWrite,
) -> Result<Product, RetailError> {
    let row: ProductRow = diesel::insert_into(products::table)
        .values(write)
        .returning(ProductRow::as_returning())
        .get_result(conn)
        .map_err(|e| map_write(e, write.barcode.as_deref()))?;
    Ok(Product::try_from(row)?)
}

pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    write: &ProductRowWrite,
) -> Result<Product, RetailError> {
    let changed = diesel::update(
        products::table
            .filter(products::shop_id.eq(shop_id))
            .filter(products::id.eq(id)),
    )
    .set(write)
    .execute(conn)
    .map_err(|e| map_write(e, write.barcode.as_deref()))?;
    if changed == 0 {
        return Err(CoreError::NotFound {
            entity: "product",
            id,
        }
        .into());
    }
    get(conn, shop_id, id).map_err(RetailError::from)
}

/// Moves the cost the shop carries the product at, and nothing else. A
/// receipt sets it to what the goods last landed at (`services::purchases`),
/// and the whole-row `update` above would need the rest of the fiche to say
/// it: a caller holding a stale copy would quietly write back a name or a
/// price somebody had changed in between.
pub fn set_cost(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    cost: crate::money::Money,
) -> Result<(), CoreError> {
    let changed = diesel::update(
        products::table
            .filter(products::shop_id.eq(shop_id))
            .filter(products::id.eq(id)),
    )
    .set(products::cost_centimes.eq(cost.as_centimes()))
    .execute(conn)?;
    if changed == 0 {
        return Err(CoreError::NotFound {
            entity: "product",
            id,
        });
    }
    Ok(())
}

/// Writes the pack size, or clears it with `None`. The two columns go
/// together, the way migration 000030's CHECK holds them.
pub fn set_contenance(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    contenance: Option<Contenance>,
) -> Result<(), CoreError> {
    let changed = diesel::update(
        products::table
            .filter(products::shop_id.eq(shop_id))
            .filter(products::id.eq(id)),
    )
    .set((
        products::contenance_milli.eq(contenance.map(Contenance::qty_milli)),
        products::contenance_unit.eq(contenance.map(|c| c.unit().as_str())),
    ))
    .execute(conn)?;
    if changed == 0 {
        return Err(CoreError::NotFound {
            entity: "product",
            id,
        });
    }
    Ok(())
}

/// The product this shop already sells under that barcode, if any. What the
/// Excel import matches a row on: a code the shop knows updates the fiche it
/// belongs to rather than opening a second one beside it (features.md §1).
///
/// `first` rather than a unique read: the column's uniqueness is the file's
/// (migration 1), and a repo that assumed it would panic on a restored file
/// that broke it.
pub fn by_barcode(
    conn: &mut SqliteConnection,
    shop_id: i32,
    barcode: &str,
) -> Result<Option<Product>, CoreError> {
    let row: Option<ProductRow> = products::table
        .filter(products::shop_id.eq(shop_id))
        .filter(products::barcode.eq(barcode))
        .select(ProductRow::as_select())
        .first(conn)
        .optional()?;
    row.map(Product::try_from).transpose()
}

/// Whether this shop already uses that barcode. The auto-numbering asks
/// before it hands a number out, so a number a user typed by hand costs one
/// number rather than a failed insert.
pub fn barcode_exists(
    conn: &mut SqliteConnection,
    shop_id: i32,
    barcode: &str,
) -> Result<bool, CoreError> {
    let found: Option<i32> = products::table
        .filter(products::shop_id.eq(shop_id))
        .filter(products::barcode.eq(barcode))
        .select(products::id)
        .first(conn)
        .optional()?;
    Ok(found.is_some())
}

#[cfg(test)]
#[path = "../../tests/unit/repos_products.rs"]
mod tests;
