//! Category rules. The shop's list, which is what the add-product form
//! reads to offer a category and the TVA rate that comes with it, and the
//! two questions everything else asks of it.
//!
//! Thin on purpose. Nothing here decides much today, and that is the point:
//! the import, the seed and the product form each reached into
//! `repos::categories` on their own, so the day a category grows a rule,
//! that rule would have had three places to be written and two to be
//! forgotten. One door, and the rule lands behind it once.

use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::category::{Category, CategoryRowWrite};
use crate::money::Bps;
use crate::repos::categories as repo;
use dzpos_kernel::services::audit;

pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<Category>, CoreError> {
    repo::list(conn, shop_id)
}

/// The category this shop already has under that name, if any. Matched on
/// the name because a spreadsheet cell holds a word and never an id, so the
/// comparison is case sensitive the way `categories.name` is.
pub fn by_name(
    conn: &mut SqliteConnection,
    shop_id: i32,
    name: &str,
) -> Result<Option<Category>, CoreError> {
    repo::by_name(conn, shop_id, name)
}

/// The category's default TVA rate in basis points.
///
/// Missing is `NotFound` and not `None`, because every caller asking this
/// already holds a `category_id` it read off a product or a row: no such
/// category for this shop means the id came from somewhere else, and that
/// is the check that keeps a product pointed at another shop's category
/// (features.md §1, rule 3).
pub fn rate_of(
    conn: &mut SqliteConnection,
    shop_id: i32,
    category_id: i32,
) -> Result<i32, CoreError> {
    repo::default_rate_bps(conn, shop_id, category_id)?.ok_or(CoreError::NotFound {
        entity: "category",
        id: category_id,
    })
}

/// A new category, with the audit row that says where it came from.
///
/// `source` is what put it there: the import names its own action, the seed
/// names itself. The two used to write this row each in their own file, one
/// copy of the same four fields apiece.
///
/// The rate arrives as `Bps` and not as a number. `CategoryRowWrite` stores
/// an `i32` and the row is only checked on the way back out, in
/// `TryFrom<CategoryRow>`, so a bad rate reached the database first and
/// failed afterwards. `Bps` cannot hold one, so the conversion happens here
/// once and the callers stop each doing their own.
pub(crate) fn create(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    name: &str,
    default_rate: Bps,
    source: &str,
) -> Result<Category, CoreError> {
    let made = repo::insert(
        conn,
        &CategoryRowWrite {
            shop_id,
            name: name.to_string(),
            default_rate_bps: i32::try_from(default_rate.as_u32())
                .map_err(|_| CoreError::validation("rate_bps", "rate out of range"))?,
        },
    )?;
    audit::record(
        conn,
        shop_id,
        user_id,
        audit::Change {
            action: audit::ACTION_CREATE,
            entity: "category",
            entity_id: Some(made.id),
            before: None,
            after: Some(
                serde_json::json!({
                    "name": made.name,
                    "default_rate_bps": made.default_rate_bps.as_u32(),
                    "source": source,
                })
                .to_string(),
            ),
        },
    )?;
    Ok(made)
}
