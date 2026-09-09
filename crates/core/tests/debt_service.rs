// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The append-only debt ledger (features.md §2), against a real temp SQLite
//! file. The balance is the sum of the ledger and nothing else, so what these
//! assert is that every movement lands in it and that a movement which says
//! nothing cannot land at all.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::money::Money;
use dzpos_core::services::customers::{self, NewCustomer, PartyKind};
use dzpos_core::services::debt::{self, DebtKind, NewDebtEntry};

const SHOP: i32 = 1;
/// The owner the first migration seeds.
const OWNER: i32 = 1;

fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let conn = dzpos_core::db::open(&path).unwrap();
    (dir, conn)
}

fn fiche(name: &str) -> NewCustomer {
    NewCustomer {
        name: name.to_string(),
        party_kind: PartyKind::Company,
        phone: None,
        address: None,
        rc: None,
        nif: None,
        nis: None,
        ai: None,
        credit_limit: None,
        warn_threshold: None,
        notes: None,
        active: true,
    }
}

fn a_customer(conn: &mut SqliteConnection, name: &str) -> i32 {
    customers::create(conn, SHOP, OWNER, fiche(name), None)
        .unwrap()
        .id
}

fn movement(customer_id: i32, kind: DebtKind, debit: i64, credit: i64) -> NewDebtEntry {
    NewDebtEntry {
        customer_id,
        document_id: None,
        kind,
        debit: Money::centimes(debit),
        credit: Money::centimes(credit),
        user_id: OWNER,
        note: None,
    }
}

#[test]
fn a_balance_is_the_sum_of_what_was_appended() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    assert_eq!(
        debt::balance(&mut conn, SHOP, customer).unwrap(),
        Money::ZERO,
        "a customer nobody has sold to owes nothing"
    );

    debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Sale, 250_000, 0),
    )
    .unwrap();
    debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Payment, 0, 100_000),
    )
    .unwrap();
    assert_eq!(
        debt::balance(&mut conn, SHOP, customer).unwrap(),
        Money::centimes(150_000)
    );
}

#[test]
fn a_customer_who_overpays_is_owed_money() {
    // The balance is signed on purpose: clamping it at zero would lose the
    // 500 DA the shop is holding, and the statement has to print it.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Sale, 100_000, 0),
    )
    .unwrap();
    debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Payment, 0, 150_000),
    )
    .unwrap();
    assert_eq!(
        debt::balance(&mut conn, SHOP, customer).unwrap(),
        Money::centimes(-50_000)
    );
}

#[test]
fn a_row_carrying_a_debit_and_a_credit_at_once_is_refused_by_the_check() {
    // The service refuses it first, so the CHECK is asserted through raw SQL:
    // a file written by anything else has to meet the same rule.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let refused = diesel::sql_query(format!(
        "INSERT INTO debt_ledger (shop_id, customer_id, kind, debit_centimes, \
         credit_centimes, user_id) VALUES (1, {customer}, 'sale', 1000, 1000, 1)"
    ))
    .execute(&mut conn);
    assert!(
        refused.is_err(),
        "a ledger row carried a debit and a credit at once"
    );

    let err = debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Sale, 1000, 1000),
    )
    .unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "debit"),
        "{err}"
    );
    assert_eq!(
        debt::balance(&mut conn, SHOP, customer).unwrap(),
        Money::ZERO
    );
}

#[test]
fn a_movement_of_nothing_is_refused() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let err = debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Adjustment, 0, 0),
    )
    .unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "debit"),
        "{err}"
    );
    assert!(debt::ledger(&mut conn, SHOP, customer).unwrap().is_empty());
}

#[test]
fn a_negative_movement_is_refused_rather_than_flipped() {
    // A credit written as a negative debit would balance the same and read as
    // a sale in the statement, so it is refused rather than corrected.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let err = debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Sale, -1000, 0),
    )
    .unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "debit"),
        "{err}"
    );
}

#[test]
fn the_ledger_reads_newest_first_and_the_id_breaks_a_tie_inside_one_second() {
    // `created_at` is whole seconds and two movements land inside one when a
    // payment settles two documents, so the order has to come from the id.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let first = debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Sale, 100_000, 0),
    )
    .unwrap();
    let second = debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Sale, 200_000, 0),
    )
    .unwrap();
    diesel::sql_query("UPDATE debt_ledger SET created_at = '2026-09-09 10:00:00'")
        .execute(&mut conn)
        .unwrap();

    let rows = debt::ledger(&mut conn, SHOP, customer).unwrap();
    assert_eq!(
        rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![second.id, first.id],
        "two movements inside one second came back in insertion order"
    );
}

#[test]
fn another_shops_customer_is_not_found() {
    // Rule 3. The foreign key alone would take the id happily.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut conn)
        .unwrap();

    for err in [
        debt::balance(&mut conn, 2, customer).unwrap_err(),
        debt::ledger(&mut conn, 2, customer).unwrap_err(),
        debt::append(&mut conn, 2, movement(customer, DebtKind::Sale, 1000, 0)).unwrap_err(),
    ] {
        assert!(
            matches!(
                err,
                CoreError::NotFound {
                    entity: "customer",
                    ..
                }
            ),
            "{err}"
        );
    }
    assert_eq!(
        debt::balance(&mut conn, SHOP, customer).unwrap(),
        Money::ZERO,
        "the refused append still wrote a row"
    );
}

#[test]
fn a_note_longer_than_a_statement_prints_is_refused() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let mut entry = movement(customer, DebtKind::Adjustment, 1000, 0);
    entry.note = Some("é".repeat(201));
    let err = debt::append(&mut conn, SHOP, entry).unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "note"),
        "{err}"
    );
}
