//! The only place the absence blocks touch diesel. Every query is scoped by
//! `shop_id` (rule 3).

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;

use crate::models::absence_block::AbsenceBlock;
use crate::schema::absence_blocks;

pub fn insert(conn: &mut SqliteConnection, row: &AbsenceBlock) -> Result<AbsenceBlock, CoreError> {
    Ok(diesel::insert_into(absence_blocks::table)
        .values(row)
        .returning(AbsenceBlock::as_returning())
        .get_result(conn)?)
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: &str) -> Result<AbsenceBlock, CoreError> {
    absence_blocks::table
        .filter(absence_blocks::shop_id.eq(shop_id))
        .filter(absence_blocks::id.eq(id))
        .select(AbsenceBlock::as_select())
        .first(conn)
        .optional()?
        .ok_or_else(|| CoreError::NotFoundText {
            entity: "absence_block",
            id: id.to_string(),
        })
}

pub fn delete(conn: &mut SqliteConnection, shop_id: i32, id: &str) -> Result<(), CoreError> {
    diesel::delete(
        absence_blocks::table
            .filter(absence_blocks::shop_id.eq(shop_id))
            .filter(absence_blocks::id.eq(id)),
    )
    .execute(conn)?;
    Ok(())
}

/// The blocks of this shop that share a minute with `[from, until)`, in
/// time order. Two half-open periods overlap when each starts before the
/// other ends, so a block ending at `from` is not one of them.
pub fn overlapping(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Vec<AbsenceBlock>, CoreError> {
    Ok(absence_blocks::table
        .filter(absence_blocks::shop_id.eq(shop_id))
        .filter(absence_blocks::starts_at.lt(until))
        .filter(absence_blocks::ends_at.gt(from))
        .order((absence_blocks::starts_at.asc(), absence_blocks::id.asc()))
        .select(AbsenceBlock::as_select())
        .load(conn)?)
}

/// The blocks of this shop not yet over at `now`, in time order.
pub fn ending_after(
    conn: &mut SqliteConnection,
    shop_id: i32,
    now: NaiveDateTime,
) -> Result<Vec<AbsenceBlock>, CoreError> {
    Ok(absence_blocks::table
        .filter(absence_blocks::shop_id.eq(shop_id))
        .filter(absence_blocks::ends_at.gt(now))
        .order((absence_blocks::starts_at.asc(), absence_blocks::id.asc()))
        .select(AbsenceBlock::as_select())
        .load(conn)?)
}
