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

/// A second shop with a category of its own, and that category's id. There
/// is no shops service yet (M7 pairs a second till), so the rows go in raw:
/// what is under test is the service's scoping, never this seed.
fn seed_second_shop(conn: &mut SqliteConnection) -> i32 {
    use diesel::prelude::*;

    #[derive(diesel::QueryableByName)]
    struct Id {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        id: i32,
    }

    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO categories (shop_id, name, default_rate_bps) VALUES (2, 'Autre', 900)",
    )
    .execute(conn)
    .unwrap();
    let row: Id = diesel::sql_query("SELECT id FROM categories WHERE shop_id = 2")
        .get_result(conn)
        .unwrap();
    row.id
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
fn a_typed_barcode_sitting_on_the_next_auto_number_does_not_block_it() {
    // The auto number used to be the row's id, so a user who typed the number
    // the next row was about to get rolled that insert back for ever: the id
    // was never consumed and every later blank create landed on it again.
    // 2000010000029 is the in-store code for sequence 2.
    let (_dir, mut conn) = open_temp();
    let mut taken = draft("Saisi à la main");
    taken.barcode = Some("2000010000029".to_string());
    products::create(&mut conn, SHOP, taken).unwrap();

    let a = products::create(&mut conn, SHOP, draft("A")).unwrap();
    let b = products::create(&mut conn, SHOP, draft("B")).unwrap();
    let (ba, bb) = (a.barcode.unwrap(), b.barcode.unwrap());
    assert_ne!(ba, bb, "two blank creates got the same number");
    for code in [&ba, &bb] {
        assert_ne!(code, "2000010000029", "the auto number reused a typed one");
        assert_eq!(code.len(), 13, "not an EAN-13: {code}");
        assert!(code.starts_with('2'), "not an in-store code: {code}");
    }
    assert_eq!(products::list(&mut conn, SHOP).unwrap().len(), 3);
}

#[test]
fn each_shop_numbers_its_own_in_store_barcodes() {
    // The series is per (shop_id, name): shop 2's first blank product gets
    // shop 2's sequence 1, and shop 1's counter does not move for it. With
    // the shop hardcoded in the counter repo both shops would share one
    // series and this goes red.
    let (_dir, mut conn) = open_temp();
    seed_second_shop(&mut conn);
    for name in ["A", "B", "C"] {
        products::create(&mut conn, SHOP, draft(name)).unwrap();
    }
    let mut for_shop_two = draft("Deuxième A");
    for_shop_two.category_id = None;
    for_shop_two.rate_bps = Some(Bps::new(900).unwrap());
    let other = products::create(&mut conn, 2, for_shop_two).unwrap();
    assert_eq!(
        other.barcode.as_deref(),
        Some("2000020000019"),
        "shop 2 did not start its own series at 1"
    );
    // The barcode itself is not proof: `next_free_in_store_barcode` walks
    // past taken numbers, so a rewound counter would still hand out 4. Read
    // the counter row, which only a scoped UPDATE leaves alone.
    let shop_one_next: i64 = {
        use diesel::prelude::*;
        use dzpos_core::schema::counters::dsl::*;
        counters
            .filter(shop_id.eq(SHOP))
            .filter(name.eq("in_store_barcode"))
            .select(next_value)
            .first(&mut conn)
            .unwrap()
    };
    assert_eq!(
        shop_one_next, 4,
        "shop 1's counter moved because of shop 2's product"
    );
    let next = products::create(&mut conn, SHOP, draft("D")).unwrap();
    assert_eq!(
        next.barcode.as_deref(),
        Some("2000010000043"),
        "shop 1's series moved because of shop 2's product"
    );
}

