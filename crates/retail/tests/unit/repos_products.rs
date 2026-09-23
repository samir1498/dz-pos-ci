// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::money::Money;
use crate::repos::testdb::{open, SHOP};

/// A second shop beside the seeded one. Raw, because what is under test
/// is the `shop_id` filter on the three functions below and never this
/// seed; there is no shops repo call that would make the row more real.
const OTHER: i32 = 2;

fn a_second_shop(conn: &mut SqliteConnection) {
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(conn)
        .unwrap();
}

/// One product in one shop, named and coded and costed by the caller so
/// a test can put the same barcode in both shops.
fn a_product(
    conn: &mut SqliteConnection,
    shop_id: i32,
    name: &str,
    barcode: &str,
    cost: i64,
) -> i32 {
    diesel::sql_query(format!(
        "INSERT INTO products (shop_id, name, barcode, unit, cost_centimes, \
         selling_centimes, qty_on_hand_milli, low_stock_at_milli, rate_bps) \
         VALUES ({shop_id}, '{name}', '{barcode}', 'piece', {cost}, 26000, 0, 0, 1900)"
    ))
    .execute(conn)
    .unwrap();
    products::table
        .filter(products::shop_id.eq(shop_id))
        .filter(products::barcode.eq(barcode))
        .select(products::id)
        .first(conn)
        .unwrap()
}

/// `get` is what `services::purchases` calls to decide a product is this
/// shop's before it puts the product on an order, so the filter here is
/// what turns the shop next door's product into a `NotFound` rather than
/// a foreign key failure later (rule 3).
#[test]
fn a_shop_gets_its_own_product_and_never_the_shop_next_doors() {
    let (_dir, mut conn) = open();
    a_second_shop(&mut conn);
    let mine = a_product(&mut conn, SHOP, "Farine 5kg", "6130001000018", 20_000);
    let theirs = a_product(&mut conn, OTHER, "Farine 5kg", "6130001000025", 21_000);

    assert_eq!(get(&mut conn, SHOP, mine).unwrap().id, mine);
    match get(&mut conn, SHOP, theirs) {
        Err(CoreError::NotFound { entity, id }) => {
            assert_eq!(entity, "product");
            assert_eq!(id, theirs);
        }
        other => panic!("expected another shop's product to be NotFound, got {other:?}"),
    }
    // And the row is still there for the shop that owns it, so the miss
    // above is "not this shop's" and not "nothing is there".
    assert_eq!(get(&mut conn, OTHER, theirs).unwrap().id, theirs);
}

/// The same barcode in both shops, which the UNIQUE index is on
/// `(shop_id, barcode)` to permit. Asking shop 2 has to answer shop 2's
/// row: a dropped `shop_id` filter would hand back whichever row the
/// file reads first, which is the one this shop never sold. The Excel
/// import matches a row on this, so a wrong answer overwrites the fiche
/// of a shop that is not importing (features.md §1).
#[test]
fn one_barcode_in_two_shops_reads_back_as_each_shops_own_product() {
    let (_dir, mut conn) = open();
    a_second_shop(&mut conn);
    let mine = a_product(&mut conn, SHOP, "Sucre 1kg", "6130002000017", 12_000);
    let theirs = a_product(&mut conn, OTHER, "Sucre 1kg", "6130002000017", 13_000);
    assert_ne!(mine, theirs);

    assert_eq!(
        by_barcode(&mut conn, SHOP, "6130002000017")
            .unwrap()
            .map(|p| p.id),
        Some(mine)
    );
    assert_eq!(
        by_barcode(&mut conn, OTHER, "6130002000017")
            .unwrap()
            .map(|p| p.id),
        Some(theirs)
    );
    // A code neither shop carries is None rather than the other shop's.
    assert!(by_barcode(&mut conn, OTHER, "6130009000014")
        .unwrap()
        .is_none());
}

/// `set_cost` is the one write here that a purchase drives: the landed
/// unit cost of a receipt, in centimes. A lost `shop_id` filter would
/// let one shop's delivery rewrite the cost price on another shop's
/// fiche, so this asserts both halves, that the call is refused and that
/// the untouched shop's centimes are exactly what they were.
#[test]
fn a_receipt_costs_its_own_shops_fiche_and_leaves_the_next_doors_alone() {
    let (_dir, mut conn) = open();
    a_second_shop(&mut conn);
    let mine = a_product(&mut conn, SHOP, "Huile 5L", "6130003000016", 20_000);
    let theirs = a_product(&mut conn, OTHER, "Huile 5L", "6130003000023", 21_000);

    set_cost(&mut conn, SHOP, mine, Money::centimes(23_500)).unwrap();
    assert_eq!(
        get(&mut conn, SHOP, mine).unwrap().cost,
        Money::centimes(23_500)
    );

    match set_cost(&mut conn, SHOP, theirs, Money::centimes(1)) {
        Err(CoreError::NotFound { entity, id }) => {
            assert_eq!(entity, "product");
            assert_eq!(id, theirs);
        }
        other => panic!("expected another shop's product to be NotFound, got {other:?}"),
    }
    assert_eq!(
        get(&mut conn, OTHER, theirs).unwrap().cost,
        Money::centimes(21_000),
        "the shop next door's cost price is the centimes it was inserted with"
    );
}
