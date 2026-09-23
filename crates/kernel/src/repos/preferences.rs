//! Shop preferences: one row per (shop, key), written over in place. Every
//! query is scoped by `shop_id` (rule 3).
//!
//! The dated series is next door in `repos::settings`. That one appends
//! because a document has to keep reading the row that was current when it
//! was issued. Nothing reads a preference except the screen showing it now,
//! so this one keeps a single row and no history.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::schema::preferences;

/// The stored value of `key`, or `None` when the shop has never set it.
pub fn value(
    conn: &mut SqliteConnection,
    shop_id: i32,
    key: &str,
) -> Result<Option<String>, CoreError> {
    Ok(preferences::table
        .filter(preferences::shop_id.eq(shop_id))
        .filter(preferences::key.eq(key.to_string()))
        .select(preferences::value)
        .first(conn)
        .optional()?)
}

/// Writes `value` over whatever was there. The UNIQUE (shop_id, key) is what
/// makes the upsert one row rather than a growing list.
pub fn put(
    conn: &mut SqliteConnection,
    shop_id: i32,
    key: &str,
    value: &str,
    at: chrono::NaiveDateTime,
) -> Result<(), CoreError> {
    diesel::insert_into(preferences::table)
        .values((
            preferences::shop_id.eq(shop_id),
            preferences::key.eq(key.to_string()),
            preferences::value.eq(value.to_string()),
            preferences::updated_at.eq(at),
        ))
        .on_conflict((preferences::shop_id, preferences::key))
        .do_update()
        .set((
            preferences::value.eq(value.to_string()),
            preferences::updated_at.eq(at),
        ))
        .execute(conn)?;
    Ok(())
}

/// Forgets the preference, so the caller falls back to its own default. The
/// theme uses this to go back to Comptoir, the default, rather than to the
/// machine's own light or dark preference.
pub fn clear(conn: &mut SqliteConnection, shop_id: i32, key: &str) -> Result<(), CoreError> {
    diesel::delete(
        preferences::table
            .filter(preferences::shop_id.eq(shop_id))
            .filter(preferences::key.eq(key.to_string())),
    )
    .execute(conn)?;
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/repos_preferences.rs"]
mod tests;
