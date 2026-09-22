//! The only place categories touch diesel. Every query is scoped by
//! `shop_id` (rule 3).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::category::{Category, CategoryRow, CategoryRowWrite};
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

/// One category, written. The only caller is the product import, which
/// opens a category a file names when the shop has none by that name.
pub fn insert(
    conn: &mut SqliteConnection,
    write: &CategoryRowWrite,
) -> Result<Category, CoreError> {
    let row: CategoryRow = diesel::insert_into(categories::table)
        .values(write)
        .returning(CategoryRow::as_returning())
        .get_result(conn)?;
    Category::try_from(row)
}

/// The category this shop already has under that name, if any. The import
/// matches on the name because a spreadsheet cell holds a word and never an
/// id; the comparison is the file's own, so it is case sensitive the way
/// `categories.name` is.
pub fn by_name(
    conn: &mut SqliteConnection,
    shop_id: i32,
    name: &str,
) -> Result<Option<Category>, CoreError> {
    let row: Option<CategoryRow> = categories::table
        .filter(categories::shop_id.eq(shop_id))
        .filter(categories::name.eq(name))
        .select(CategoryRow::as_select())
        .first(conn)
        .optional()?;
    row.map(Category::try_from).transpose()
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
