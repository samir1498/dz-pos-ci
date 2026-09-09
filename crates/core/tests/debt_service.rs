// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The append-only debt ledger (features.md §2), against a real temp SQLite
//! file. The balance is the sum of the ledger and nothing else, so what these
//! assert is that every movement lands in it and that a movement which says
//! nothing cannot land at all.

use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::money::{Money, PaymentMode, Regime, Totals};
use dzpos_core::services::audit;
use dzpos_core::services::customers::{self, NewCustomer, PartyKind};
use dzpos_core::services::debt::{
    self, DebtKind, DocumentRef, NewDebtAllocation, NewDebtEntry, PaymentMethod,
};
use dzpos_core::services::documents::{
    self, BalanceTriple, DocumentKind, NewDocument, PartyBlock, SellerBlock,
};

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
    assert_eq!(down.statement.balance, Money::centimes(100_000));

    let up = debt::adjust(&mut conn, SHOP, OWNER, id, Money::centimes(20_000), None).unwrap();
    assert_eq!(up.entry.debit, Money::centimes(20_000));
    assert_eq!(up.entry.credit, Money::ZERO);
    assert_eq!(up.statement.balance, Money::centimes(120_000));
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

/// A user id no row carries. `debt_ledger.user_id` and `audit_log.user_id`
/// both have a foreign key to `users`, so a write naming this fails at the
/// file.
const NO_SUCH_USER: i32 = 999;

/// The movement and the entry that says who wrote it are one transaction, so
/// a correction that fails leaves neither.
///
/// The foreign key fires on the ledger insert, which is the first of the two
/// writes, so what this holds is that nothing survives a failed adjustment.
/// The fiche's `an_update_that_fails_at_the_audit_entry_leaves_the_fiche_as_it_was`
/// is where the audit's own rollback is pinned: an update writes no other
/// row carrying a user id, so there the audit entry is the write that fails.
#[test]
fn an_adjustment_that_fails_leaves_neither_the_movement_nor_the_log() {
    let (_dir, mut conn) = open_temp();
    let id = a_customer(&mut conn, "Brahim");
    debt::adjust(&mut conn, SHOP, OWNER, id, Money::centimes(150_000), None).unwrap();
    // The fiche's own creation is logged too, so the count is what it was
    // before rather than a number written down here.
    let logged = audit::list(&mut conn, SHOP).unwrap().len();

    let err = debt::adjust(
        &mut conn,
        SHOP,
        NO_SUCH_USER,
        id,
        Money::centimes(50_000),
        Some("erreur de saisie".into()),
    )
    .unwrap_err();
    assert_eq!(err.code(), "storage", "{err}");
    assert_eq!(
        debt::ledger(&mut conn, SHOP, id).unwrap().len(),
        1,
        "the failed correction left a movement behind"
    );
    assert_eq!(
        debt::balance(&mut conn, SHOP, id).unwrap(),
        Money::centimes(150_000),
        "the balance moved for a correction that was never written"
    );
    assert_eq!(
        audit::list(&mut conn, SHOP).unwrap().len(),
        logged,
        "the failed correction left an entry behind"
    );
}

/// The correction answers the ledger its own transaction read, and the
/// balance the audit's after carries is that same figure. A second read
/// afterwards would be a second answer to what the customer owes, and the
/// two could part company between them.
#[test]
fn an_adjustment_answers_the_ledger_its_own_transaction_read() {
    let (_dir, mut conn) = open_temp();
    let id = a_customer(&mut conn, "Brahim");
    debt::append(&mut conn, SHOP, movement(id, DebtKind::Opening, 150_000, 0)).unwrap();

    let written = debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        id,
        Money::centimes(-50_000),
        Some("erreur de saisie".to_string()),
    )
    .unwrap();

    assert_eq!(written.statement.balance, Money::centimes(100_000));
    let running: Vec<(DebtKind, Money)> = written
        .statement
        .lines
        .iter()
        .map(|line| (line.entry.kind, line.balance_after))
        .collect();
    assert_eq!(
        running,
        [
            (DebtKind::Adjustment, Money::centimes(100_000)),
            (DebtKind::Opening, Money::centimes(150_000)),
        ],
        "the movement it wrote is not the newest line of what it answered"
    );
    assert_eq!(written.statement.lines[0].entry.id, written.entry.id);

    let log = audit::list(&mut conn, SHOP).unwrap();
    let entry = log
        .iter()
        .find(|e| e.entity == "customer_debt")
        .expect("the adjustment left no audit entry");
    let after: serde_json::Value =
        serde_json::from_str(entry.after.as_deref().unwrap_or("null")).unwrap();
    assert_eq!(
        after["balance_centimes"],
        written.statement.balance.as_centimes(),
        "the log and the answer carry two different balances"
    );
}

