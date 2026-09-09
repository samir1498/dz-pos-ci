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
use dzpos_core::services::debt::{self, DebtKind, NewDebtAllocation, NewDebtEntry, PaymentMethod};
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
fn a_document_on_credit(
    conn: &mut SqliteConnection,
    customer_id: i32,
    net: i64,
    day: u32,
) -> i32 {
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
    debt::append(
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
    )
    .unwrap();
    doc.id
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
    assert_eq!(remaining_debt(&mut conn, document), Money::centimes(100_000));
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
    assert_eq!(after["allocated_document_ids"][0], document);
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
