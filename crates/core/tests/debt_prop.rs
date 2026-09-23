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
use dzpos_core::error::{CoreError, RetailError};
use dzpos_core::money::{Bps, Money, PaymentMode, Regime, Totals, TvaLine};
use dzpos_core::services::avoir::{self, AvoirLine};
use dzpos_core::services::customers::{self, PartyKind};
use dzpos_core::services::debt::{self, DebtKind, NewDebtEntry, PaymentMethod};
use dzpos_core::services::documents::{
    self, BalanceTriple, DocumentKind, DocumentStatus, NewDocument, NewDocumentLine, PartyBlock,
    SellerBlock,
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
    /// A credit note against the newest facture that still stands. `true` is
    /// the whole of what is left on it, `false` is one of its four units.
    Avoir(bool),
}

fn steps() -> impl Strategy<Value = Vec<Step>> {
    let amount = (1i64..=20).prop_map(|d| d * 10_000);
    let step = prop_oneof![
        amount.clone().prop_map(Step::Sell),
        (amount.clone(), any::<bool>()).prop_map(|(a, up)| Step::Correct(if up { a } else { -a })),
        amount.prop_map(Step::Pay),
        any::<bool>().prop_map(Step::Avoir),
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
                        Err(RetailError::PaymentAboveDebt { .. }) => {
                            let after = debt::balance(&mut conn, SHOP, customer).unwrap();
                            prop_assert_eq!(before, after, "a refused payment moved the balance");
                        }
                        Err(other) => prop_assert!(false, "step {}: {:?}", n, other),
                    }
                }
                Step::Avoir(whole) => {
                    day += 1;
                    let before = debt::balance(&mut conn, SHOP, customer).unwrap();
                    match an_avoir(&mut conn, customer, whole, day) {
                        // No facture standing with anything left on it, or one
                        // whose whole value has already come back: the credit
                        // note has nothing to be written against.
                        None | Some((_, Ok(_))) => {}
                        Some((true, Err(e))) => prop_assert!(
                            false,
                            "step {}: the closing avoir was refused: {:?}",
                            n,
                            e
                        ),
                        Some((false, Err(RetailError::Kernel(CoreError::Validation { .. })))) => {
                            let after = debt::balance(&mut conn, SHOP, customer).unwrap();
                            prop_assert_eq!(before, after, "a refused avoir moved the balance");
                        }
                        Some((_, Err(other))) => prop_assert!(false, "step {}: {:?}", n, other),
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
        // Σ of what has been placed on a document never passes what it asked
        // for. It follows from the equality below while the equality holds,
        // and it is stated on its own because it is the thing the guards in
        // `settle_document` and `settle_oldest_first` defend: a row written
        // into `debt_allocations` moves no column, so a file can hold a
        // document settled twice over whose remaining figure still adds up.
        prop_assert!(
            placed <= net_to_pay,
            "step {}: document {} carries allocations of {} against a net of {}",
            n,
            document_id,
            placed,
            net_to_pay
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
    // owed, and now for no more than that at all.
    //
    // The slack this used to allow was credit that landed before the document
    // existed. A correction downwards taken while nothing was outstanding sat
    // unallocated, the sale that followed was issued asking for its whole net,
    // and the two were left to be netted out by whoever read them. A credit
    // sale now consumes that credit at issue (features.md §3), so there is
    // nothing left on the ledger for the paper to disagree with.
    //
    // A balance below zero is the shop holding money for the customer, which
    // is no document at all, so the papers are compared against nothing owed
    // rather than against a negative. Reaching that state means every
    // outstanding document has been filled, which is what makes the zero on
    // this side of the comparison the truth rather than a floor.
    let owed = if balance.is_negative() {
        Money::ZERO
    } else {
        balance
    };
    prop_assert!(
        owed_on_paper <= owed,
        "step {}: the documents ask for {:?} against a balance of {:?}",
        n,
        owed_on_paper,
        balance
    );
    Ok(())
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
    // Not the avoirs. A credit note's stored triple is its own effect on the
    // account, and its middle figure is the negative of its net rather than
    // something the customer is being asked for (features.md §3). It is the
    // papers that ask for money that this invariant is about.
    diesel::sql_query(format!(
        "SELECT id, remaining_debt_centimes, net_to_pay_centimes FROM documents \
         WHERE shop_id = {SHOP} AND customer_id = {customer} AND status = 'issued' \
         AND kind != 'avoir'"
    ))
    .load::<Row>(conn)
    .unwrap()
    .into_iter()
    .map(|r| (r.id, r.remaining_debt_centimes, r.net_to_pay_centimes))
    .collect()
}

mod common;

/// The one customer these properties need. No identifiers: nothing here
/// issues a facture.
fn a_customer(conn: &mut SqliteConnection) -> i32 {
    common::a_customer(conn, "Entreprise Benali")
}

/// A facture on credit and the ledger row beside it, written by hand rather
/// than sold: what is under test is the settlement, not the sale. It goes
/// through the same two calls `services::sales` makes, and in the same order,
/// so the invariant is asserted against the rule the till applies and not
/// against a second one written here.
fn a_facture_on_credit(conn: &mut SqliteConnection, customer_id: i32, net: i64, day: u32) -> i32 {
    let net = Money::centimes(net);
    let issued_at = chrono::NaiveDate::from_ymd_opt(2026, 9, day)
        .and_then(|d| d.and_hms_opt(10, 0, 0))
        .unwrap();
    let customer = customers::prove(conn, SHOP, customer_id).unwrap();
    let before = debt::balance(conn, SHOP, customer_id).unwrap();
    // Credit the customer is already holding settles the new document at
    // issue, so what the paper asks for is what is really left on it.
    let consumed = debt::credit_held(before).unwrap().min(net);
    let remaining = net.checked_sub(consumed).unwrap();
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
            customer: Some(customer),
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
                remaining_debt: remaining,
                total_debt: before.checked_add(net).unwrap(),
            }),
            totals: Totals {
                total_ht: net,
                discount: Money::ZERO,
                subtotal_ht: net,
                // A réel facture carries a recap row for every rate its lines
                // are at, exempt ones included. Without the row at 0 % the
                // first avoir writes a rate the facture does not carry, and
                // every closing avoir after it is refused for it.
                tva_by_rate: vec![TvaLine {
                    rate: Bps::ZERO,
                    base: net,
                    amount: Money::ZERO,
                }],
                tva: Money::ZERO,
                total_ttc: net,
                stamp: Money::ZERO,
                net_to_pay: net,
            },
            tendered: None,
            change: None,
            // Four units of one nameless article, because an avoir credits
            // lines and a facture with none of them cannot be credited at all.
            // A quarter of the net apiece, which every amount the generator
            // makes divides into exactly.
            lines: vec![NewDocumentLine {
                product_id: None,
                name: "Article".to_string(),
                barcode: None,
                qty_milli: 4_000,
                unit_price: Money::centimes(net.as_centimes() / 4),
                line_discount: Money::ZERO,
                rate_bps: Bps::ZERO,
                line_total: net,
                ref_line_id: None,
            }],
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
    debt::settle_from_credit(conn, SHOP, customer_id, doc.id, consumed).unwrap();
    doc.id
}