// ---------------------------------------------------------------- payments
//
// The documents below are built by hand, `debt::append` writing the `sale`
// row beside a document that carries its own `remaining_debt`, rather than by
// selling on credit: the credit sale is being written on its own branch and
// this half of features.md §2 is the settlement, not the sale.

/// A document made out to `customer` for `net` centimes, unpaid, issued on
/// the day given so the oldest-first order is a fact of the fixture and not
/// of the insert order.
fn a_document_on_credit(conn: &mut SqliteConnection, customer_id: i32, net: i64, day: u32) -> i32 {
    let net = Money::centimes(net);
    let issued_at = NaiveDate::from_ymd_opt(2026, 9, day)
        .and_then(|d| d.and_hms_opt(10, 0, 0))
        .unwrap();
    let before = debt::balance(conn, SHOP, customer_id).unwrap();
    let doc = documents::issue(
        conn,
        SHOP,
        NewDocument {
            kind: DocumentKind::Facture,
            issued_at,
            user_id: OWNER,
            regime: Regime::Reel,
            payment_mode: PaymentMode::Credit,
            seller: SellerBlock {
                name: "Mon magasin".to_string(),
                rc: None,
                nif: None,
                nis: None,
                ai: None,
                address: None,
                phone: None,
            },
            customer_id: Some(customer_id),
            buyer: Some(PartyBlock {
                name: "Entreprise Benali".to_string(),
                party_kind: PartyKind::Company,
                rc: None,
                nif: None,
                nis: None,
                ai: None,
                address: None,
            }),
            ref_document_id: None,
            balance: Some(BalanceTriple {
                old_balance: before,
                remaining_debt: net,
                total_debt: before.checked_add(net).unwrap(),
            }),
            totals: Totals {
                total_ht: net,
                discount: Money::ZERO,
                subtotal_ht: net,
                tva_by_rate: Vec::new(),
                tva: Money::ZERO,
                total_ttc: net,
                stamp: Money::ZERO,
                net_to_pay: net,
            },
            tendered: None,
            change: None,
            lines: Vec::new(),
        },
    )
    .unwrap();
    // Stamped with the day the document was issued, not the day the test
    // runs: a fixture dated by the wall clock would drift in and out of the
    // ranges the statement tests read, and pass or fail by the calendar.
    debt::append_at(
        conn,
        SHOP,
        NewDebtEntry {
            customer_id,
            document_id: Some(doc.id),
            kind: DebtKind::Sale,
            debit: net,
            credit: Money::ZERO,
            user_id: OWNER,
            note: None,
        },
        Some(issued_at),
    )
    .unwrap();
    doc.id
}

/// What the customer owed before the range, written as an opening balance on
/// a day of its own. `customers::create` can carry an opening debt, but the
/// row it writes is stamped now, and a statement test needs the movement to
/// sit on a day it chose.
fn an_opening_balance(conn: &mut SqliteConnection, customer_id: i32, centimes: i64, day: u32) {
    debt::append_at(
        conn,
        SHOP,
        NewDebtEntry {
            customer_id,
            document_id: None,
            kind: DebtKind::Opening,
            debit: Money::centimes(centimes),
            credit: Money::ZERO,
            user_id: OWNER,
            note: None,
        },
        Some(at(day)),
    )
    .unwrap();
}

fn at(day: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, day)
        .and_then(|d| d.and_hms_opt(16, 30, 0))
        .unwrap()
}

fn remaining_debt(conn: &mut SqliteConnection, document_id: i32) -> Money {
    documents::get(conn, SHOP, document_id)
        .unwrap()
        .balance
        .expect("a document issued on credit carries the balance triple")
        .remaining_debt
}

#[test]
fn a_payment_fills_the_oldest_documents_first_and_stops_where_the_money_does() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let first = a_document_on_credit(&mut conn, customer, 100_000, 10);
    let second = a_document_on_credit(&mut conn, customer, 200_000, 11);

    let paid = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(150_000),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap();

    assert_eq!(paid.entry.kind, DebtKind::Payment);
    assert_eq!(paid.entry.credit, Money::centimes(150_000));
    assert_eq!(paid.entry.debit, Money::ZERO);
    assert_eq!(paid.balance_after, Money::centimes(150_000));
    assert_eq!(
        paid.allocations
            .iter()
            .map(|a| (a.document_id, a.amount))
            .collect::<Vec<(i32, Money)>>(),
        [
            (first, Money::centimes(100_000)),
            (second, Money::centimes(50_000)),
        ],
        "the money did not fill the older document before it touched the newer"
    );
    // The document's own column moves with the allocation: the balance triple
    // says what is left unpaid on this piece of paper, so it cannot stay at
    // what the paper asked for once part of it has been settled.
    assert_eq!(remaining_debt(&mut conn, first), Money::ZERO);
    assert_eq!(remaining_debt(&mut conn, second), Money::centimes(150_000));
}

