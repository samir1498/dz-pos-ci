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
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::repos::testdb::{open, SHOP};

    fn at(day: u32) -> chrono::NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 9, day)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap()
    }

    #[test]
    fn a_key_never_set_reads_as_nothing() {
        let (_dir, mut conn) = open();
        assert_eq!(value(&mut conn, SHOP, "theme").unwrap(), None);
    }

    #[test]
    fn writing_twice_leaves_one_row_holding_the_second() {
        let (_dir, mut conn) = open();
        put(&mut conn, SHOP, "theme", "registre", at(10)).unwrap();
        put(&mut conn, SHOP, "theme", "observe", at(11)).unwrap();
        assert_eq!(
            value(&mut conn, SHOP, "theme").unwrap(),
            Some("observe".to_string())
        );
        let rows: i64 = preferences::table
            .filter(preferences::shop_id.eq(SHOP))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(rows, 1, "the second write appended instead of replacing");
    }

    #[test]
    fn clearing_takes_the_row_out_rather_than_blanking_it() {
        let (_dir, mut conn) = open();
        put(&mut conn, SHOP, "theme", "registre", at(10)).unwrap();
        clear(&mut conn, SHOP, "theme").unwrap();
        assert_eq!(value(&mut conn, SHOP, "theme").unwrap(), None);
        // A stored empty string would read as "set to nothing" and the caller
        // could not tell it from "never chosen".
        let rows: i64 = preferences::table
            .filter(preferences::shop_id.eq(SHOP))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(rows, 0);
    }

    #[test]
    fn clearing_a_key_never_set_is_not_an_error() {
        let (_dir, mut conn) = open();
        clear(&mut conn, SHOP, "theme").unwrap();
    }

    #[test]
    fn two_keys_do_not_write_over_each_other() {
        let (_dir, mut conn) = open();
        put(&mut conn, SHOP, "theme", "registre", at(10)).unwrap();
        put(&mut conn, SHOP, "density", "compact", at(10)).unwrap();
        assert_eq!(
            value(&mut conn, SHOP, "theme").unwrap(),
            Some("registre".to_string())
        );
        assert_eq!(
            value(&mut conn, SHOP, "density").unwrap(),
            Some("compact".to_string())
        );
    }
}
