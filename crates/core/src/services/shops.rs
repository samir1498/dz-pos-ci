//! The store block: what a ticket prints as the seller (features.md §3).
//! A name is required because every document carries it; the identifiers
//! are kept as typed, trimmed, and an emptied one is stored as nothing.

use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::shop::{Shop, ShopRowWrite, StoreBlock};
use crate::repos::shops as repo;
use crate::services::{audit, bounded_field, optional_field};

pub fn get(conn: &mut SqliteConnection, shop_id: i32) -> Result<Shop, CoreError> {
    repo::get(conn, shop_id)
}

pub fn update_store(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    block: StoreBlock,
) -> Result<Shop, CoreError> {
    let write = validate(&block)?;
    // The audit entry and the change are one transaction: an entry without
    // its change, or a change nobody can trace, is the failure this log
    // exists to prevent (features.md §5).
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id)?;
        let after = repo::update(conn, shop_id, &write)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_UPDATE,
                entity: "shop",
                entity_id: Some(shop_id),
                before: Some(as_json(&before)),
                after: Some(as_json(&after)),
            },
        )?;
        Ok(after)
    })
}

fn as_json(shop: &Shop) -> String {
    serde_json::json!({
        "name": shop.name,
        "rc": shop.rc,
        "nif": shop.nif,
        "nis": shop.nis,
        "ai": shop.ai,
        "address": shop.address,
        "phone": shop.phone,
    })
    .to_string()
}

fn validate(block: &StoreBlock) -> Result<ShopRowWrite, CoreError> {
    let name = block.name.trim();
    if name.is_empty() {
        return Err(CoreError::validation(
            "name",
            "the shop needs a name; it prints on every ticket",
        ));
    }
    bounded_field("name", name)?;
    Ok(ShopRowWrite {
        name: name.to_string(),
        rc: optional_field("rc", block.rc.as_deref())?,
        nif: optional_field("nif", block.nif.as_deref())?,
        nis: optional_field("nis", block.nis.as_deref())?,
        ai: optional_field("ai", block.ai.as_deref())?,
        address: optional_field("address", block.address.as_deref())?,
        phone: optional_field("phone", block.phone.as_deref())?,
    })
}

