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
use dzpos_core::services::audit;
use dzpos_core::services::customers::{self, NewCustomer, PartyKind};
use dzpos_core::services::debt::{self, DebtKind, NewDebtAllocation, NewDebtEntry};

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

/// A document to settle against. The debt service is what is under test, so
/// the row is written straight in: issuing one through the sale path would
/// need a product, a régime and a cart, none of which this asserts anything
/// about.
fn a_document(conn: &mut SqliteConnection, id: i32, shop_id: i32, number: i32) {
    diesel::sql_query(format!(
        "INSERT INTO documents (id, shop_id, kind, series, number, issued_at, user_id, \
         regime, payment_mode, seller_name, total_ht_centimes, discount_centimes, \
         subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes, \
         net_to_pay_centimes) VALUES ({id}, {shop_id}, 'facture', 'doc_facture', {number}, \
         '2026-09-09 10:00:00', 1, 'reel', 'credit', 'Magasin', 100000, 0, 100000, 19000, \
         119000, 0, 119000)"
    ))
    .execute(conn)
    .unwrap();
}

#[test]
fn an_allocation_says_which_document_a_payment_settled() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    a_document(&mut conn, 1, SHOP, 1);
    let payment = debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Payment, 0, 50_000),
    )
    .unwrap();

    let made = debt::allocate(
        &mut conn,
        SHOP,
        NewDebtAllocation {
            payment_ledger_id: payment.id,
            document_id: 1,
            amount: Money::centimes(50_000),
        },
    )
    .unwrap();
    let read = debt::allocations(&mut conn, SHOP, 1).unwrap();
    assert_eq!(read, vec![made]);
    assert_eq!(read[0].amount, Money::centimes(50_000));
}

#[test]
fn an_allocation_of_nothing_is_refused() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    a_document(&mut conn, 1, SHOP, 1);
    let payment = debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Payment, 0, 50_000),
    )
    .unwrap();

    for amount in [Money::ZERO, Money::centimes(-1)] {
        let err = debt::allocate(
            &mut conn,
            SHOP,
            NewDebtAllocation {
                payment_ledger_id: payment.id,
                document_id: 1,
                amount,
            },
        )
        .unwrap_err();
        assert!(
            matches!(err, CoreError::Validation { ref field, .. } if field == "amount"),
            "{err}"
        );
    }
    assert!(debt::allocations(&mut conn, SHOP, 1).unwrap().is_empty());
}

#[test]
fn an_allocation_never_reaches_across_shops() {
    // Rule 3: both foreign keys would take the other shop's row, so the
    // payment and the document are each checked against the shop asking.
    let (_dir, mut conn) = open_temp();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut conn)
        .unwrap();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    a_document(&mut conn, 1, SHOP, 1);
    a_document(&mut conn, 2, 2, 1);
    let payment = debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Payment, 0, 50_000),
    )
    .unwrap();

    let stolen_document = debt::allocate(
        &mut conn,
        SHOP,
        NewDebtAllocation {
            payment_ledger_id: payment.id,
            document_id: 2,
            amount: Money::centimes(50_000),
        },
    )
    .unwrap_err();
    assert!(
        matches!(
            stolen_document,
            CoreError::NotFound {
                entity: "document",
                ..
            }
        ),
        "{stolen_document}"
    );

    let stolen_payment = debt::allocate(
        &mut conn,
        2,
        NewDebtAllocation {
            payment_ledger_id: payment.id,
            document_id: 2,
            amount: Money::centimes(50_000),
        },
    )
    .unwrap_err();
    assert!(
        matches!(
            stolen_payment,
            CoreError::NotFound {
                entity: "debt_entry",
                ..
            }
        ),
        "{stolen_payment}"
    );

    let stolen_list = debt::allocations(&mut conn, 2, 1).unwrap_err();
    assert!(
        matches!(
            stolen_list,
            CoreError::NotFound {
                entity: "document",
                ..
            }
        ),
        "{stolen_list}"
    );
    assert!(debt::allocations(&mut conn, SHOP, 1).unwrap().is_empty());
}

#[test]
fn a_movement_pointing_at_another_shops_document_is_refused() {
    // The sale that raised the debt is named by id, and the foreign key
    // would take the neighbour's facture: a statement would then cite a
    // document this shop never issued (rule 3).
    let (_dir, mut conn) = open_temp();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut conn)
        .unwrap();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    a_document(&mut conn, 1, 2, 1);

    let mut entry = movement(customer, DebtKind::Sale, 100_000, 0);
    entry.document_id = Some(1);
    let err = debt::append(&mut conn, SHOP, entry).unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::NotFound {
                entity: "document",
                ..
            }
        ),
        "{err}"
    );
    assert_eq!(
        debt::balance(&mut conn, SHOP, customer).unwrap(),
        Money::ZERO,
        "the refused movement landed anyway"
    );
}

