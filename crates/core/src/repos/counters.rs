//! The per-shop number series a shop hands out for itself. One row per
//! `(shop_id, name)`; the number and its advance are one step, so two
//! callers never take the same one.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::schema::counters;

/// The series behind an auto-generated in-store barcode (features.md §1).
pub const IN_STORE_BARCODE: &str = "in_store_barcode";

/// Takes this shop's next number and advances the series. Call it inside a
/// transaction: the caller's row and this advance have to commit together.
///
/// The row is created on first use, so a shop the migration never seeded
/// (a second till paired in M7) starts at 1 rather than failing.
pub fn take_next(conn: &mut SqliteConnection, shop_id: i32, name: &str) -> Result<i64, CoreError> {
    diesel::insert_or_ignore_into(counters::table)
        .values((
            counters::shop_id.eq(shop_id),
            counters::name.eq(name),
            counters::next_value.eq(1_i64),
        ))
        .execute(conn)?;

    let taken: i64 = counters::table
        .filter(counters::shop_id.eq(shop_id))
        .filter(counters::name.eq(name))
        .select(counters::next_value)
        .first(conn)?;
    let after = taken
        .checked_add(1)
        .ok_or_else(|| CoreError::validation(name, "this number series is exhausted"))?;

    diesel::update(
        counters::table
            .filter(counters::shop_id.eq(shop_id))
            .filter(counters::name.eq(name)),
    )
    .set(counters::next_value.eq(after))
    .execute(conn)?;
    Ok(taken)
}
