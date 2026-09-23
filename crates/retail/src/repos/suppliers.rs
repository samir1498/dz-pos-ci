//! The only place suppliers touch diesel. Every query is scoped by `shop_id`
//! (rule 3); a function that forgets it is the bug this layer exists to make
//! visible.

use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::supplier::{Supplier, SupplierRow, SupplierRowWrite};
use crate::repos::contains_pattern;
use crate::schema::suppliers;

/// The one constraint a caller can trip here is `UNIQUE (shop_id, name)`, so
/// it becomes the rule features.md §1 states rather than a SQL fault the
/// screen would show as "storage". A `conflict` and not a `validation`: what
/// was typed is well formed and what refuses it is a fiche the shop already
/// has, which is a different sentence from "that name is too long for a
/// ticket". The field is named either way, so the message lands under the
/// input.
fn map_write(err: DieselError) -> CoreError {
    match &err {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => CoreError::conflict(
            "name",
            "this shop already buys from a supplier under that name",
        ),
        _ => CoreError::Query(err),
    }
}

/// The ones the shop still buys from first, then alphabetical inside each
/// group: the list is read by somebody looking for a supplier to order from,
/// and a deactivated fiche is kept for its ledger rather than for that. The
/// name is unique inside the shop, so it settles the order on its own; the id
/// closes it anyway, the way the customers list does.
/// `search` matches a piece of the name or of the phone, the way the customer
/// list's does: it is a substring somebody typed into a box, so the wildcards
/// SQLite reads in a LIKE pattern are escaped into characters to match.
pub fn list(
    conn: &mut SqliteConnection,
    shop_id: i32,
    search: Option<&str>,
) -> Result<Vec<Supplier>, CoreError> {
    let mut query = suppliers::table
        .filter(suppliers::shop_id.eq(shop_id))
        .into_boxed();
    if let Some(text) = search {
        let pattern = contains_pattern(text);
        query = query.filter(
            suppliers::name
                .like(pattern.clone())
                .escape('\\')
                // A fiche with no phone is not a match, and `NULL LIKE …` is
                // NULL, which an OR treats as no match. `assume_not_null`
                // only says so to the type system.
                .or(suppliers::phone
                    .like(pattern)
                    .escape('\\')
                    .assume_not_null()),
        );
    }
    let rows: Vec<SupplierRow> = query
        .order((
            suppliers::active.desc(),
            suppliers::name.asc(),
            suppliers::id.asc(),
        ))
        .select(SupplierRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(Supplier::from).collect())
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Supplier, CoreError> {
    let row: SupplierRow = suppliers::table
        .filter(suppliers::shop_id.eq(shop_id))
        .filter(suppliers::id.eq(id))
        .select(SupplierRow::as_select())
        .first(conn)
        .optional()?
        .ok_or(CoreError::NotFound {
            entity: "supplier",
            id,
        })?;
    Ok(Supplier::from(row))
}

pub fn insert(
    conn: &mut SqliteConnection,
    write: &SupplierRowWrite,
) -> Result<Supplier, CoreError> {
    let row: SupplierRow = diesel::insert_into(suppliers::table)
        .values(write)
        .returning(SupplierRow::as_returning())
        .get_result(conn)
        .map_err(map_write)?;
    Ok(Supplier::from(row))
}

pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    write: &SupplierRowWrite,
) -> Result<Supplier, CoreError> {
    let changed = diesel::update(
        suppliers::table
            .filter(suppliers::shop_id.eq(shop_id))
            .filter(suppliers::id.eq(id)),
    )
    .set(write)
    .execute(conn)
    .map_err(map_write)?;
    if changed == 0 {
        return Err(CoreError::NotFound {
            entity: "supplier",
            id,
        });
    }
    get(conn, shop_id, id)
}

/// Whether the supplier is one of this shop's. A purchase and a ledger row
/// both point at a supplier by id, and the foreign key alone would let
/// another shop's row through.
pub fn belongs_to_shop(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<bool, CoreError> {
    let found: Option<i32> = suppliers::table
        .filter(suppliers::shop_id.eq(shop_id))
        .filter(suppliers::id.eq(supplier_id))
        .select(suppliers::id)
        .first(conn)
        .optional()?;
    Ok(found.is_some())
}

#[cfg(test)]
#[path = "../../tests/unit/repos_suppliers.rs"]
mod tests;
