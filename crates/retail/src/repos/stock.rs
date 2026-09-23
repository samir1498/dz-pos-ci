//! The only place the stock ledger touches diesel. Every query is scoped by
//! `shop_id` (rule 3).

use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Nullable};
use diesel::sqlite::SqliteConnection;

use std::collections::HashMap;

use crate::error::{CoreError, RetailError};
use crate::models::stock::{
    Counted, MovementKind, StockMovement, StockMovementRow, StockMovementRowWrite,
};
use crate::money::Money;
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

/// What the units of each product cost when they left the shop on this
/// document, read back off the sale movements the document wrote.
///
/// One entry per product and not per line: a sale reads the fiche's cost
/// once and writes it on every line it moves, so two lines of one product
/// left on the same cost. That is asserted rather than assumed. The lowest
/// and the highest cost of each product are read and a product whose two
/// disagree is refused, so the day a line carries a cost of its own this
/// answers an error instead of whichever row the map happened to keep last.
///
/// An empty map is a document that moved no stock; a product absent from it
/// is a line the ledger cannot price, which the caller refuses.
pub fn sale_costs_of_document(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
) -> Result<HashMap<i32, Money>, RetailError> {
    let rows: Vec<(i32, Option<i64>, Option<i64>)> = stock_movements::table
        .filter(stock_movements::shop_id.eq(shop_id))
        .filter(stock_movements::document_id.eq(document_id))
        .filter(stock_movements::kind.eq(MovementKind::Sale))
        .group_by(stock_movements::product_id)
        .select((
            stock_movements::product_id,
            sql::<Nullable<BigInt>>("MIN(unit_cost_centimes)"),
            sql::<Nullable<BigInt>>("MAX(unit_cost_centimes)"),
        ))
        .load(conn)?;
    let mut costs = HashMap::with_capacity(rows.len());
    for (product_id, low, high) in rows {
        // A group SQLite answered has at least one row in it, so neither
        // bound is null; a file that answers otherwise is one this cannot
        // price either, and it takes the same refusal.
        let (Some(low), Some(high)) = (low, high) else {
            return Err(RetailError::UnpricedReversal {
                document_id,
                product_id,
                reason: "its sale movements carry no cost",
            });
        };
        if low != high {
            return Err(RetailError::UnpricedReversal {
                document_id,
                product_id,
                reason: "its sale movements carry two different costs",
            });
        }
        costs.insert(product_id, Money::centimes(low));
    }
    Ok(costs)
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
    // Indexed rather than scanned per product: the recount walks the whole
    // catalogue, and a shop with a few thousand products was doing a few
    // million comparisons to answer a question SQLite had already grouped.
    let by_product: std::collections::HashMap<i32, i64> = sums
        .into_iter()
        .map(|s| (s.product_id, s.total_milli))
        .collect();
    Ok(cached
        .into_iter()
        .map(|(product_id, name, cached_milli)| Counted {
            product_id,
            name,
            cached_milli,
            // A product with no movement at all is not missing from the
            // answer: its ledger explains nothing, which is a sum of zero,
            // and a cache above that is drift like any other.
            ledger_milli: by_product.get(&product_id).copied().unwrap_or(0),
        })
        .collect())
}
