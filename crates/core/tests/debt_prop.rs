// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The invariant that ties the ledger to the paper, over sequences nobody
//! wrote by hand: whatever order sales, corrections and payments arrive in,
//! what the documents still ask for adds up to no more than what the customer
//! owes, and every document's own three figures agree.
//!
//! Against a real temp SQLite file, one per case: the invariant is about what
//! the writes leave in the file, and a generator feeding a mock would prove
//! the mock.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::money::{Money, PaymentMode, Regime, Totals};
use dzpos_core::services::customers::{self, NewCustomer, PartyKind};
use dzpos_core::services::debt::{self, DebtKind, NewDebtEntry, PaymentMethod};
use dzpos_core::services::documents::{
    self, BalanceTriple, DocumentKind, NewDocument, PartyBlock, SellerBlock,
};
use proptest::prelude::*;

const SHOP: i32 = 1;
const OWNER: i32 = 1;

/// One thing a shop does to a customer's account. The amounts are whole
/// dinars in a range small enough that payments and corrections land on top
/// of each other rather than each running out of documents to touch.
#[derive(Debug, Clone, Copy)]
enum Step {
    /// A facture on credit for this many centimes.
    Sell(i64),
    /// A correction, up or down, by this many centimes.
    Correct(i64),
    /// Money handed over. Refused when it is more than the customer owes,
    /// which is one of the things worth generating.
    Pay(i64),
}

fn steps() -> impl Strategy<Value = Vec<Step>> {
    let amount = (1i64..=20).prop_map(|d| d * 10_000);
    let step = prop_oneof![
        amount.clone().prop_map(Step::Sell),
        (amount.clone(), any::<bool>()).prop_map(|(a, up)| Step::Correct(if up { a } else { -a })),
        amount.prop_map(Step::Pay),
    ];
    prop::collection::vec(step, 1..=12)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn the_papers_never_ask_for_more_than_the_ledger_says_is_owed(steps in steps()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let mut conn = dzpos_core::db::open(&path).unwrap();
        let customer = a_customer(&mut conn);
        let mut day = 1;

        for (n, step) in steps.iter().enumerate() {
            match *step {
                Step::Sell(centimes) => {
                    day += 1;
                    a_facture_on_credit(&mut conn, customer, centimes, day);
                }
                Step::Correct(centimes) => {
                    debt::adjust(&mut conn, SHOP, OWNER, customer, Money::centimes(centimes), None)
                        .unwrap();
                }
                Step::Pay(centimes) => {
                    let before = debt::balance(&mut conn, SHOP, customer).unwrap();
                    match debt::pay(
                        &mut conn,
                        SHOP,
                        OWNER,
                        customer,
                        Money::centimes(centimes),
                        PaymentMethod::Cash,
                        None,
                        chrono::NaiveDate::from_ymd_opt(2026, 9, day)
                            .and_then(|d| d.and_hms_opt(16, 30, 0))
                            .unwrap(),
                    ) {
                        Ok(_) => {}
                        // More than the customer owes is refused, and a
                        // refusal leaves the file exactly as it was.
                        Err(CoreError::PaymentAboveDebt { .. }) => {
                            let after = debt::balance(&mut conn, SHOP, customer).unwrap();
                            prop_assert_eq!(before, after, "a refused payment moved the balance");
                        }
                        Err(other) => prop_assert!(false, "step {}: {:?}", n, other),
                    }
                }
            }
            check(&mut conn, customer, n)?;
        }

        // And the point of all of it: whatever the sequence was, the customer
        // can hand over exactly what the ledger says they owe and no document
        // refuses it. A paper still asking for more than the ledger does is
        // what would turn that last payment into a refusal.
        let owed = debt::balance(&mut conn, SHOP, customer).unwrap();
        if !owed.is_negative() && owed != Money::ZERO {
            debt::pay(
                &mut conn,
                SHOP,
                OWNER,
                customer,
                owed,
                PaymentMethod::Cash,
                None,
                chrono::NaiveDate::from_ymd_opt(2026, 10, 1)
                    .and_then(|d| d.and_hms_opt(16, 30, 0))
                    .unwrap(),
            )
            .map_err(|e| TestCaseError::fail(format!("settling {owed:?} was refused: {e:?}")))?;
            prop_assert_eq!(
                debt::balance(&mut conn, SHOP, customer).unwrap(),
                Money::ZERO
            );
            check(&mut conn, customer, steps.len())?;
        }
    }
}

