use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::sale_idempotency::{SaleIdempotencyRow, SaleIdempotencyWrite};
use crate::schema::sale_idempotency_keys;

pub(crate) fn find(
    conn: &mut SqliteConnection,
    shop_id: i32,
    key: &str,
) -> Result<Option<SaleIdempotencyRow>, CoreError> {
    Ok(sale_idempotency_keys::table
        .filter(sale_idempotency_keys::shop_id.eq(shop_id))
        .filter(sale_idempotency_keys::key.eq(key))
        .select(SaleIdempotencyRow::as_select())
        .first(conn)
        .optional()?)
}

/// Records which document a key rang. Returns false instead of failing when
/// another ring won the race and took the key first: the caller re-reads
/// and answers the original. Any other failure is a real one.
pub(crate) fn record(
    conn: &mut SqliteConnection,
    shop_id: i32,
    key: &str,
    sale_id: i32,
    request_hash: &str,
    now: NaiveDateTime,
) -> Result<bool, CoreError> {
    let write = SaleIdempotencyWrite {
        shop_id,
        key: key.to_string(),
        sale_id,
        request_hash: request_hash.to_string(),
        created_at: now,
    };
    match diesel::insert_into(sale_idempotency_keys::table)
        .values(&write)
        .execute(conn)
    {
        Ok(_) => Ok(true),
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        )) => Ok(false),
        Err(other) => Err(CoreError::Query(other)),
    }
}