#[test]
fn a_second_payment_carries_on_from_where_the_first_stopped() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let first = a_document_on_credit(&mut conn, customer, 100_000, 10);
    let second = a_document_on_credit(&mut conn, customer, 200_000, 11);
    let pay = |conn: &mut SqliteConnection, centimes: i64, day: u32| {
        debt::pay(
            conn,
            SHOP,
            OWNER,
            customer,
            Money::centimes(centimes),
            PaymentMethod::Card,
            None,
            at(day),
        )
        .unwrap()
    };

    pay(&mut conn, 150_000, 12);
    let second_payment = pay(&mut conn, 150_000, 13);

    assert_eq!(
        second_payment
            .allocations
            .iter()
            .map(|a| (a.document_id, a.amount))
            .collect::<Vec<(i32, Money)>>(),
        [(second, Money::centimes(150_000))],
        "the second payment went back over a document the first one settled"
    );
    assert_eq!(second_payment.balance_after, Money::ZERO);
    assert_eq!(remaining_debt(&mut conn, first), Money::ZERO);
    assert_eq!(remaining_debt(&mut conn, second), Money::ZERO);
}

#[test]
fn a_payment_above_what_the_customer_owes_is_refused_with_the_outstanding_amount() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    a_document_on_credit(&mut conn, customer, 100_000, 10);

    let refused = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(100_001),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap_err();

    match refused {
        CoreError::PaymentAboveDebt {
            outstanding_centimes,
        } => assert_eq!(outstanding_centimes, 100_000),
        other => panic!("a payment over the debt was refused as {other:?}"),
    }
    // Money over a debt is an avoir's business, never a credit balance a
    // payment opened on the way past: nothing at all was written.
    assert_eq!(
        debt::balance(&mut conn, SHOP, customer).unwrap(),
        Money::centimes(100_000)
    );
    assert_eq!(debt::ledger(&mut conn, SHOP, customer).unwrap().len(), 1);
}

#[test]
fn a_customer_who_owes_nothing_takes_no_payment() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");

    let refused = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(1),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap_err();

    match refused {
        CoreError::PaymentAboveDebt {
            outstanding_centimes,
        } => assert_eq!(outstanding_centimes, 0),
        other => panic!("a payment against no debt was refused as {other:?}"),
    }
}

#[test]
fn a_payment_of_nothing_or_of_a_negative_is_refused() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    a_document_on_credit(&mut conn, customer, 100_000, 10);

    for centimes in [0, -1_000] {
        let refused = debt::pay(
            &mut conn,
            SHOP,
            OWNER,
            customer,
            Money::centimes(centimes),
            PaymentMethod::Cash,
            None,
            at(12),
        )
        .unwrap_err();
        assert!(
            matches!(refused, CoreError::Validation { ref field, .. } if field == "amount_centimes"),
            "{centimes} centimes was refused as {refused:?}"
        );
    }
}

#[test]
fn money_against_a_debt_no_document_carries_settles_the_balance_and_no_paper() {
    let (_dir, mut conn) = open_temp();
    // An opening balance is the debt the shop was carrying before it had the
    // app: real money owed, and no document in the file to place it on.
    let customer = customers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Entreprise Benali"),
        Some(Money::centimes(80_000)),
    )
    .unwrap()
    .id;

    let paid = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(30_000),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap();

    assert!(
        paid.allocations.is_empty(),
        "a payment invented a document to settle"
    );
    assert_eq!(paid.balance_after, Money::centimes(50_000));
}

