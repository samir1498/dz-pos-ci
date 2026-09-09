//! The store block: what a ticket prints as the seller (features.md §3).
//! A name is required because every document carries it; the identifiers
//! are kept as typed, trimmed, and an emptied one is stored as nothing.

use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::shop::{Shop, ShopRowWrite, StoreBlock};
use crate::repos::shops as repo;

/// Longest value any one field keeps. An RC is 20-odd characters and an
/// address a few lines; anything past this is a paste gone wrong, and the
/// ticket template would wrap it into nonsense.
const MAX_FIELD_CHARS: usize = 200;

pub fn get(conn: &mut SqliteConnection, shop_id: i32) -> Result<Shop, CoreError> {
    repo::get(conn, shop_id)
}

pub fn update_store(
    conn: &mut SqliteConnection,
    shop_id: i32,
    block: StoreBlock,
) -> Result<Shop, CoreError> {
    let write = validate(&block)?;
    repo::update(conn, shop_id, &write)
}

fn validate(block: &StoreBlock) -> Result<ShopRowWrite, CoreError> {
    let name = block.name.trim();
    if name.is_empty() {
        return Err(CoreError::validation(
            "name",
            "the shop needs a name; it prints on every ticket",
        ));
    }
    bounded("name", name)?;
    Ok(ShopRowWrite {
        name: name.to_string(),
        rc: optional("rc", block.rc.as_deref())?,
        nif: optional("nif", block.nif.as_deref())?,
        nis: optional("nis", block.nis.as_deref())?,
        ai: optional("ai", block.ai.as_deref())?,
        address: optional("address", block.address.as_deref())?,
        phone: optional("phone", block.phone.as_deref())?,
    })
}

/// Trimmed, and blank becomes `None`: the column is cleared, never left
/// holding a space.
fn optional(field: &str, value: Option<&str>) -> Result<Option<String>, CoreError> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    bounded(field, value)?;
    Ok(Some(value.to_string()))
}

fn bounded(field: &str, value: &str) -> Result<(), CoreError> {
    if value.chars().count() > MAX_FIELD_CHARS {
        return Err(CoreError::validation(
            field,
            "longer than a ticket or a facture can print",
        ));
    }
    Ok(())
}
