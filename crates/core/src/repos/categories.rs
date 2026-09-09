//! The only place categories touch diesel. Every query is scoped by
//! `shop_id` (rule 3).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::category::{Category, CategoryRow};
use crate::schema::categories;

pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<Category>, CoreError> {
    categories::table
        .filter(categories::shop_id.eq(shop_id))
        .order(categories::name.asc())
        .select(CategoryRow::as_select())
        .load(conn)?
        .into_iter()
        .map(Category::try_from)
        .collect()
}

/// The category's default TVA rate in basis points, or `None` when no
/// category with that id belongs to this shop. The `None` is what proves a
/// caller is not reaching into another shop.
pub fn default_rate_bps(
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