#[test]
fn a_document_already_settled_by_an_allocation_nobody_wrote_a_payment_for_refuses_the_money() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let document = a_document_on_credit(&mut conn, customer, 100_000, 10);
    // An allocation written straight into the table moves no column, so the
    // document still reads as unpaid while it has already been settled in
    // full. Σ of the allocations is the only thing that catches it.
    let payment = debt::append(
        &mut conn,
        SHOP,
        NewDebtEntry {
            customer_id: customer,
            document_id: None,
            kind: DebtKind::Payment,
            debit: Money::ZERO,
            credit: Money::centimes(100_000),
            user_id: OWNER,
            note: None,
        },
    )
    .unwrap();
    debt::allocate(
        &mut conn,
        SHOP,
        NewDebtAllocation {
            payment_ledger_id: payment.id,
            document_id: document,
            amount: Money::centimes(100_000),
        },
    )
    .unwrap();
    // The forged payment took the balance to nothing, so the debt has to be
    // put back for the refusal under test to be the allocation check and not
    // the outstanding check.
    debt::append(
        &mut conn,
        SHOP,
        NewDebtEntry {
            customer_id: customer,
            document_id: None,
            kind: DebtKind::Adjustment,
            debit: Money::centimes(100_000),
            credit: Money::ZERO,
            user_id: OWNER,
            note: None,
        },
    )
    .unwrap();

    let refused = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(10_000),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap_err();

    assert!(
        matches!(refused, CoreError::Validation { ref field, .. } if field == "amount_centimes"),
        "a document settled twice over was refused as {refused:?}"
    );
    // The refusal rolled the whole thing back: no payment row, and the
    // document still reads what it read before.
    assert_eq!(
        debt::balance(&mut conn, SHOP, customer).unwrap(),
        Money::centimes(100_000)
    );
    assert_eq!(
        remaining_debt(&mut conn, document),
        Money::centimes(100_000)
    );
}

#[test]
fn a_payment_carries_the_mode_it_was_taken_in_and_the_moment_it_landed() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    a_document_on_credit(&mut conn, customer, 100_000, 10);

    let paid = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(40_000),
        PaymentMethod::Card,
        Some("acompte".to_string()),
        at(12),
    )
    .unwrap();

    assert_eq!(paid.entry.payment_mode, Some(PaymentMethod::Card));
    assert_eq!(paid.entry.created_at, at(12));
    assert_eq!(paid.entry.note.as_deref(), Some("acompte"));
    // A sale says nothing about a mode: nothing was handed over.
    let sale = debt::ledger(&mut conn, SHOP, customer)
        .unwrap()
        .into_iter()
        .find(|e| e.kind == DebtKind::Sale)
        .unwrap();
    assert_eq!(sale.payment_mode, None);
}

#[test]
fn a_payment_leaves_an_audit_entry_carrying_the_balance_on_both_sides() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let document = a_document_on_credit(&mut conn, customer, 100_000, 10);

    let paid = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(40_000),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap();

    let log = audit::list(&mut conn, SHOP).unwrap();
    let entry = log
        .iter()
        .find(|e| e.action == "pay_debt")
        .expect("the payment left no audit entry");
    let before: serde_json::Value =
        serde_json::from_str(entry.before.as_deref().unwrap_or("null")).unwrap();
    let after: serde_json::Value =
        serde_json::from_str(entry.after.as_deref().unwrap_or("null")).unwrap();
    assert_eq!(before["balance_centimes"], 100_000);
    assert_eq!(after["balance_centimes"], paid.balance_after.as_centimes());
    assert_eq!(after["amount_centimes"], 40_000);
    assert_eq!(after["payment_mode"], "cash");
    assert_eq!(after["ledger_id"], paid.entry.id);
    // What the money settled, document by document, and not only which
    // documents it touched: a log that says a facture was settled without
    // saying by how much cannot be read against the facture.
    assert_eq!(
        after["allocations"],
        serde_json::json!([{ "document_id": document, "amount_centimes": 40_000 }])
    );
}

#[test]
fn the_payments_of_a_customer_read_back_newest_first_with_what_each_one_settled() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let first = a_document_on_credit(&mut conn, customer, 100_000, 10);
    let second = a_document_on_credit(&mut conn, customer, 200_000, 11);
    debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(150_000),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap();
    debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(20_000),
        PaymentMethod::Card,
        None,
        at(13),
    )
    .unwrap();

    let payments = debt::payments(&mut conn, SHOP, customer).unwrap();

    assert_eq!(payments.len(), 2, "a sale row was read back as a payment");
    assert_eq!(payments[0].entry.credit, Money::centimes(20_000));
    assert_eq!(payments[0].balance_after, Money::centimes(130_000));
    assert_eq!(
        payments[0]
            .allocations
            .iter()
            .map(|a| a.document_id)
            .collect::<Vec<i32>>(),
        [second]
    );
    assert_eq!(payments[1].entry.credit, Money::centimes(150_000));
    assert_eq!(
        payments[1]
            .allocations
            .iter()
            .map(|a| a.document_id)
            .collect::<Vec<i32>>(),
        [first, second]
    );
}

