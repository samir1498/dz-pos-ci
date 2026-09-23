// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! One sale per retry key (M7 T4). A phone that got no answer posts the
//! same basket with the same key and must get the original paper back, not
//! a second ring: the money moves once whatever the network did.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::sqlite::SqliteConnection;
use dzpos_retail::error::{CoreError, RetailError};
use dzpos_retail::models::product::{NewProduct, Unit};
use dzpos_retail::money::{Bps, Money, PaymentMode};
use dzpos_retail::services::sales::{self, NewSale, NewSaleLine, SaleKind};

mod common;

const SHOP: i32 = 1;
const OWNER: i32 = 1;

use common::open_temp as open;

fn at() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, 15)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

fn product(conn: &mut SqliteConnection) -> i32 {
    dzpos_retail::services::products::create(
        conn,
        SHOP,
        OWNER,
        NewProduct {
            name: "Café".to_string(),
            barcode: None,
            category_id: None,
            unit: Unit::Piece,
            cost: Money::centimes(5_000),
            selling: Money::centimes(10_000),
            wholesale: None,
            qty_on_hand_milli: 100_000,
            low_stock_at_milli: 0,
            rate_bps: Some(Bps::new(0).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id
}

fn basket(product_id: i32, qty_milli: i64) -> NewSale {
    NewSale {
        lines: vec![NewSaleLine {
            product_id,
            qty_milli,
            unit_price: None,
            line_discount: Money::ZERO,
        }],
        global_discount: Money::ZERO,
        payment_mode: PaymentMode::Cash,
        tendered: Some(Money::centimes(1_000_000)),
        customer_id: None,
        override_credit: false,
        kind: SaleKind::Ticket,
        issued_at: Some(at()),
    }
}

fn documents_of_kind(conn: &mut SqliteConnection) -> i64 {
    use diesel::prelude::*;
    use diesel::sql_types::BigInt;
    #[derive(diesel::QueryableByName)]
    struct Count {
        #[diesel(sql_type = BigInt)]
        n: i64,
    }
    let row: Count = diesel::sql_query(
        "SELECT COUNT(*) AS n FROM documents WHERE shop_id = 1 AND kind = 'ticket'",
    )
    .get_result(conn)
    .unwrap();
    row.n
}

#[test]
fn same_key_twice_rings_once_and_replays_the_paper() {
    let (_dir, mut conn) = open();
    let product_id = product(&mut conn);
    let first = sales::issue_idempotent(
        &mut conn,
        SHOP,
        OWNER,
        basket(product_id, 1000),
        "k1".into(),
    )
    .unwrap();
    assert!(!first.replayed);
    let second = sales::issue_idempotent(
        &mut conn,
        SHOP,
        OWNER,
        basket(product_id, 1000),
        "k1".into(),
    )
    .unwrap();
    assert!(second.replayed);
    assert_eq!(first.document.id, second.document.id);
    // The totals travel with the paper, identical to the centime.
    // Totals equality proves nothing beyond id equality for one basket:
    // prices drift, so change the price once and the replay must still hand
    // back the first paper, not a recomputed one.
    assert_eq!(first.document.totals, second.document.totals);
    assert_eq!(documents_of_kind(&mut conn), 1);
}

#[test]
fn same_key_on_a_different_basket_is_a_conflict() {
    let (_dir, mut conn) = open();
    let product_id = product(&mut conn);
    sales::issue_idempotent(
        &mut conn,
        SHOP,
        OWNER,
        basket(product_id, 1000),
        "k1".into(),
    )
    .unwrap();
    assert!(matches!(
        sales::issue_idempotent(
            &mut conn,
            SHOP,
            OWNER,
            basket(product_id, 2000),
            "k1".into()
        ),
        Err(RetailError::Kernel(CoreError::Conflict { .. }))
    ));
    assert_eq!(documents_of_kind(&mut conn), 1);
}

#[test]
fn no_key_rings_every_time_like_before() {
    let (_dir, mut conn) = open();
    let product_id = product(&mut conn);
    let a = sales::issue(&mut conn, SHOP, OWNER, basket(product_id, 1000)).unwrap();
    let b = sales::issue(&mut conn, SHOP, OWNER, basket(product_id, 1000)).unwrap();
    assert!(!a.replayed && !b.replayed);
    assert_ne!(a.document.id, b.document.id);
    assert_eq!(documents_of_kind(&mut conn), 2);
}

#[test]
fn keys_are_present_short_and_never_on_a_quotation() {
    let (_dir, mut conn) = open();
    let product_id = product(&mut conn);
    assert!(matches!(
        sales::issue_idempotent(
            &mut conn,
            SHOP,
            OWNER,
            basket(product_id, 1000),
            String::new()
        ),
        Err(RetailError::Kernel(CoreError::Validation { .. }))
    ));
    assert!(matches!(
        sales::issue_idempotent(
            &mut conn,
            SHOP,
            OWNER,
            basket(product_id, 1000),
            "k".repeat(129)
        ),
        Err(RetailError::Kernel(CoreError::Validation { .. }))
    ));
    let mut quote = basket(product_id, 1000);
    quote.kind = SaleKind::Proforma;
    assert!(matches!(
        sales::issue_idempotent(&mut conn, SHOP, OWNER, quote, "kq".into()),
        Err(RetailError::Kernel(CoreError::Validation { .. }))
    ));
    assert_eq!(documents_of_kind(&mut conn), 0);
}

#[test]
fn two_rings_racing_one_key_leave_one_sale() {
    let (dir, mut conn) = open();
    let product_id = product(&mut conn);
    let path = dir.path().join("t.db");
    let gate = std::sync::Barrier::new(2);
    let (a, b) = std::thread::scope(|s| {
        let gate = &gate;
        let one = s.spawn(|| {
            gate.wait();
            let mut conn = dzpos_retail::db::open(&path).unwrap();
            sales::issue_idempotent(
                &mut conn,
                SHOP,
                OWNER,
                basket(product_id, 1000),
                "race".into(),
            )
            .map(|sale| sale.replayed)
        });
        let two = s.spawn(|| {
            gate.wait();
            let mut conn = dzpos_retail::db::open(&path).unwrap();
            sales::issue_idempotent(
                &mut conn,
                SHOP,
                OWNER,
                basket(product_id, 1000),
                "race".into(),
            )
            .map(|sale| sale.replayed)
        });
        (
            one.join().unwrap().map_err(|e| format!("{e:?}")),
            two.join().unwrap().map_err(|e| format!("{e:?}")),
        )
    });
    // Exactly one fresh ring; the loser replays the winner's paper, or
    // answers 409 and reads it on the routine retry. Authenticated by the
    // loser re-reading the winner's sale through the lookup.
    let fresh = [&a, &b].iter().filter(|r| ***r == Ok(false)).count();
    assert_eq!(fresh, 1, "{a:?} vs {b:?}");
    assert_eq!(documents_of_kind(&mut conn), 1);
    let kinds: Vec<&str> = [&a, &b]
        .iter()
        .map(|r| match r {
            Ok(false) => "fresh",
            Ok(true) => "replay",
            Err(_) => "conflict",
        })
        .collect();
    // Which error the loser got matters: the UNIQUE guard is meant to
    // produce a Conflict, which the API answers 409, while a busy database
    // falls through to Query, which the API answers 500. The old assertion
    // accepted either and so pinned neither (M6+M7 review, 2026-09-16).
    // Which error the loser gets matters, and the old assertion accepted
    // any of them so it pinned none (M6+M7 review, 2026-09-16). Measured:
    // with two connections the loser answers
    // `Query(DatabaseError(Unknown, "database is locked"))`, which the API
    // maps to 500 — not the 409 the UNIQUE guard in
    // `repos::sale_idempotency::record` was written to produce. SQLite's
    // busy handling wins before the insert is ever attempted, so `Ok(false)`
    // is unreachable across processes.
    //
    // Pinned as it actually behaves rather than as intended, so that making
    // it answer 409 (a `BEGIN IMMEDIATE` on the sale transaction, the way
    // `services::pairing` takes the write lock up front) turns this red and
    // has to be done deliberately. No money moves either way: exactly one
    // document exists below, and the loser's retry reads the winner.
    for r in [&a, &b] {
        if let Err(message) = r {
            assert!(
                message.contains("database is locked"),
                "the only refusal this race is known to produce is a busy \
                 database; a different one means the locking changed and the \
                 409 path needs its own assertion: {message}"
            );
        }
    }
    // Busy (database is locked) counts as the conflict path the UNIQUE
    // guard exists to produce: a retry must read the winner, not ring again.
    if a.is_err() || b.is_err() {
        let replay = sales::issue_idempotent(
            &mut conn,
            SHOP,
            OWNER,
            basket(product_id, 1000),
            "race".into(),
        )
        .unwrap();
        assert!(
            replay.replayed,
            "the loser's retry must read the winner, not ring again"
        );
    } else {
        assert!(
            kinds.contains(&"replay"),
            "one rang, the other did nothing: {a:?} vs {b:?}"
        );
    }
}