/// What must be true after every step.
fn check(conn: &mut SqliteConnection, customer: i32, n: usize) -> Result<(), TestCaseError> {
    let balance = debt::balance(conn, SHOP, customer).unwrap();
    let mut owed_on_paper = Money::ZERO;
    for (document_id, remaining, net_to_pay) in issued_documents(conn, customer) {
        // Every centime of a document is either still asked for or placed on
        // it by something: the three figures are one statement about the same
        // piece of paper, so two of them decide the third.
        let placed: i64 = debt::allocations(conn, SHOP, document_id)
            .unwrap()
            .iter()
            .map(|a| a.amount.as_centimes())
            .sum();
        prop_assert!(
            remaining >= 0,
            "step {}: document {} owes {}",
            n,
            document_id,
            remaining
        );
        prop_assert_eq!(
            placed + remaining,
            net_to_pay,
            "step {}: document {} placed {} + remaining {} is not {}",
            n,
            document_id,
            placed,
            remaining,
            net_to_pay
        );
        owed_on_paper = owed_on_paper
            .checked_add(Money::centimes(remaining))
            .unwrap();
    }
    // The ledger is the truth: the papers never ask for more than it says is
    // owed. Two things are allowed on the other side of that comparison, and
    // both are money the ledger holds that no paper ever asked for.
    //
    // A balance below zero is the shop holding money for the customer, which
    // is no document at all, so the papers are compared against nothing owed
    // rather than against a negative. And credit that landed before the
    // document existed — an opening credit, a correction downwards while
    // nothing was unpaid — was never placed on anything and stays
    // unallocated: the sale that comes afterwards is issued asking for its
    // whole net (features.md §3), and the ledger nets the two out where the
    // paper cannot.
    let owed = if balance.is_negative() {
        Money::ZERO
    } else {
        balance
    };
    let unplaced = unallocated_credit(conn, customer);
    prop_assert!(
        owed_on_paper <= owed.checked_add(unplaced).unwrap(),
        "step {}: the documents ask for {:?} against a balance of {:?} and {:?} of credit nothing was placed on",
        n,
        owed_on_paper,
        balance,
        unplaced
    );
    Ok(())
}

/// Money handed over or corrected off that no document took: what a customer
/// paid before there was anything to pay it against.
fn unallocated_credit(conn: &mut SqliteConnection, customer: i32) -> Money {
    #[derive(QueryableByName)]
    struct Sum {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        total: i64,
    }
    let credited = diesel::sql_query(format!(
        "SELECT COALESCE(SUM(credit_centimes), 0) AS total FROM debt_ledger \
         WHERE shop_id = {SHOP} AND customer_id = {customer}"
    ))
    .load::<Sum>(conn)
    .unwrap();
    let placed = diesel::sql_query(format!(
        "SELECT COALESCE(SUM(a.amount_centimes), 0) AS total FROM debt_allocations a \
         JOIN debt_ledger l ON l.id = a.payment_ledger_id \
         WHERE a.shop_id = {SHOP} AND l.customer_id = {customer}"
    ))
    .load::<Sum>(conn)
    .unwrap();
    Money::centimes(credited[0].total - placed[0].total)
}

/// Every issued document of the customer: its id, what it still asks for and
/// what it asked for at issue. Read straight out of the file, so the query
/// the settlement uses cannot hide a document from the invariant.
fn issued_documents(conn: &mut SqliteConnection, customer: i32) -> Vec<(i32, i64, i64)> {
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        id: i32,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        remaining_debt_centimes: i64,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        net_to_pay_centimes: i64,
    }
    diesel::sql_query(format!(
        "SELECT id, remaining_debt_centimes, net_to_pay_centimes FROM documents \
         WHERE shop_id = {SHOP} AND customer_id = {customer} AND status = 'issued'"
    ))
    .load::<Row>(conn)
    .unwrap()
    .into_iter()
    .map(|r| (r.id, r.remaining_debt_centimes, r.net_to_pay_centimes))
    .collect()
}

fn a_customer(conn: &mut SqliteConnection) -> i32 {
    customers::create(
        conn,
        SHOP,
        OWNER,
        NewCustomer {
            name: "Entreprise Benali".to_string(),
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
        },
        None,
    )
    .unwrap()
    .id
}

/// A facture on credit and the ledger row beside it, written by hand rather
/// than sold: what is under test is the settlement, not the sale.
fn a_facture_on_credit(conn: &mut SqliteConnection, customer_id: i32, net: i64, day: u32) -> i32 {
    let net = Money::centimes(net);
    let issued_at = chrono::NaiveDate::from_ymd_opt(2026, 9, day)
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