#[test]
fn a_statement_opens_at_what_was_owed_before_the_range_and_closes_at_the_last_movement_in_it() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    // Four movements dated by hand: two before the range, one inside it and
    // one after. A range that took the wrong side of either day would read
    // the wrong opening or the wrong closing balance.
    an_opening_balance(&mut conn, customer, 150_000, 1);
    let document = a_document_on_credit(&mut conn, customer, 200_000, 5);
    debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(50_000),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap();
    debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(100_000),
        PaymentMethod::Card,
        None,
        at(25),
    )
    .unwrap();

    let range = debt::statement_between(
        &mut conn,
        SHOP,
        customer,
        NaiveDate::from_ymd_opt(2026, 9, 10).unwrap(),
        NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
    )
    .unwrap();

    // The opening balance is the running balance of the newest movement
    // before the range: the opening row and the facture, both dated earlier.
    assert_eq!(range.opening, Money::centimes(350_000));
    assert_eq!(
        range.entries.len(),
        1,
        "the range took a movement outside it"
    );
    assert_eq!(range.entries[0].entry.kind, DebtKind::Payment);
    assert_eq!(range.entries[0].balance_after, Money::centimes(300_000));
    assert_eq!(range.closing, Money::centimes(300_000));
    // A payment cites no document; the sale does, and it is outside this
    // range, so nothing here names one.
    assert_eq!(range.entries[0].document, None);

    // The same range widened to the sale picks up the document the movement
    // cites, under the kind and the number a customer quotes.
    let wider = debt::statement_between(
        &mut conn,
        SHOP,
        customer,
        NaiveDate::from_ymd_opt(2026, 9, 5).unwrap(),
        NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
    )
    .unwrap();
    let sale = wider
        .entries
        .iter()
        .find(|line| line.entry.kind == DebtKind::Sale)
        .expect("the widened range dropped the sale");
    assert_eq!(sale.entry.document_id, Some(document));
    assert_eq!(
        sale.document.map(|d| d.kind),
        Some(dzpos_core::services::documents::DocumentKind::Facture)
    );
}

#[test]
fn a_range_that_ends_before_it_starts_is_refused() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");

    let refused = debt::statement_between(
        &mut conn,
        SHOP,
        customer,
        NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
        NaiveDate::from_ymd_opt(2026, 9, 10).unwrap(),
    )
    .unwrap_err();

    assert!(
        matches!(refused, CoreError::Validation { ref field, .. } if field == "to"),
        "{refused:?}"
    );
}

#[test]
fn a_range_with_nothing_in_it_closes_where_it_opened() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    an_opening_balance(&mut conn, customer, 150_000, 1);

    let range = debt::statement_between(
        &mut conn,
        SHOP,
        customer,
        NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2027, 1, 31).unwrap(),
    )
    .unwrap();

    assert!(range.entries.is_empty());
    assert_eq!(range.opening, Money::centimes(150_000));
    assert_eq!(range.closing, range.opening);
}

#[test]
fn a_movement_is_stamped_by_the_shops_clock_and_not_by_utc() {
    // A document's `issued_at` is on the shop's calendar (services::clock),
    // and the ledger has to be on the same one: a statement asks for days,
    // and between 23:00 and midnight UTC the two clocks disagree about which
    // day a payment landed on. Algeria is UTC+1 all year, so a row stamped by
    // the database's own CURRENT_TIMESTAMP is an hour behind this.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");

    let written = debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Opening, 150_000, 0),
    )
    .unwrap();

    let drift = written
        .created_at
        .signed_duration_since(dzpos_core::services::clock::now())
        .num_seconds()
        .abs();
    assert!(
        drift <= 5,
        "the movement is stamped {drift}s from the shop's clock: {}",
        written.created_at
    );
}

#[test]
fn the_first_and_the_last_moment_of_a_range_are_inside_it() {
    // Both days are included (features.md §2), which is the whole of the two
    // days and not the two instants they start at: a payment taken at half
    // past midnight on the first day and one taken in the last minute of the
    // last day are both in the statement the customer is sent.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let day = |d: u32, h: u32, m: u32| {
        NaiveDate::from_ymd_opt(2026, 9, d)
            .and_then(|date| date.and_hms_opt(h, m, 0))
            .unwrap()
    };
    // Outside, first moment of the range, last moment of the range, outside.
    for (at, centimes) in [
        (day(9, 23, 30), 10_000),
        (day(10, 0, 30), 20_000),
        (day(20, 23, 30), 30_000),
        (day(21, 0, 30), 40_000),
    ] {
        debt::append_at(
            &mut conn,
            SHOP,
            movement(customer, DebtKind::Adjustment, centimes, 0),
            Some(at),
        )
        .unwrap();
    }

    let range = debt::statement_between(
        &mut conn,
        SHOP,
        customer,
        NaiveDate::from_ymd_opt(2026, 9, 10).unwrap(),
        NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
    )
    .unwrap();

    let inside: Vec<i64> = range
        .entries
        .iter()
        .map(|line| line.entry.debit.as_centimes())
        .collect();
    assert_eq!(
        inside,
        [20_000, 30_000],
        "the range took the wrong side of one of its two days"
    );
    assert_eq!(range.opening, Money::centimes(10_000));
    assert_eq!(range.closing, Money::centimes(60_000));
}

