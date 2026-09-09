//! Category rules. For now the shop's list, which is what the add-product
//! form reads to offer a category and the TVA rate that comes with it.

use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::category::Category;
use crate::repos::categories as repo;

pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<Category>, CoreError> {
    repo::list(conn, shop_id)
}