/// The fiche prints a running balance beside each movement, and it is the
/// core's to compute: a screen adding the column itself would be a second
/// place the debt is worked out, and the two would disagree the day a
/// movement is added.
#[test]
fn the_statement_runs_the_balance_up_from_the_oldest_movement() {
    let (_dir, mut conn) = open_temp();
    let id = a_customer(&mut conn, "Brahim");
    debt::append(&mut conn, SHOP, movement(id, DebtKind::Opening, 150_000, 0)).unwrap();
    debt::append(&mut conn, SHOP, movement(id, DebtKind::Sale, 50_000, 0)).unwrap();
    debt::append(&mut conn, SHOP, movement(id, DebtKind::Payment, 0, 70_000)).unwrap();

    let statement = debt::statement(&mut conn, SHOP, id).unwrap();
    assert_eq!(
        statement.balance,
        Money::centimes(130_000),
        "the envelope's balance is not what the ledger sums to"
    );
    assert_eq!(
        statement.balance,
        debt::balance(&mut conn, SHOP, id).unwrap(),
        "the statement and the balance query disagree"
    );
    let read: Vec<(DebtKind, Money)> = statement
        .lines
        .iter()
        .map(|line| (line.entry.kind, line.balance_after))
        .collect();
    assert_eq!(
        read,
        [
            (DebtKind::Payment, Money::centimes(130_000)),
            (DebtKind::Sale, Money::centimes(200_000)),
            (DebtKind::Opening, Money::centimes(150_000)),
        ],
        "the lines read newest first and each one carries the balance as of itself"
    );
}

#[test]
fn a_customer_with_no_movement_has_an_empty_statement_and_owes_nothing() {
    let (_dir, mut conn) = open_temp();
    let id = a_customer(&mut conn, "Brahim");
    let statement = debt::statement(&mut conn, SHOP, id).unwrap();
    assert!(statement.lines.is_empty());
    assert_eq!(statement.balance, Money::ZERO);
}

#[test]
fn another_shop_reads_no_statement_of_this_shops_customer() {
    let (_dir, mut conn) = open_temp();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut conn)
        .unwrap();
    let id = a_customer(&mut conn, "Brahim");
    let err = debt::statement(&mut conn, 2, id).unwrap_err();
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

/// features.md §2: the ledger is append-only, so a wrong figure is corrected
/// by a movement a comptable can read. The sign says which way it moves.
#[test]
fn an_adjustment_moves_the_debt_the_way_its_sign_says_and_answers_the_new_balance() {
    let (_dir, mut conn) = open_temp();
    let id = a_customer(&mut conn, "Brahim");
    debt::append(&mut conn, SHOP, movement(id, DebtKind::Opening, 150_000, 0)).unwrap();

    let down = debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        id,
        Money::centimes(-50_000),
        Some("erreur de saisie".to_string()),
    )
    .unwrap();
    assert_eq!(down.entry.kind, DebtKind::Adjustment);
    assert_eq!(down.entry.credit, Money::centimes(50_000));
    assert_eq!(down.entry.debit, Money::ZERO);
    assert_eq!(down.entry.note.as_deref(), Some("erreur de saisie"));
    assert_eq!(down.balance, Money::centimes(100_000));

    let up = debt::adjust(&mut conn, SHOP, OWNER, id, Money::centimes(20_000), None).unwrap();
    assert_eq!(up.entry.debit, Money::centimes(20_000));
    assert_eq!(up.entry.credit, Money::ZERO);
    assert_eq!(up.balance, Money::centimes(120_000));
    assert_eq!(
        debt::balance(&mut conn, SHOP, id).unwrap(),
        Money::centimes(120_000),
        "the answer and the stored ledger disagree"
    );
    assert_eq!(debt::ledger(&mut conn, SHOP, id).unwrap().len(), 3);
}

#[test]
fn an_adjustment_of_nothing_is_refused_and_writes_no_row() {
    let (_dir, mut conn) = open_temp();
    let id = a_customer(&mut conn, "Brahim");
    let err = debt::adjust(&mut conn, SHOP, OWNER, id, Money::ZERO, None).unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "amount"),
        "{err}"
    );
    assert!(debt::ledger(&mut conn, SHOP, id).unwrap().is_empty());
    assert!(audit::list(&mut conn, SHOP)
        .unwrap()
        .iter()
        .all(|entry| entry.entity != "customer_debt"));
}

/// features.md §5: a correction to what somebody owes is a sensitive action,
/// and the entry carries the two balances so a reader never has to re-derive
/// them from the ledger.
#[test]
fn an_adjustment_is_audited_with_the_balance_before_and_after() {
    let (_dir, mut conn) = open_temp();
    let id = a_customer(&mut conn, "Brahim");
    debt::append(&mut conn, SHOP, movement(id, DebtKind::Opening, 150_000, 0)).unwrap();
    debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        id,
        Money::centimes(-50_000),
        Some("erreur de saisie".to_string()),
    )
    .unwrap();

    let log = audit::list(&mut conn, SHOP).unwrap();
    let entry = log
        .iter()
        .find(|e| e.entity == "customer_debt")
        .expect("the adjustment left no audit entry");
    assert_eq!(entry.action, "adjust_debt");
    assert_eq!(entry.entity_id, Some(id));
    assert_eq!(entry.user_id, OWNER);
    let before: serde_json::Value =
        serde_json::from_str(entry.before.as_deref().unwrap_or("null")).unwrap();
    let after: serde_json::Value =
        serde_json::from_str(entry.after.as_deref().unwrap_or("null")).unwrap();
    assert_eq!(before["balance_centimes"], 150_000);
    assert_eq!(after["balance_centimes"], 100_000);
    assert_eq!(after["amount_centimes"], -50_000);
    assert_eq!(after["note"], "erreur de saisie");
}

#[test]
fn an_adjustment_for_another_shops_customer_is_not_found_and_writes_nothing() {
    let (_dir, mut conn) = open_temp();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut conn)
        .unwrap();
    let id = a_customer(&mut conn, "Brahim");
    let err = debt::adjust(&mut conn, 2, OWNER, id, Money::centimes(50_000), None).unwrap_err();
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
    assert!(debt::ledger(&mut conn, SHOP, id).unwrap().is_empty());
    assert!(audit::list(&mut conn, 2).unwrap().is_empty());
}
