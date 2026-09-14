use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::pairing::{
    PairedDeviceRow, PairedDeviceWrite, PairingTokenRow, PairingTokenWrite,
};
use crate::schema::{paired_devices, pairing_tokens};

pub(crate) fn insert_pairing_token(
    conn: &mut SqliteConnection,
    write: &PairingTokenWrite,
) -> Result<PairingTokenRow, CoreError> {
    Ok(diesel::insert_into(pairing_tokens::table)
        .values(write)
        .returning(PairingTokenRow::as_returning())
        .get_result(conn)?)
}

pub(crate) fn pairing_by_hash(
    conn: &mut SqliteConnection,
    shop_id: i32,
    token_hash: &str,
) -> Result<Option<PairingTokenRow>, CoreError> {
    Ok(pairing_tokens::table
        .filter(pairing_tokens::shop_id.eq(shop_id))
        .filter(pairing_tokens::token_hash.eq(token_hash))
        .select(PairingTokenRow::as_select())
        .first(conn)
        .optional()?)
}

pub(crate) fn mark_pairing_used(
    conn: &mut SqliteConnection,
    id: i32,
    now: NaiveDateTime,
) -> Result<(), CoreError> {
    diesel::update(pairing_tokens::table.filter(pairing_tokens::id.eq(id)))
        .set(pairing_tokens::used_at.eq(now))
        .execute(conn)?;
    Ok(())
}

pub(crate) fn insert_device(
    conn: &mut SqliteConnection,
    write: &PairedDeviceWrite,
) -> Result<PairedDeviceRow, CoreError> {
    Ok(diesel::insert_into(paired_devices::table)
        .values(write)
        .returning(PairedDeviceRow::as_returning())
        .get_result(conn)?)
}

#[allow(dead_code)]
pub(crate) fn device_by_hash(
    conn: &mut SqliteConnection,
    shop_id: i32,
    token_hash: &str,
) -> Result<Option<PairedDeviceRow>, CoreError> {
    Ok(paired_devices::table
        .filter(paired_devices::shop_id.eq(shop_id))
        .filter(paired_devices::token_hash.eq(token_hash))
        .select(PairedDeviceRow::as_select())
        .first(conn)
        .optional()?)
}
