// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The store block: the seller a ticket prints (features.md §3).

use dzpos_core::error::CoreError;
use dzpos_core::models::shop::StoreBlock;
use dzpos_core::services::shops;

const SHOP: i32 = 1;
/// The owner the first migration seeds. A service takes whoever acted as an
/// argument; the API reads that from the session, and a test says it here.
const OWNER: i32 = 1;

mod common;

use common::open_temp;

fn full_block() -> StoreBlock {
    StoreBlock {
        name: "Superette El Baraka".to_string(),
        rc: Some("16/00-1234567 B 20".to_string()),
        nif: Some("000016001234567".to_string()),
        nis: Some("000116001234567".to_string()),
        ai: Some("16012345678".to_string()),
        address: Some("12 rue Didouche Mourad, Alger".to_string()),
        phone: Some("0555 12 34 56".to_string()),
    }
}

#[test]
fn the_seeded_shop_has_a_name_and_no_identifiers() {
    let (_dir, mut conn) = open_temp();
    let shop = shops::get(&mut conn, SHOP).unwrap();
    assert_eq!(shop.id, SHOP);
    assert_eq!(shop.name, "Mon magasin");
    assert_eq!(
        (shop.rc, shop.nif, shop.nis, shop.ai),
        (None, None, None, None)
    );
    assert_eq!((shop.address, shop.phone), (None, None));
}

#[test]
fn an_update_stores_every_field_of_the_seller_block() {
    let (_dir, mut conn) = open_temp();
    let after = shops::update_store(&mut conn, SHOP, OWNER, full_block()).unwrap();
    let want = full_block();
    assert_eq!(after.name, want.name);
    assert_eq!(after.rc, want.rc);
    assert_eq!(after.nif, want.nif);
    assert_eq!(after.nis, want.nis);
    assert_eq!(after.ai, want.ai);
    assert_eq!(after.address, want.address);
    assert_eq!(after.phone, want.phone);
    assert_eq!(
        shops::get(&mut conn, SHOP).unwrap(),
        after,
        "the read agrees with the write"
    );
}

#[test]
fn a_cleared_identifier_is_stored_as_nothing_not_as_a_space() {
    let (_dir, mut conn) = open_temp();
    shops::update_store(&mut conn, SHOP, OWNER, full_block()).unwrap();
    let cleared = StoreBlock {
        rc: None,
        nif: Some("   ".to_string()),
        ..full_block()
    };
    let after = shops::update_store(&mut conn, SHOP, OWNER, cleared).unwrap();
    assert_eq!(after.rc, None, "a None over a stored value clears it");
    assert_eq!(after.nif, None, "blank text clears it too");
    assert_eq!(after.nis, full_block().nis, "the fields sent again stay");
}

#[test]
fn values_are_trimmed_before_they_are_stored() {
    let (_dir, mut conn) = open_temp();
    let padded = StoreBlock {
        name: "  Superette El Baraka  ".to_string(),
        phone: Some(" 0555 12 34 56 ".to_string()),
        ..full_block()
    };
    let after = shops::update_store(&mut conn, SHOP, OWNER, padded).unwrap();
    assert_eq!(after.name, "Superette El Baraka");
    assert_eq!(after.phone.as_deref(), Some("0555 12 34 56"));
}

#[test]
fn a_blank_name_is_refused_and_nothing_changes() {
    let (_dir, mut conn) = open_temp();
    for name in ["", "   "] {
        let err = shops::update_store(
            &mut conn,
            SHOP,
            OWNER,
            StoreBlock {
                name: name.to_string(),
                ..full_block()
            },
        )
        .unwrap_err();
        assert!(
            matches!(&err, CoreError::Validation { field, .. } if field == "name"),
            "{name:?} gave {err:?}"
        );
    }
    let shop = shops::get(&mut conn, SHOP).unwrap();
    assert_eq!(shop.name, "Mon magasin");
    assert_eq!(shop.rc, None, "the refused write left no field behind");
}

#[test]
fn a_field_longer_than_a_document_can_print_is_refused() {
    let (_dir, mut conn) = open_temp();
    let long = "x".repeat(201);
    let err = shops::update_store(
        &mut conn,
        SHOP,
        OWNER,
        StoreBlock {
            address: Some(long),
            ..full_block()
        },
    )
    .unwrap_err();
    assert!(matches!(&err, CoreError::Validation { field, .. } if field == "address"));
    // Exactly the bound is fine.
    let ok = shops::update_store(
        &mut conn,
        SHOP,
        OWNER,
        StoreBlock {
            address: Some("y".repeat(200)),
            ..full_block()
        },
    )
    .unwrap();
    assert_eq!(ok.address.map(|a| a.len()), Some(200));
}

#[test]
fn another_shop_is_not_found_on_read_or_write() {
    // Rule 3: the process is started for one shop; id 2 does not exist for
    // it whether or not a row with that id is ever created.
    let (_dir, mut conn) = open_temp();
    assert!(matches!(
        shops::get(&mut conn, 2),
        Err(CoreError::NotFound {
            entity: "shop",
            id: 2
        })
    ));
    assert!(matches!(
        shops::update_store(&mut conn, 2, OWNER, full_block()),
        Err(CoreError::NotFound {
            entity: "shop",
            id: 2
        })
    ));
    assert_eq!(
        shops::get(&mut conn, SHOP).unwrap().name,
        "Mon magasin",
        "shop 1 was not touched by the write aimed at shop 2"
    );
}

#[test]
fn a_store_block_change_leaves_an_audit_entry_holding_both_versions() {
    // The seller block is a legal block of every document (décret 05-468
    // art. 2). Who changed the NIF, and to what, is the question the log
    // exists to answer (features.md §5).
    use dzpos_core::services::audit;
    let (_dir, mut conn) = open_temp();
    let before_name = shops::get(&mut conn, SHOP).unwrap().name;
    shops::update_store(&mut conn, SHOP, OWNER, full_block()).unwrap();

    let entries = audit::list(&mut conn, SHOP).unwrap();
    assert_eq!(entries.len(), 1, "the store block change was not logged");
    let entry = &entries[0];
    assert_eq!(entry.action, "update");
    assert_eq!(entry.entity, "shop");
    assert_eq!(entry.entity_id, Some(SHOP));
    assert_eq!(entry.user_id, OWNER);
    let before = entry.before.clone().expect("no before");
    let after = entry.after.clone().expect("no after");
    assert!(
        before.contains(&before_name),
        "the old name is missing: {before}"
    );
    assert!(
        after.contains("000016001234567"),
        "the new NIF is missing: {after}"
    );
}