#[test]
fn a_cancelled_document_takes_none_of_a_payment() {
    // A cancelled facture is not a debt any more: whatever is left on its
    // `remaining_debt` column, money handed over settles the paper that still
    // stands. The ledger row the cancellation writes is what moves the
    // balance (T6); this only refuses to fill the cancelled sheet.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let cancelled = a_document_on_credit(&mut conn, customer, 100_000, 10);
    let standing = a_document_on_credit(&mut conn, customer, 200_000, 11);
    diesel::sql_query(format!(
        "UPDATE documents SET status = 'cancelled' WHERE id = {cancelled}"
    ))
    .execute(&mut conn)
    .unwrap();

    let paid = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(50_000),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap();

    assert_eq!(
        paid.allocations
            .iter()
            .map(|a| a.document_id)
            .collect::<Vec<i32>>(),
        [standing],
        "the payment filled a cancelled document"
    );
    assert_eq!(
        remaining_debt(&mut conn, cancelled),
        Money::centimes(100_000)
    );
    assert_eq!(
        remaining_debt(&mut conn, standing),
        Money::centimes(150_000)
    );
}

#[test]
fn a_cancelled_document_takes_none_of_a_correction_downwards_either() {
    // A correction downwards settles paper through the same read as a
    // payment (`allocate_oldest_first`), so it inherits the `status =
    // issued` filter. Asserted on its own path rather than trusted to the
    // payment's: the two callers are the whole of what that filter protects,
    // and a rewrite that gave one of them a query of its own would otherwise
    // go through green.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let cancelled = a_document_on_credit(&mut conn, customer, 100_000, 10);
    let standing = a_document_on_credit(&mut conn, customer, 200_000, 11);
    diesel::sql_query(format!(
        "UPDATE documents SET status = 'cancelled' WHERE id = {cancelled}"
    ))
    .execute(&mut conn)
    .unwrap();

    let corrected = debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(-50_000),
        Some("erreur de saisie".to_string()),
    )
    .unwrap();

    assert_eq!(
        corrected
            .allocations
            .iter()
            .map(|a| a.document_id)
            .collect::<Vec<i32>>(),
        [standing],
        "the correction came off a cancelled document"
    );
    assert_eq!(
        remaining_debt(&mut conn, cancelled),
        Money::centimes(100_000)
    );
    assert_eq!(
        remaining_debt(&mut conn, standing),
        Money::centimes(150_000)
    );
}

#[test]
fn the_oldest_document_is_the_one_issued_first_and_not_the_one_written_first() {
    // The two orders are made to disagree: the newer facture is written into
    // the file first and carries the lower id. Oldest-first means the day the
    // paper was issued, which is what a customer means by "my oldest
    // invoice", so a settlement sorted by id would fill the wrong one.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let newer = a_document_on_credit(&mut conn, customer, 200_000, 11);
    let older = a_document_on_credit(&mut conn, customer, 100_000, 10);
    assert!(
        newer < older,
        "the fixture no longer inverts the two orders"
    );

    let paid = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(100_000),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap();

    assert_eq!(
        paid.allocations
            .iter()
            .map(|a| a.document_id)
            .collect::<Vec<i32>>(),
        [older]
    );
    assert_eq!(remaining_debt(&mut conn, older), Money::ZERO);
    assert_eq!(remaining_debt(&mut conn, newer), Money::centimes(200_000));
}

#[test]
fn two_documents_in_the_same_second_are_filled_in_the_order_they_were_written() {
    // `issued_at` is whole seconds, so two factures rung up in the same
    // second carry the same one. The id breaks the tie: settling oldest first
    // has to mean one order, and the answer cannot depend on whichever order
    // SQLite felt like returning.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let first = a_document_on_credit(&mut conn, customer, 100_000, 10);
    let second = a_document_on_credit(&mut conn, customer, 200_000, 10);

    let paid = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(150_000),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap();

    assert_eq!(
        paid.allocations
            .iter()
            .map(|a| (a.document_id, a.amount.as_centimes()))
            .collect::<Vec<(i32, i64)>>(),
        [(first, 100_000), (second, 50_000)]
    );
}