/// A credit note against the newest facture of the customer that still stands.
/// `None` when there is no such facture, which is a step the generator is free
/// to produce and which is not a failure.
///
/// A refusal is an answer here: a facture whose value has already come back in
/// full refuses the next avoir, and what the invariant cares about is that the
/// refusal left the file alone.
fn an_avoir(
    conn: &mut SqliteConnection,
    customer_id: i32,
    whole: bool,
    day: u32,
) -> Option<(bool, Result<i32, RetailError>)> {
    let issued_at = chrono::NaiveDate::from_ymd_opt(2026, 9, day)
        .and_then(|d| d.and_hms_opt(11, 0, 0))
        .unwrap();
    let facture = documents::list(conn, SHOP, Some(DocumentKind::Facture))
        .unwrap()
        .into_iter()
        .rfind(|d| d.customer_id == Some(customer_id) && d.status == DocumentStatus::Issued)?;
    let line = facture.lines.first()?;
    let taken: i64 = avoir::list_for(conn, SHOP, facture.id)
        .unwrap()
        .iter()
        .flat_map(|a| a.lines.clone())
        .filter(|l| l.ref_line_id == Some(line.id))
        .map(|l| l.qty_milli)
        .sum();
    let left = line.qty_milli - taken;
    // The one that takes the last quantity off the facture is computed as the
    // subtraction, and that is the one that must never be refused: a facture
    // nobody can finish crediting is a facture nobody can annul.
    let closing = left > 0 && (whole || left == 1_000);
    let lines = if whole {
        None
    } else {
        Some(vec![AvoirLine {
            document_line_id: line.id,
            qty_milli: 1_000,
        }])
    };
    Some((
        closing,
        avoir::issue(conn, SHOP, OWNER, facture.id, lines, None, Some(issued_at)).map(|a| a.id),
    ))
}
