//! The dated settings series. Every query is scoped by `shop_id` (rule 3).

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::schema::settings;

/// The value of `key` that was current at `at`: the latest row whose
/// `valid_from` is at or before that moment. `seq` breaks a tie, so two
/// changes inside one second resolve to the later insert.
pub fn value_as_of(
    conn: &mut SqliteConnection,
    shop_id: i32,
    key: &str,
    at: NaiveDateTime,
) -> Result<Option<String>, CoreError> {
    Ok(settings::table
        .filter(settings::shop_id.eq(shop_id))
        .filter(settings::key.eq(key.to_string()))
        .filter(settings::valid_from.le(at))
        .order((settings::valid_from.desc(), settings::seq.desc()))
        .select(settings::value)
        .first(conn)
        .optional()?)
}

/// Appends a row to the series. A setting is never updated in place: the
/// document issued yesterday has to keep reading yesterday's value.
pub fn append(
    conn: &mut SqliteConnection,
    shop_id: i32,
    key: &str,
    value: &str,
    valid_from: NaiveDateTime,
) -> Result<(), CoreError> {
    diesel::insert_into(settings::table)
        .values((
            settings::shop_id.eq(shop_id),
            settings::key.eq(key.to_string()),
            settings::value.eq(value.to_string()),
            settings::valid_from.eq(valid_from),
        ))
        .execute(conn)?;
    Ok(())
}
