// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Every service path against a real temp SQLite file. The service is the
//! only door: nothing here touches diesel.

use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::money::{Bps, Money};
use dzpos_core::services::products;

const SHOP: i32 = 1;
const SEEDED_CATEGORY: i32 = 1;

fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let conn = dzpos_core::db::open(&path).unwrap();
    (dir, conn)
}

fn draft(name: &str) -> NewProduct {
    NewProduct {
        name: name.to_string(),
        barcode: None,
        category_id: Some(SEEDED_CATEGORY),
        unit: Unit::Piece,
        cost: Money::centimes(820),
        selling: Money::centimes(920),
        wholesale: None,
        qty_on_hand_milli: 24_000,
        low_stock_at_milli: 10_000,
        rate_bps: None,
        active: true,
    }
}

#[test]
fn a_new_database_lists_no_products() {
    let (_dir, mut conn) = open_temp();
    assert!(products::list(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn create_then_list_and_get_round_trip() {
    let (_dir, mut conn) = open_temp();
    let made = products::create(&mut conn, SHOP, draft("Huile Elio 5L")).unwrap();
    assert_eq!(made.shop_id, SHOP);
    assert_eq!(made.name, "Huile Elio 5L");
    assert_eq!(made.cost, Money::centimes(820));
    assert_eq!(made.selling, Money::centimes(920));
    assert_eq!(made.unit, Unit::Piece);

    let listed = products::list(&mut conn, SHOP).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, made.id);

    let fetched = products::get(&mut conn, SHOP, made.id).unwrap();
    assert_eq!(fetched.selling, Money::centimes(920));
}

#[test]
fn money_survives_the_database_as_centimes() {
    // Rule 6: the value read back is the same integer, never a float.
    let (_dir, mut conn) = open_temp();
    let mut d = draft("Gros montant");
    d.cost = Money::centimes(9_007_199_254_740_993);
    d.selling = Money::centimes(9_007_199_254_740_995);
    d.wholesale = Some(Money::centimes(1));
    let made = products::create(&mut conn, SHOP, d).unwrap();
    let back = products::get(&mut conn, SHOP, made.id).unwrap();
    assert_eq!(back.cost.as_centimes(), 9_007_199_254_740_993);
    assert_eq!(back.selling.as_centimes(), 9_007_199_254_740_995);
    assert_eq!(back.wholesale, Some(Money::centimes(1)));
}

#[test]
fn a_blank_barcode_is_auto_numbered_and_unique() {
    // features.md §1: barcode unique, optional, auto-generated numeric if blank.
    let (_dir, mut conn) = open_temp();
    let a = products::create(&mut conn, SHOP, draft("A")).unwrap();
    let b = products::create(&mut conn, SHOP, draft("B")).unwrap();
    let (ba, bb) = (a.barcode.unwrap(), b.barcode.unwrap());
    assert_ne!(ba, bb);
    for code in [&ba, &bb] {
        assert_eq!(code.len(), 13, "not an EAN-13: {code}");
        assert!(
            code.chars().all(|c| c.is_ascii_digit()),
            "not numeric: {code}"
        );
    }
    assert!(
        ba.starts_with('2'),
        "in-store codes use the restricted prefix"
    );
}

#[test]
fn whitespace_only_barcode_counts_as_blank() {
    let (_dir, mut conn) = open_temp();
    let mut d = draft("A");
    d.barcode = Some("   ".to_string());
    let made = products::create(&mut conn, SHOP, d).unwrap();
    assert_eq!(made.barcode.map(|b| b.len()), Some(13));
}

#[test]
fn a_given_barcode_is_kept_verbatim() {
    let (_dir, mut conn) = open_temp();
    let mut d = draft("A");
    d.barcode = Some(" 6130001000018 ".to_string());
    let made = products::create(&mut conn, SHOP, d).unwrap();
    assert_eq!(made.barcode.as_deref(), Some("6130001000018"));
}

#[test]
fn a_duplicate_barcode_in_the_same_shop_is_rejected() {
    let (_dir, mut conn) = open_temp();
    let mut d = draft("A");
    d.barcode = Some("6130001000018".to_string());
    products::create(&mut conn, SHOP, d).unwrap();

    let mut e = draft("B");
    e.barcode = Some("6130001000018".to_string());
    match products::create(&mut conn, SHOP, e) {
        Err(CoreError::DuplicateBarcode(code)) => assert_eq!(code, "6130001000018"),
        other => panic!("expected DuplicateBarcode, got {other:?}"),
    }
}

#[test]
fn an_empty_name_is_rejected() {
    let (_dir, mut conn) = open_temp();
    let mut d = draft("  ");
    d.barcode = None;
    match products::create(&mut conn, SHOP, d) {
        Err(CoreError::Validation { field, .. }) => assert_eq!(field, "name"),
        other => panic!("expected a name validation error, got {other:?}"),
    }
}

#[test]
fn a_negative_selling_price_is_rejected() {
    let (_dir, mut conn) = open_temp();
    let mut d = draft("A");
    d.selling = Money::centimes(-1);
    match products::create(&mut conn, SHOP, d) {
        Err(CoreError::Validation { field, .. }) => assert_eq!(field, "selling_centimes"),
        other => panic!("expected a selling validation error, got {other:?}"),
    }
}

#[test]
fn a_negative_cost_is_rejected() {
    let (_dir, mut conn) = open_temp();
    let mut d = draft("A");
    d.cost = Money::centimes(-1);
    match products::create(&mut conn, SHOP, d) {
        Err(CoreError::Validation { field, .. }) => assert_eq!(field, "cost_centimes"),
        other => panic!("expected a cost validation error, got {other:?}"),
    }
}

#[test]
fn a_zero_selling_price_is_allowed() {
    let (_dir, mut conn) = open_temp();
    let mut d = draft("Échantillon");
    d.selling = Money::ZERO;
    assert_eq!(
        products::create(&mut conn, SHOP, d).unwrap().selling,
        Money::ZERO
    );
}

#[test]
fn the_rate_defaults_from_the_category() {
    // The seeded category "Général" carries 1900 bps (features.md, TVA rates).
    let (_dir, mut conn) = open_temp();
    let made = products::create(&mut conn, SHOP, draft("A")).unwrap();
    assert_eq!(made.rate_bps, Bps::new(1900).unwrap());
}

#[test]
fn an_explicit_rate_beats_the_category_default() {
    let (_dir, mut conn) = open_temp();
    let mut d = draft("Farine");
    d.rate_bps = Some(Bps::new(900).unwrap());
    assert_eq!(
        products::create(&mut conn, SHOP, d).unwrap().rate_bps,
        Bps::new(900).unwrap()
    );
}

#[test]
fn without_a_category_the_rate_must_be_given() {
    let (_dir, mut conn) = open_temp();
    let mut d = draft("A");
    d.category_id = None;
    match products::create(&mut conn, SHOP, d) {
        Err(CoreError::Validation { field, .. }) => assert_eq!(field, "rate_bps"),
        other => panic!("expected a rate validation error, got {other:?}"),
    }

    let (_dir2, mut conn2) = open_temp();
    let mut e = draft("A");
    e.category_id = None;
    e.rate_bps = Some(Bps::new(0).unwrap());
    assert_eq!(
        products::create(&mut conn2, SHOP, e).unwrap().rate_bps,
        Bps::new(0).unwrap()
    );
}

#[test]
fn a_category_from_another_shop_is_rejected() {
    let (_dir, mut conn) = open_temp();
    let mut d = draft("A");
    d.category_id = Some(4242);
    match products::create(&mut conn, SHOP, d) {
        Err(CoreError::NotFound { entity, .. }) => assert_eq!(entity, "category"),
        other => panic!("expected a category NotFound, got {other:?}"),
    }
}

#[test]
fn get_and_list_are_scoped_by_shop() {
    // Rule 3: every query is scoped by shop_id, so another shop sees nothing.
    let (_dir, mut conn) = open_temp();
    let made = products::create(&mut conn, SHOP, draft("A")).unwrap();
    assert!(products::list(&mut conn, 2).unwrap().is_empty());
    match products::get(&mut conn, 2, made.id) {
        Err(CoreError::NotFound { entity, .. }) => assert_eq!(entity, "product"),
        other => panic!("expected a product NotFound, got {other:?}"),
    }
}

#[test]
fn get_on_a_missing_id_is_not_found() {
    let (_dir, mut conn) = open_temp();
    match products::get(&mut conn, SHOP, 999) {
        Err(CoreError::NotFound { entity, id }) => {
            assert_eq!(entity, "product");
            assert_eq!(id, 999);
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn update_changes_the_row_and_keeps_the_id() {
    let (_dir, mut conn) = open_temp();
    let made = products::create(&mut conn, SHOP, draft("A")).unwrap();
    let mut d = draft("A renommé");
    d.selling = Money::centimes(1_050);
    d.unit = Unit::Kg;
    d.active = false;
    let after = products::update(&mut conn, SHOP, made.id, d).unwrap();
    assert_eq!(after.id, made.id);
    assert_eq!(after.name, "A renommé");
    assert_eq!(after.selling, Money::centimes(1_050));
    assert_eq!(after.unit, Unit::Kg);
    assert!(!after.active);
    assert_eq!(products::list(&mut conn, SHOP).unwrap().len(), 1);
}

#[test]
fn update_keeps_the_existing_barcode_when_none_is_given() {
    let (_dir, mut conn) = open_temp();
    let made = products::create(&mut conn, SHOP, draft("A")).unwrap();
    let after = products::update(&mut conn, SHOP, made.id, draft("A")).unwrap();
    assert_eq!(after.barcode, made.barcode);
}

#[test]
fn update_onto_another_products_barcode_is_rejected() {
    let (_dir, mut conn) = open_temp();
    let a = products::create(&mut conn, SHOP, draft("A")).unwrap();
    let b = products::create(&mut conn, SHOP, draft("B")).unwrap();
    let mut d = draft("B");
    d.barcode = a.barcode.clone();
    match products::update(&mut conn, SHOP, b.id, d) {
        Err(CoreError::DuplicateBarcode(_)) => {}
        other => panic!("expected DuplicateBarcode, got {other:?}"),
    }
}

#[test]
fn update_is_scoped_by_shop() {
    let (_dir, mut conn) = open_temp();
    let made = products::create(&mut conn, SHOP, draft("A")).unwrap();
    match products::update(&mut conn, 2, made.id, draft("stolen")) {
        Err(CoreError::NotFound { entity, .. }) => assert_eq!(entity, "product"),
        other => panic!("expected NotFound, got {other:?}"),
    }
    assert_eq!(products::get(&mut conn, SHOP, made.id).unwrap().name, "A");
}

#[test]
fn every_error_carries_a_stable_code_for_the_ui() {
    // architecture.md, error policy: the UI never shows a Rust or SQL message.
    let (_dir, mut conn) = open_temp();
    let mut d = draft(" ");
    d.barcode = None;
    let err = products::create(&mut conn, SHOP, d).unwrap_err();
    assert_eq!(err.code(), "validation");
    assert_eq!(
        products::get(&mut conn, SHOP, 1).unwrap_err().code(),
        "not_found"
    );
}

#[test]
fn a_rate_above_one_whole_never_panics_on_the_way_out() {
    // Bps refuses a rate above 10 000 bps, so a row written by hand (an old
    // import, a repaired database) must come back as an error, not a panic.
    use diesel::prelude::*;
    let (_dir, mut conn) = open_temp();
    let made = products::create(&mut conn, SHOP, draft("A")).unwrap();
    diesel::sql_query(format!(
        "UPDATE products SET rate_bps = 190000 WHERE id = {}",
        made.id
    ))
    .execute(&mut conn)
    .unwrap();
    match products::get(&mut conn, SHOP, made.id) {
        Err(CoreError::Money(_)) => {}
        other => panic!("expected a money error, got {other:?}"),
    }
}

#[test]
fn units_round_trip_through_the_database() {
    let (_dir, mut conn) = open_temp();
    for (i, unit) in [Unit::Piece, Unit::Kg, Unit::Litre, Unit::Box]
        .into_iter()
        .enumerate()
    {
        let mut d = draft(&format!("p{i}"));
        d.unit = unit;
        let made = products::create(&mut conn, SHOP, d).unwrap();
        assert_eq!(products::get(&mut conn, SHOP, made.id).unwrap().unit, unit);
    }
}
