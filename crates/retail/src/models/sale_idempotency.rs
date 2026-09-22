use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::schema::sale_idempotency_keys;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = sale_idempotency_keys)]
pub struct SaleIdempotencyRow {
    pub id: i32,
    pub shop_id: i32,
    pub key: String,
    pub sale_id: i32,
    pub request_hash: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = sale_idempotency_keys)]
pub struct SaleIdempotencyWrite {
    pub shop_id: i32,
    pub key: String,
    pub sale_id: i32,
    pub request_hash: String,
    pub created_at: NaiveDateTime,
}