#[test]
fn a_spent_barcode_series_is_not_blamed_on_the_user() {
    // The counter cannot advance past i64::MAX. That is the shop's series
    // being exhausted, not bad input, so the code is "exhausted" (409 at
    // the API), never "validation".
    use diesel::prelude::*;

    let (_dir, mut conn) = open_temp();
    diesel::sql_query(format!(
        "INSERT INTO counters (shop_id, name, next_value) VALUES (1, 'in_store_barcode', {}) \
         ON CONFLICT (shop_id, name) DO UPDATE SET next_value = excluded.next_value",
        i64::MAX
    ))
    .execute(&mut conn)
    .unwrap();
    let err = products::create(&mut conn, SHOP, draft("A")).unwrap_err();
    assert_eq!(err.code(), "exhausted", "{err}");
    assert_eq!(
        products::list(&mut conn, SHOP).unwrap().len(),
        0,
        "a product landed without a number"
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
fn one_barcode_may_exist_once_in_each_shop() {
    // The unique index is (shop_id, barcode). Making it global passed every
    // other test in this suite, so this is the one that pins it: two shops
    // stocking the same product print the manufacturer's code on both.
    let (_dir, mut conn) = open_temp();
    let theirs = seed_second_shop(&mut conn);

    let mut mine = draft("Huile Elio 5L");
    mine.barcode = Some("6130001000018".to_string());
    let ours = products::create(&mut conn, SHOP, mine).unwrap();
    assert_eq!(ours.shop_id, SHOP);

    let mut yours = draft("Huile Elio 5L");
    yours.barcode = Some("6130001000018".to_string());
    yours.category_id = Some(theirs);
    let made = products::create(&mut conn, 2, yours).unwrap();
    assert_eq!(made.shop_id, 2);
    assert_eq!(made.barcode.as_deref(), Some("6130001000018"));

    // And each shop still sees only its own.
    assert_eq!(products::list(&mut conn, SHOP).unwrap().len(), 1);
    assert_eq!(products::list(&mut conn, 2).unwrap().len(), 1);
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
fn another_shops_category_is_rejected_even_when_the_rate_is_explicit() {
    // The category was only looked up when it had to supply a rate, so an
    // explicit rate let a product point at another shop's category (rule 3).
    let (_dir, mut conn) = open_temp();
    let theirs = seed_second_shop(&mut conn);
    let mut d = draft("A");
    d.category_id = Some(theirs);
    d.rate_bps = Some(Bps::new(1900).unwrap());
    match products::create(&mut conn, SHOP, d) {
        Err(CoreError::NotFound { entity, id }) => {
            assert_eq!(entity, "category");
            assert_eq!(id, theirs);
        }
        other => panic!("expected a category NotFound, got {other:?}"),
    }
}

#[test]
fn a_category_that_no_longer_exists_is_not_found_rather_than_a_storage_failure() {
    // The check and the insert share one transaction, so the row cannot
    // slip between them; a category gone before the call is a 404, never
    // the foreign-key failure the database would raise (a 500).
    use diesel::prelude::*;

    let (_dir, mut conn) = open_temp();
    let gone = seed_second_shop(&mut conn);
    diesel::sql_query(format!("DELETE FROM categories WHERE id = {gone}"))
        .execute(&mut conn)
        .unwrap();
    let mut d = draft("Orpheline");
    d.category_id = Some(gone);
    d.rate_bps = Some(Bps::new(900).unwrap());
    let err = products::create(&mut conn, 2, d).unwrap_err();
    assert_eq!(err.code(), "not_found", "{err}");
}

#[test]
fn an_update_onto_another_shops_category_is_rejected_too() {
    let (_dir, mut conn) = open_temp();
    let theirs = seed_second_shop(&mut conn);
    let made = products::create(&mut conn, SHOP, draft("A")).unwrap();
    let mut d = draft("A");
    d.category_id = Some(theirs);
    d.rate_bps = Some(Bps::new(1900).unwrap());
    match products::update(&mut conn, SHOP, made.id, d) {
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
    // The migration's CHECK now refuses this row, which is the point of the
    // constraint. The pragma stands in for the file this test is about: one
    // written by an old import or a repair tool that never saw the CHECK.
    diesel::sql_query("PRAGMA ignore_check_constraints = ON")
        .execute(&mut conn)
        .unwrap();
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
