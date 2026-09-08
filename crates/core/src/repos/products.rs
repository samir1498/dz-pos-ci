//! The only place products touch diesel. Every query is scoped by
//! `shop_id` (rule 3); a function that forgets it is the bug this layer
//! exists to make visible.

use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::product::{Product, ProductRow, ProductRowWrite};
use crate::schema::{categories, products};

/// A unique-index violation on `(shop_id, barcode)` is the only constraint
/// a caller can trip, so it becomes the domain error rather than a SQL one.
fn map_write(err: DieselError, barcode: Option<&str>) -> CoreError {
    match (&err, barcode) {
        (DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _), Some(code)) => {
            CoreError::DuplicateBarcode(code.to_string())
        }
        _ => CoreError::Query(err),
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

pub fn insert(conn: &mut SqliteConnection, write: &ProductRowWrite) -> Result<Product, CoreError> {
    let row: ProductRow = diesel::insert_into(products::table)
        .values(write)
        .returning(ProductRow::as_returning())
        .get_result(conn)
        .map_err(|e| map_write(e, write.barcode.as_deref()))?;
    Product::try_from(row)
}

pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    write: &ProductRowWrite,
) -> Result<Product, CoreError> {
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
        });
    }
    get(conn, shop_id, id)
}

/// Writes a barcode onto a row that was inserted without one. Split from
/// `insert` because the number is derived from the id the insert assigns.
pub fn set_barcode(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    barcode: &str,
) -> Result<(), CoreError> {
    diesel::update(
        products::table
            .filter(products::shop_id.eq(shop_id))
            .filter(products::id.eq(id)),
    )
    .set(products::barcode.eq(barcode))
    .execute(conn)
    .map_err(|e| map_write(e, Some(barcode)))?;
    Ok(())
}

/// The category's default TVA rate in basis points, or `None` when no
/// category with that id belongs to this shop.
pub fn category_default_rate_bps(
    conn: &mut SqliteConnection,
    shop_id: i32,
    category_id: i32,
) -> Result<Option<i32>, CoreError> {
    Ok(categories::table
        .filter(categories::shop_id.eq(shop_id))
        .filter(categories::id.eq(category_id))
        .select(categories::default_rate_bps)
        .first(conn)
        .optional()?)
}
