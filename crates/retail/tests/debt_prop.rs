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
use dzpos_retail::error::{CoreError, RetailError};
use dzpos_retail::money::{Bps, Money, PaymentMode, Regime, Totals, TvaLine};
use dzpos_retail::services::avoir::{self, AvoirLine};
use dzpos_retail::services::customers::{self, PartyKind};
use dzpos_retail::services::debt::{self, DebtKind, NewDebtEntry, PaymentMethod};
use dzpos_retail::services::debt_order;
use dzpos_retail::services::documents::{
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
    fn the_papers_never_ask_for_more_than_the_ledger_says_is_owed(
        opening in prop_oneof![Just(0i64), (1i64..=20).prop_map(|d| d * 10_000)],
        steps in steps(),
    ) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let mut conn = dzpos_retail::db::open(&path).unwrap();
        let customer = a_customer(&mut conn, opening);
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
                        // Every centime of the payment is placed exactly once:
                        // on a paper, or on debt no paper carries. None is
                        // lost and none is made up on the way.
                        Ok(paid) => {
                            let mut placed = paid.without_document;
                            for allocation in &paid.allocations {
                                prop_assert!(allocation.amount > Money::ZERO);
                                placed = placed.checked_add(allocation.amount).unwrap();
                            }
                            prop_assert_eq!(placed, Money::centimes(centimes), "step {}", n);
                            prop_assert!(!paid.without_document.is_negative(), "step {}", n);
                        }
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

/// The one customer these properties need, carrying `opening` centimes of
/// debt from before the app when it is above zero. No identifiers: nothing
/// here issues a facture.
fn a_customer(conn: &mut SqliteConnection, opening: i64) -> i32 {
    customers::create(
        conn,
        SHOP,
        OWNER,
        common::a_fiche("Entreprise Benali"),
        (opening > 0).then_some(Money::centimes(opening)),
    )
    .unwrap()
    .id
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

/// A document's id and what it still asks for, for the planner's properties.
fn papers() -> impl Strategy<Value = Vec<(i32, Money)>> {
    prop::collection::vec(0i64..=1_000_000, 0..=6).prop_map(|asks| {
        asks.into_iter()
            .zip(1..)
            .map(|(a, id)| (id, Money::centimes(a)))
            .collect()
    })
}

proptest! {
    /// The order itself, without a file: whatever the amount, the opening
    /// debt and the papers, the plan places every centime exactly once, never
    /// more on anything than it asks for, and fills the opening debt and then
    /// each paper before it reaches the next.
    #[test]
    fn a_settlement_neither_loses_nor_invents_a_centime(
        amount in 0i64..=10_000_000,
        opening_left in 0i64..=2_000_000,
        papers in papers(),
    ) {
        let amount = Money::centimes(amount);
        let opening_left = Money::centimes(opening_left);
        let plan = debt_order::plan(amount, opening_left, &papers).unwrap();

        let mut total = plan.opening.checked_add(plan.beyond).unwrap();
        for (_, take) in &plan.documents {
            total = total.checked_add(*take).unwrap();
        }
        prop_assert_eq!(total, amount, "{:?}", plan);
        prop_assert!(plan.opening <= opening_left);
        prop_assert!(!plan.beyond.is_negative());

        // The opening debt is filled before any paper sees a centime.
        if !plan.documents.is_empty() {
            prop_assert_eq!(plan.opening, opening_left);
        }
        // Each paper is taken to no more than it asks, in the order offered,
        // and one is only reached once every paper before it is full.
        let mut reached = 0;
        for (i, (id, asks)) in papers.iter().enumerate() {
            let take = plan
                .documents
                .iter()
                .find(|(d, _)| d == id)
                .map_or(Money::ZERO, |(_, t)| *t);
            prop_assert!(take <= *asks);
            if take > Money::ZERO {
                let earlier_full = papers[reached..i].iter().all(|(earlier, earlier_asks)| {
                    plan.documents
                        .iter()
                        .find(|(d, _)| d == earlier)
                        .map_or(Money::ZERO, |(_, t)| *t)
                        == *earlier_asks
                });
                prop_assert!(earlier_full, "paper {} was reached before an older one was full", id);
                reached = i;
            }
        }
        // Money is left over only when nothing was left to take it.
        if plan.beyond > Money::ZERO {
            prop_assert_eq!(plan.opening, opening_left);
            for (id, asks) in &papers {
                let take = plan
                    .documents
                    .iter()
                    .find(|(d, _)| d == id)
                    .map_or(Money::ZERO, |(_, t)| *t);
                prop_assert_eq!(take, *asks);
            }
        }
    }

    /// What the opening debt is still owed never passes what it was, never
    /// goes below zero, and never passes what the balance owes beyond the
    /// papers and the corrections upwards.
    #[test]
    fn the_opening_debt_owed_stays_inside_what_was_opened_and_what_is_owed(
        balance in -1_000_000i64..=5_000_000,
        on_paper in 0i64..=2_000_000,
        opening in 0i64..=2_000_000,
        raised in 0i64..=1_000_000,
    ) {
        let left = debt_order::opening_outstanding(
            Money::centimes(balance),
            Money::centimes(on_paper),
            Money::centimes(opening),
            Money::centimes(raised),
        )
        .unwrap();
        prop_assert!(!left.is_negative());
        prop_assert!(left <= Money::centimes(opening));
        prop_assert!(left.as_centimes() <= (balance - on_paper - raised).max(0));
    }
}

/// The top of the range is an error and not a panic, on both functions.
#[test]
fn a_settlement_at_the_edge_of_the_range_is_refused_rather_than_wrapped() {
    let overflow = debt_order::opening_outstanding(
        Money::centimes(i64::MIN),
        Money::centimes(1),
        Money::centimes(1),
        Money::ZERO,
    );
    assert!(matches!(overflow, Err(CoreError::Money(_))), "{overflow:?}");

    // The largest amount there is, against papers that cannot take it all,
    // still sums back to itself.
    let plan = debt_order::plan(
        Money::centimes(i64::MAX),
        Money::centimes(i64::MAX - 1),
        &[(1, Money::centimes(1)), (2, Money::centimes(i64::MAX))],
    )
    .unwrap();
    assert_eq!(plan.opening, Money::centimes(i64::MAX - 1));
    assert_eq!(plan.documents, [(1, Money::centimes(1))]);
    assert_eq!(plan.beyond, Money::ZERO);

    // A negative figure anywhere is refused: a negative remaining would turn
    // a take into a gift.
    for (amount, opening, asks) in [(-1, 0, 0), (1, -1, 0), (1, 0, -1)] {
        assert!(debt_order::plan(
            Money::centimes(amount),
            Money::centimes(opening),
            &[(1, Money::centimes(asks))],
        )
        .is_err());
    }
}

/// `fixtures/money/debt_payment_order.json`, case by case (features.md §2,
/// Payments; Samir's ruling on T33/T34, 2026-09-24): the opening debt first,
/// then the papers oldest first, then what no paper carries. Every figure in
/// the file was written by hand from the rule.
#[test]
fn debt_payment_order() {
    let path = format!(
        "{}/../../fixtures/money/debt_payment_order.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let cents = |v: &serde_json::Value| v.as_i64().unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let mut conn = dzpos_retail::db::open(dir.path().join("t.db")).unwrap();
        // The opening debt is the fiche's own, written when it is opened.
        let opening = case["steps"][0].get("open").map_or(0, cents);
        let customer = a_customer(&mut conn, opening);
        let mut papers: Vec<i32> = Vec::new();
        let expected_allocations = |step: &serde_json::Value, papers: &[i32]| {
            step["allocations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|pair| {
                    let index = usize::try_from(pair[0].as_u64().unwrap()).unwrap();
                    (papers[index], Money::centimes(cents(&pair[1])))
                })
                .collect::<Vec<(i32, Money)>>()
        };
        for step in case["steps"].as_array().unwrap() {
            if step.get("open").is_some() {
                continue;
            } else if let Some(net) = step.get("sell") {
                let day = u32::try_from(step["day"].as_u64().unwrap()).unwrap();
                papers.push(a_facture_on_credit(&mut conn, customer, cents(net), day));
            } else if let Some(amount) = step.get("pay") {
                let day = u32::try_from(step["day"].as_u64().unwrap()).unwrap();
                let paid = debt::pay(
                    &mut conn,
                    SHOP,
                    OWNER,
                    customer,
                    Money::centimes(cents(amount)),
                    PaymentMethod::Cash,
                    None,
                    chrono::NaiveDate::from_ymd_opt(2026, 9, day)
                        .and_then(|d| d.and_hms_opt(16, 30, 0))
                        .unwrap(),
                )
                .unwrap();
                assert_eq!(
                    paid.allocations
                        .iter()
                        .map(|a| (a.document_id, a.amount))
                        .collect::<Vec<(i32, Money)>>(),
                    expected_allocations(step, &papers),
                    "{name}: the payment settled the wrong papers"
                );
                assert_eq!(
                    paid.without_document,
                    Money::centimes(cents(&step["without_document"])),
                    "{name}: what the payment settled on no paper"
                );
                // The same figure read back off the ledger, which is where
                // the fiche's list of payments gets it.
                let read_back = debt::payments(&mut conn, SHOP, customer).unwrap();
                assert_eq!(read_back[0].without_document, paid.without_document);
            } else if let Some(amount) = step.get("correct") {
                let corrected = debt::adjust(
                    &mut conn,
                    SHOP,
                    OWNER,
                    customer,
                    Money::centimes(cents(amount)),
                    None,
                )
                .unwrap();
                if step.get("allocations").is_some() {
                    assert_eq!(
                        corrected
                            .allocations
                            .iter()
                            .map(|a| (a.document_id, a.amount))
                            .collect::<Vec<(i32, Money)>>(),
                        expected_allocations(step, &papers),
                        "{name}: the correction settled the wrong papers"
                    );
                }
            }
        }
        let remaining: Vec<i64> = papers
            .iter()
            .map(|id| {
                documents::get(&mut conn, SHOP, *id)
                    .unwrap()
                    .balance
                    .map_or(0, |b| b.remaining_debt.as_centimes())
            })
            .collect();
        let expected: Vec<i64> = case["remaining"]
            .as_array()
            .unwrap()
            .iter()
            .map(cents)
            .collect();
        assert_eq!(remaining, expected, "{name}: what the papers still ask for");
        assert_eq!(
            debt::balance(&mut conn, SHOP, customer).unwrap(),
            Money::centimes(cents(&case["balance"])),
            "{name}: the balance"
        );
    }
}
