//! The stock ledger (features.md §1). Every change to a quantity on hand is
//! a row here, and `products.qty_on_hand_milli` is a cache of the sum. The
//! ledger is the truth; `rederive` is what proves the cache still matches it.

use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::stock::{Drift, Movement, StockMovement, StockMovementRowWrite};
use crate::repos::stock as repo;

/// Writes one movement and moves the product's cached quantity by the same
/// amount. Call it inside the caller's transaction: the row and the cache
/// have to commit together, and a sale writes one movement per line.
///
/// Stock may end up below zero. A shop's count is often wrong before its
/// first inventory, and refusing the sale would stop the till over a number
/// nobody typed in.
pub fn record(
    conn: &mut SqliteConnection,
    shop_id: i32,
    movement: &Movement,
) -> Result<StockMovement, CoreError> {
    if movement.unit_cost.is_negative() {
        return Err(CoreError::validation(
            "unit_cost_centimes",
            "a unit cost cannot be negative",
        ));
    }
    // The cache update carries the shop scope, so a product of another shop
    // is reported before the ledger row exists.
    repo::add_to_cached_quantity(conn, shop_id, movement.product_id, movement.qty_milli)?;
    repo::insert(
        conn,
        &StockMovementRowWrite {
            shop_id,
            product_id: movement.product_id,
            kind: movement.kind,
            qty_milli: movement.qty_milli,
            unit_cost_centimes: movement.unit_cost.as_centimes(),
            document_id: movement.document_id,
            user_id: movement.user_id,
        },
    )
}

pub fn list_for_product(
    conn: &mut SqliteConnection,
    shop_id: i32,
    product_id: i32,
) -> Result<Vec<StockMovement>, CoreError> {
    repo::list_for_product(conn, shop_id, product_id)
}

/// Every product whose cached quantity and ledger sum disagree. It reports
/// and never repairs: the difference is what a shop owner has to look at.
pub fn rederive(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<Drift>, CoreError> {
    Ok(repo::cached_and_ledger(conn, shop_id)?
        .into_iter()
        .filter(|(_, cached, ledger)| cached != ledger)
        .map(|(product_id, cached_milli, ledger_milli)| Drift {
            product_id,
            cached_milli,
            ledger_milli,
        })
        .collect())
}
