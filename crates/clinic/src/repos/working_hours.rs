//! The only place the working week touches diesel. Every query is scoped by
//! `shop_id` (rule 3).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;

use crate::models::working_hours::{WorkingHoursInsert, WorkingHoursRow};
use crate::schema::working_hours;

/// Every range of this shop, by weekday then opening minute.
pub fn all(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<WorkingHoursRow>, CoreError> {
    Ok(working_hours::table
        .filter(working_hours::shop_id.eq(shop_id))
        .order((
            working_hours::weekday.asc(),
            working_hours::opens_minute.asc(),
        ))
        .select(WorkingHoursRow::as_select())
        .load(conn)?)
}

/// Replaces the shop's whole week with `rows`. The caller holds the
/// transaction.
pub fn replace(
    conn: &mut SqliteConnection,
    shop_id: i32,
    rows: &[WorkingHoursInsert],
) -> Result<(), CoreError> {
    diesel::delete(working_hours::table.filter(working_hours::shop_id.eq(shop_id)))
        .execute(conn)?;
    diesel::insert_into(working_hours::table)
        .values(rows)
        .execute(conn)?;
    Ok(())
}