#[test]
fn a_payment_never_reaches_the_documents_of_the_other_customer() {
    // Two fiches of the same shop. Money handed over by one settles that
    // one's paper and nothing else: the other customer's oldest facture is
    // older than anything here and would be filled first by a query that
    // forgot whose debt it was reading.
    let (_dir, mut conn) = open_temp();
    let payer = a_customer(&mut conn, "Entreprise Benali");
    let other = a_customer(&mut conn, "Entreprise Amrani");
    let theirs = a_document_on_credit(&mut conn, other, 100_000, 5);
    let mine = a_document_on_credit(&mut conn, payer, 200_000, 10);

    let paid = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        payer,
        Money::centimes(150_000),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap();

    assert_eq!(
        paid.allocations
            .iter()
            .map(|a| a.document_id)
            .collect::<Vec<i32>>(),
        [mine]
    );
    assert_eq!(remaining_debt(&mut conn, theirs), Money::centimes(100_000));
    assert_eq!(remaining_debt(&mut conn, mine), Money::centimes(50_000));
    assert_eq!(
        debt::balance(&mut conn, SHOP, other).unwrap(),
        Money::centimes(100_000),
        "the other customer's balance moved"
    );
}

#[test]
fn a_correction_downwards_settles_the_oldest_documents_the_way_a_payment_does() {
    // A discount agreed after the facture was printed, a returned bag of
    // cement, a keying mistake: the correction lowers the debt, and the paper
    // it lowers has to say so too. Otherwise the document keeps asking for
    // 1 000,00 while the ledger says 700,00, and a payment of what is really
    // owed is refused by the very documents it was meant to close.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let document = a_document_on_credit(&mut conn, customer, 100_000, 10);

    let corrected = debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(-30_000),
        Some("remise accordée après coup".to_string()),
    )
    .unwrap();

    assert_eq!(
        corrected
            .allocations
            .iter()
            .map(|a| (a.document_id, a.amount.as_centimes()))
            .collect::<Vec<(i32, i64)>>(),
        [(document, 30_000)]
    );
    assert_eq!(remaining_debt(&mut conn, document), Money::centimes(70_000));

    // And what is left on the paper is exactly what the customer can now pay
    // off it, in one go and without a refusal.
    let paid = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(70_000),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap();
    assert_eq!(paid.balance_after, Money::ZERO);
    assert_eq!(remaining_debt(&mut conn, document), Money::ZERO);
}

#[test]
fn a_correction_upwards_is_debt_that_no_document_carries() {
    // Money owed that no paper asks for: it raises the balance and leaves
    // every document exactly as it was. Nothing to settle means nothing to
    // allocate, and a facture must never grow because a correction did.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let document = a_document_on_credit(&mut conn, customer, 100_000, 10);

    let corrected = debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(20_000),
        None,
    )
    .unwrap();

    assert!(corrected.allocations.is_empty());
    assert_eq!(
        remaining_debt(&mut conn, document),
        Money::centimes(100_000)
    );
    assert_eq!(corrected.statement.balance, Money::centimes(120_000));
}

#[test]
fn a_correction_downwards_is_audited_with_what_it_took_off_each_document() {
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let first = a_document_on_credit(&mut conn, customer, 100_000, 10);
    let second = a_document_on_credit(&mut conn, customer, 200_000, 11);

    debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(-150_000),
        None,
    )
    .unwrap();

    let log = audit::list(&mut conn, SHOP).unwrap();
    let entry = log
        .iter()
        .find(|e| e.action == "adjust_debt")
        .expect("the correction left no audit entry");
    let after: serde_json::Value =
        serde_json::from_str(entry.after.as_deref().unwrap_or("null")).unwrap();
    assert_eq!(
        after["allocations"],
        serde_json::json!([
            { "document_id": first, "amount_centimes": 100_000 },
            { "document_id": second, "amount_centimes": 50_000 },
        ])
    );
}

#[test]
fn credit_taken_before_the_facture_existed_stays_on_the_ledger_and_not_on_the_paper() {
    // Frozen from the property run in tests/debt_prop.rs, which found it
    // while the invariant was written as "the papers never ask for more than
    // the balance": correct 100,00 off an account that owes nothing, then
    // sell 100,00 on credit. The customer owes nothing, and the facture still
    // asks for its whole net, because that is what a facture is issued with
    // (features.md §3) and there was nothing unpaid for the correction to
    // settle when it landed.
    //
    // So the two figures part company by exactly the credit nobody could
    // place. The document is not wrong and the ledger is not wrong; what a
    // later payment can settle is what the ledger says, and that is nothing.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let corrected = debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(-100_000),
        None,
    )
    .unwrap();
    assert!(
        corrected.allocations.is_empty(),
        "there was no document to settle"
    );

    let document = a_document_on_credit(&mut conn, customer, 100_000, 10);

    assert_eq!(
        debt::balance(&mut conn, SHOP, customer).unwrap(),
        Money::ZERO
    );
    assert_eq!(
        remaining_debt(&mut conn, document),
        Money::centimes(100_000)
    );
    // And nothing can be handed over against it, because nothing is owed.
    let refused = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(1_000),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap_err();
    assert!(
        matches!(refused, CoreError::PaymentAboveDebt { outstanding_centimes } if outstanding_centimes == 0),
        "{refused:?}"
    );
}

