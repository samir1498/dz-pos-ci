//! The only place the stock ledger touches diesel. Every query is scoped by
//! `shop_id` (rule 3).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::stock::{Counted, StockMovement, StockMovementRow, StockMovementRowWrite};
use crate::schema::{products, stock_movements};

pub fn insert(
    conn: &mut SqliteConnection,
    write: &StockMovementRowWrite,
) -> Result<StockMovement, CoreError> {
    let row: StockMovementRow = diesel::insert_into(stock_movements::table)
        .values(write)
        .returning(StockMovementRow::as_returning())
        .get_result(conn)?;
    Ok(StockMovement::from(row))
}

pub fn list_for_product(
    conn: &mut SqliteConnection,
    shop_id: i32,
    product_id: i32,
) -> Result<Vec<StockMovement>, CoreError> {
    let rows: Vec<StockMovementRow> = stock_movements::table
        .filter(stock_movements::shop_id.eq(shop_id))
        .filter(stock_movements::product_id.eq(product_id))
        .order(stock_movements::id.asc())
        .select(StockMovementRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(StockMovement::from).collect())
}

/// Adds `qty_milli` to the product's cached quantity. The cache and the
/// ledger row are written in the caller's transaction, so a movement never
/// exists without its effect. Zero rows changed means the product is not in
/// this shop.
pub fn add_to_cached_quantity(
    conn: &mut SqliteConnection,
    shop_id: i32,
    product_id: i32,
    qty_milli: i64,
) -> Result<(), CoreError> {
    let changed = diesel::update(
        products::table
            .filter(products::shop_id.eq(shop_id))
            .filter(products::id.eq(product_id)),
    )
    .set(products::qty_on_hand_milli.eq(products::qty_on_hand_milli + qty_milli))
    .execute(conn)?;
    if changed == 0 {
        return Err(CoreError::NotFound {
            entity: "product",
            id: product_id,
        });
    }
    Ok(())
}

/// Writes a cached quantity outright, which only the recount does: every
/// other caller moves the cache by a movement it is writing beside it
/// (`add_to_cached_quantity`). Zero rows changed means the product is not in
/// this shop.
pub fn set_cached_quantity(
    conn: &mut SqliteConnection,
    shop_id: i32,
    product_id: i32,
    qty_milli: i64,
) -> Result<(), CoreError> {
    let changed = diesel::update(
        products::table
            .filter(products::shop_id.eq(shop_id))
            .filter(products::id.eq(product_id)),
    )
    .set(products::qty_on_hand_milli.eq(qty_milli))
    .execute(conn)?;
    if changed == 0 {
        return Err(CoreError::NotFound {
            entity: "product",
            id: product_id,
        });
    }
    Ok(())
}

/// Every product of the shop with its cached quantity and the sum its ledger
/// explains. A product with no movement sums to zero.
pub fn cached_and_ledger(
    conn: &mut SqliteConnection,
    shop_id: i32,
) -> Result<Vec<Counted>, CoreError> {
    let cached: Vec<(i32, String, i64)> = products::table
        .filter(products::shop_id.eq(shop_id))
        .order(products::id.asc())
        .select((products::id, products::name, products::qty_on_hand_milli))
        .load(conn)?;
    // Written out rather than built with the dsl: diesel types SUM over a
    // BigInt column as Nullable<Numeric>, which would put a decimal on a path
    // that only ever holds thousandths of a unit.
    #[derive(diesel::QueryableByName)]
    struct LedgerSum {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        product_id: i32,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        total_milli: i64,
    }
    let sums: Vec<LedgerSum> = diesel::sql_query(
        "SELECT product_id, COALESCE(SUM(qty_milli), 0) AS total_milli \
         FROM stock_movements WHERE shop_id = ? GROUP BY product_id",
    )
    .bind::<diesel::sql_types::Integer, _>(shop_id)
    .load(conn)?;
    Ok(cached
        .into_iter()
        .map(|(product_id, name, cached_milli)| Counted {
            product_id,
            name,
            cached_milli,
            ledger_milli: sums
                .iter()
                .find(|s| s.product_id == product_id)
                .map_or(0, |s| s.total_milli),
        })
        .collect())
}
