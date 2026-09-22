//! The only place the shops table is read or written. The id is the scope:
//! a caller started for shop 1 cannot read or write shop 2's row (rule 3).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::shop::{Shop, ShopRow, ShopRowWrite};
use crate::schema::shops;

pub fn get(conn: &mut SqliteConnection, shop_id: i32) -> Result<Shop, CoreError> {
    let row = shops::table
        .filter(shops::id.eq(shop_id))
        .select(ShopRow::as_select())
        .first(conn)
        .optional()?
        .ok_or(CoreError::NotFound {
            entity: "shop",
            id: shop_id,
        })?;
    Ok(Shop::from(row))
}

pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    write: &ShopRowWrite,
) -> Result<Shop, CoreError> {
    let changed = diesel::update(shops::table.filter(shops::id.eq(shop_id)))
        .set(write)
        .execute(conn)?;
    if changed == 0 {
        return Err(CoreError::NotFound {
            entity: "shop",
            id: shop_id,
        });
    }
    get(conn, shop_id)
}
