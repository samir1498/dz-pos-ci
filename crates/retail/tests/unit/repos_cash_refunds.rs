// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::repos::testdb::{open, OWNER, SHOP};

fn at(day: u32, hour: u32) -> NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 9, day)
        .and_then(|d| d.and_hms_opt(hour, 0, 0))
        .expect("a real moment")
}

fn write(document_id: i32, centimes: i64, day: u32, hour: u32) -> CashRefundRowWrite {
    CashRefundRowWrite {
        shop_id: SHOP,
        document_id,
        user_id: OWNER,
        amount_centimes: centimes,
        refunded_at: at(day, hour),
    }
}

/// The round trip, and the index behind it: a second refund naming the
/// same paper is refused, and refused as a validation error rather than
/// as a bare query failure.
#[test]
fn one_document_is_refunded_once() {
    let (_dir, mut conn) = open();
    let document = seed_a_ticket(&mut conn);
    let written = append(&mut conn, &write(document, 3_000, 10, 12)).expect("the first refund");
    assert_eq!(written.amount, Money::centimes(3_000));
    assert_eq!(written.user_id, OWNER);

    let err = append(&mut conn, &write(document, 3_000, 10, 13)).expect_err("the second");
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "document_id"),
        "a second refund on one document is a validation error, was {err:?}"
    );
}

/// A ticket to hang the refund on. Written straight in: the repo is being
/// asked about its own table and not about how a sale is made.
fn seed_a_ticket(conn: &mut SqliteConnection) -> i32 {
    #[derive(diesel::QueryableByName)]
    struct Id {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        id: i32,
    }
    diesel::sql_query(
        "INSERT INTO documents (shop_id, kind, series, number, issued_at, user_id, \
         seller_name, payment_mode, regime, total_ht_centimes, discount_centimes, \
         subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes, \
         net_to_pay_centimes, status) \
         VALUES (1, 'ticket', 'doc_ticket:2026', \
         (SELECT COALESCE(MAX(number), 0) + 1 FROM documents), \
         '2026-09-10 12:00:00', 1, 'A shop', \
         'cash', 'reel', 3000, 0, 3000, 0, 3000, 0, 3000, 'issued')",
    )
    .execute(conn)
    .expect("a seeded ticket");
    let found: Id = diesel::sql_query("SELECT MAX(id) AS id FROM documents")
        .get_result(conn)
        .expect("its id");
    found.id
}

/// The shop sum takes every person's refunds; the narrow one takes the
/// named person's and leaves the other's alone. Both are half open, so a
/// refund at the closing second falls outside.
#[test]
fn the_shop_sum_and_one_persons_sum_are_not_the_same_question() {
    let (_dir, mut conn) = open();
    let first = seed_a_ticket(&mut conn);
    let second = seed_a_ticket(&mut conn);
    append(&mut conn, &write(first, 3_000, 10, 12)).expect("the owner's refund");
    let other = CashRefundRowWrite {
        user_id: 2,
        ..write(second, 5_000, 10, 13)
    };
    diesel::sql_query(
        "INSERT INTO users (id, shop_id, name, role, active) \
         VALUES (2, 1, 'B', 'cashier', 1)",
    )
    .execute(&mut conn)
    .expect("a second cashier");
    append(&mut conn, &other).expect("the other cashier's refund");

    assert_eq!(
        total_for_shop(&mut conn, SHOP, at(10, 0), at(11, 0)).expect("the shop's day"),
        Money::centimes(8_000)
    );
    assert_eq!(
        total_for_user(&mut conn, SHOP, OWNER, at(10, 0), at(11, 0)).expect("one person"),
        Money::centimes(3_000)
    );
    // Half open at the top: the 13:00 refund is outside a window ending
    // at 13:00 and the 12:00 one is inside a window starting there.
    assert_eq!(
        total_for_shop(&mut conn, SHOP, at(10, 12), at(10, 13)).expect("one hour"),
        Money::centimes(3_000)
    );
    // A window with nothing in it answers zero rather than nothing.
    assert_eq!(
        total_for_shop(&mut conn, SHOP, at(11, 0), at(12, 0)).expect("an empty day"),
        Money::ZERO
    );
}