#[test]
fn a_closed_fiche_still_takes_a_payment() {
    // A shop closes a fiche to stop selling to somebody, not to stop
    // collecting from them: a customer who owes 1 000,00 on the day their
    // fiche is closed still walks in with the money. The sale is what a
    // closed fiche refuses (services::sales), never the settlement.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    let document = a_document_on_credit(&mut conn, customer, 100_000, 10);
    let mut closed = fiche("Entreprise Benali");
    closed.active = false;
    customers::update(&mut conn, SHOP, OWNER, customer, closed).unwrap();

    let paid = debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(100_000),
        PaymentMethod::Cash,
        None,
        at(12),
    )
    .unwrap();

    assert_eq!(paid.balance_after, Money::ZERO);
    assert_eq!(remaining_debt(&mut conn, document), Money::ZERO);
}

#[test]
fn a_closed_fiche_still_takes_a_correction() {
    // Same rule from the other side: a keying mistake on a fiche that has
    // since been closed is still a mistake, and the only way to correct a
    // ledger is to write a movement.
    let (_dir, mut conn) = open_temp();
    let customer = a_customer(&mut conn, "Entreprise Benali");
    debt::append(
        &mut conn,
        SHOP,
        movement(customer, DebtKind::Opening, 150_000, 0),
    )
    .unwrap();
    let mut closed = fiche("Entreprise Benali");
    closed.active = false;
    customers::update(&mut conn, SHOP, OWNER, customer, closed).unwrap();

    let corrected = debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        customer,
        Money::centimes(-50_000),
        Some("erreur de saisie".to_string()),
    )
    .unwrap();

    assert_eq!(corrected.statement.balance, Money::centimes(100_000));
}

#[test]
fn the_recent_movements_are_the_newest_ones_and_the_balance_counts_them_all() {
    // What the debt slip prints (features.md §2). The window is the newest
    // few movements; the figure under the customer's name is the whole
    // ledger, so a shop handing over the slip is not quoting a smaller debt
    // than the one it is owed.
    let (_dir, mut conn) = open_temp();
    let id = a_customer(&mut conn, "Brahim");
    a_document(&mut conn, 1, SHOP, 7);
    debt::append(&mut conn, SHOP, movement(id, DebtKind::Opening, 150_000, 0)).unwrap();
    for _ in 0..11 {
        debt::append(&mut conn, SHOP, movement(id, DebtKind::Sale, 10_000, 0)).unwrap();
    }
    let newest = debt::append(
        &mut conn,
        SHOP,
        NewDebtEntry {
            document_id: Some(1),
            ..movement(id, DebtKind::Sale, 5_000, 0)
        },
    )
    .unwrap();

    let slip = debt::recent(&mut conn, SHOP, id, 10).unwrap();

    assert_eq!(
        slip.balance,
        Money::centimes(265_000),
        "the balance is not what the whole ledger sums to"
    );
    assert_eq!(
        slip.balance,
        debt::balance(&mut conn, SHOP, id).unwrap(),
        "the slip and the balance query disagree"
    );
    assert_eq!(
        slip.entries.len(),
        10,
        "the window is not the size asked for"
    );
    assert_eq!(
        slip.entries[0].entry.id, newest.id,
        "the rows do not read newest first"
    );
    assert_eq!(
        slip.entries[0].balance_after,
        Money::centimes(265_000),
        "the newest row does not carry the balance as of itself"
    );
    // The oldest movement of all is outside the window, which is the whole
    // point of it: the opening row is the thirteenth from the end.
    assert!(
        !slip
            .entries
            .iter()
            .any(|line| line.entry.kind == DebtKind::Opening),
        "a movement older than the window reached the slip"
    );
    // The document a row cites is named by the kind and number a customer
    // quotes, read off the document rather than off the ledger row.
    assert_eq!(
        slip.entries[0].document,
        Some(DocumentRef {
            kind: DocumentKind::Facture,
            number: 7,
        }),
        "the newest row does not name the facture it was written for"
    );
    assert!(
        slip.entries[1].document.is_none(),
        "a movement citing no document was given one"
    );
}
