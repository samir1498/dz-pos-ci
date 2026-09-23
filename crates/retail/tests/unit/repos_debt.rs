// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::models::customer::{CustomerRowWrite, PartyKind};
use crate::models::debt::{DebtKind, PaymentMethod};
use crate::repos::customers;
use crate::repos::testdb::{open, OWNER, SHOP};

fn a_customer(conn: &mut SqliteConnection, name: &str) -> i32 {
    customers::insert(
        conn,
        &CustomerRowWrite {
            shop_id: SHOP,
            name: name.to_string(),
            party_kind: PartyKind::Consumer,
            phone: None,
            address: None,
            rc: None,
            nif: None,
            nis: None,
            ai: None,
            credit_limit_centimes: None,
            warn_threshold_centimes: None,
            notes: None,
            active: true,
            updated_at: dzpos_kernel::services::clock::now(),
        },
    )
    .unwrap()
    .id
}

fn movement(customer_id: i32, kind: DebtKind, debit: i64, credit: i64) -> DebtRowWrite {
    DebtRowWrite {
        shop_id: SHOP,
        customer_id,
        document_id: None,
        kind,
        debit_centimes: debit,
        credit_centimes: credit,
        user_id: OWNER,
        note: None,
        payment_mode: None,
        // Stamped from the shop's clock, which is what `append` asks of
        // every caller.
        created_at: Some(dzpos_kernel::services::clock::now()),
    }
}

/// Every query here takes a `shop_id`, and the customer ledger is the
/// column the dashboard reads its debt figure from, so a shop seeing the
/// shop next door's debtors is the worst answer this file can give. The
/// supplier ledger beside it is proved the same way
/// (`repos::supplier_debt`).
#[test]
fn a_shop_reads_its_own_ledger_and_never_the_shop_next_door() {
    let (_dir, mut conn) = open();
    let customer = a_customer(&mut conn, "Cliente Amrani");
    append(&mut conn, &movement(customer, DebtKind::Sale, 100_000, 0)).unwrap();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(&mut conn)
        .unwrap();

    assert!(balances(&mut conn, 2).unwrap().is_empty());
    assert!(ledger(&mut conn, 2, customer).unwrap().is_empty());
    // And this shop still has what it wrote, so the two reads above are
    // answering "not this shop's" rather than "nothing is there".
    assert_eq!(
        balances(&mut conn, SHOP).unwrap(),
        vec![(customer, 100_000, 0)]
    );
}

#[test]
fn a_movement_with_no_moment_on_it_is_refused_rather_than_dated_by_the_file() {
    // The column's default is SQLite's CURRENT_TIMESTAMP, which is UTC,
    // and the day a shop counts its drawer for is a day on the shop's
    // calendar (UTC+1). A payment taken at 00:30 in Algiers would land on
    // the day before in the file and drop out of that count and out of
    // the cash position, which reads this column through
    // `repos::cash::customer_payments`. So the caller stamps it from the
    // clock or the row does not go in.
    let (_dir, mut conn) = open();
    let customer = a_customer(&mut conn, "Cliente Sans Heure");
    let mut unstamped = movement(customer, DebtKind::Payment, 0, 1_000);
    unstamped.payment_mode = Some(PaymentMethod::Cash);
    unstamped.created_at = None;
    match append(&mut conn, &unstamped) {
        Err(RetailError::Unstamped { entity }) => assert_eq!(entity, "debt_ledger"),
        other => panic!("expected an unstamped row to be refused, got {other:?}"),
    }
    // And nothing was written: a refusal leaves the ledger as it was.
    assert!(ledger(&mut conn, SHOP, customer).unwrap().is_empty());
}

#[test]
fn a_stamped_movement_keeps_the_moment_it_was_handed() {
    // The mirror of the test above, so the refusal cannot be read as
    // this repo refusing every payment. What it adds over
    // `debt_service::a_movement_is_stamped_by_the_shops_clock_and_not_by_utc`,
    // which already holds the calendar a layer up, is exactness: the
    // moment comes back the same to the nanosecond, so the column keeps
    // what it was handed rather than something near it.
    let (_dir, mut conn) = open();
    let customer = a_customer(&mut conn, "Cliente Amrani");
    let at = dzpos_kernel::services::clock::now();
    let mut stamped = movement(customer, DebtKind::Payment, 0, 1_000);
    stamped.payment_mode = Some(PaymentMethod::Cash);
    stamped.created_at = Some(at);
    let written = append(&mut conn, &stamped).unwrap();
    assert_eq!(written.created_at, at);
    assert_eq!(ledger(&mut conn, SHOP, customer).unwrap().len(), 1);
}
