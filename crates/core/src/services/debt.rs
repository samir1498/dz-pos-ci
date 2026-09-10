//! What a customer owes (features.md §2). The ledger is append-only and the
//! balance is its sum, so there is nothing here that edits or deletes a
//! movement: a mistake is corrected by an `adjustment` row a comptable can
//! read, and a customer's fiche being edited never touches the ledger.
//! `adjust` below is that correction, and it is the one write here that opens
//! a transaction of its own, because the movement and its audit entry are one
//! change.
//!
//! `append` runs in whatever transaction the caller opened, and does not open
//! one of its own: a sale on credit writes the document, the lines, the stock
//! and the debt together or writes none of them. The one caller in this
//! milestone that has no transaction of its own is `customers::create`, and
//! it opens one.

use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime};
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::debt::{DebtAllocationRowWrite, DebtRowWrite};
use crate::models::document::DocumentKind;
use crate::money::Money;
use crate::repos::customers as customers_repo;
use crate::repos::debt as repo;
use crate::repos::documents as documents_repo;
use crate::services::{audit, clock, optional_field};

pub use crate::models::debt::{
    DebtAllocation, DebtEntry, DebtKind, NewDebtAllocation, NewDebtEntry, PaymentMethod,
};

/// What the customer owes right now: the sum of the ledger, never a stored
/// number. Below zero is the shop owing the customer after an overpayment.
pub fn balance(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<Money, CoreError> {
    ensure_customer(conn, shop_id, customer_id)?;
    repo::balance(conn, shop_id, customer_id)
}

/// What every customer of the shop owes, by customer id. A customer with no
/// movement is not a key: no rows is no debt, and the caller reads a missing
/// one as nothing owed.
pub fn balances(
    conn: &mut SqliteConnection,
    shop_id: i32,
) -> Result<HashMap<i32, Money>, CoreError> {
    let mut sums = HashMap::new();
    for (customer_id, debit, credit) in repo::balances(conn, shop_id)? {
        let balance = Money::centimes(debit).checked_sub(Money::centimes(credit))?;
        sums.insert(customer_id, balance);
    }
    Ok(sums)
}

/// The customer's movements, newest first.
pub fn ledger(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<Vec<DebtEntry>, CoreError> {
    ensure_customer(conn, shop_id, customer_id)?;
    repo::ledger(conn, shop_id, customer_id)
}

/// One movement and what the customer owed once it had landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerLine {
    pub entry: DebtEntry,
    /// The balance as of this movement: every older movement counted, no
    /// newer one. The newest line's is the balance the customer owes now.
    pub balance_after: Money,
}

/// A customer's ledger as the fiche reads it: the movements newest first,
/// each with the balance as of itself, and the balance the whole thing sums
/// to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub balance: Money,
    pub lines: Vec<LedgerLine>,
}

/// The ledger with its running balance. The column is computed here rather
/// than on the screen that shows it: a second place that works out what a
/// customer owes is a second answer, and the two would part company the day
/// a kind of movement is added.
pub fn statement(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<Statement, CoreError> {
    let entries = ledger(conn, shop_id, customer_id)?;
    let mut running = Money::ZERO;
    let mut lines = Vec::with_capacity(entries.len());
    // Oldest first, which is the only order a running balance can be built
    // in; the answer is turned back the way the fiche reads it below.
    for entry in entries.into_iter().rev() {
        running = running.checked_add(entry.signed()?)?;
        lines.push(LedgerLine {
            entry,
            balance_after: running,
        });
    }
    lines.reverse();
    Ok(Statement {
        balance: running,
        lines,
    })
}

/// The document a movement cites, as a statement prints it: the kind decides
/// the printed prefix and the number is the one the series handed out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentRef {
    pub kind: DocumentKind,
    pub number: i64,
}

/// One movement of a statement: the row, the balance as of it, and the
/// document it cites when it cites one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementEntry {
    pub entry: DebtEntry,
    pub balance_after: Money,
    pub document: Option<DocumentRef>,
}

/// A customer's account over a range of days: what they owed on the morning
/// of `from`, every movement between the two days, and what they owed on the
/// evening of `to`.
///
/// The opening balance is the running balance of the newest movement before
/// the range, and the closing balance is the newest one inside it, so neither
/// is a second sum of the ledger: they are read off the same running column
/// `statement` builds, and a page printing them can add nothing up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangedStatement {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub opening: Money,
    /// Oldest first, which is the order a statement is read in.
    pub entries: Vec<StatementEntry>,
    pub closing: Money,
}

/// The customer's account between two days, both included (features.md §2).
///
/// `to` is inclusive to the end of its day: a range asked for as one day is
/// that day's movements, and a payment taken at 16:30 falls inside a range
/// that ends on the day it was taken. The day a movement is compared by is
/// the day on the shop's calendar, because every row is stamped by the shop's
/// clock (`append_at`) and a document's `issued_at` is too.
pub fn statement_between(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<RangedStatement, CoreError> {
    if from > to {
        return Err(CoreError::validation(
            "to",
            "a range ends on the day it starts or later",
        ));
    }
    let whole = statement(conn, shop_id, customer_id)?;
    let mut opening = Money::ZERO;
    let mut inside = Vec::new();
    // `statement` answers newest first; a statement is read the other way.
    for line in whole.lines.into_iter().rev() {
        let day = line.entry.created_at.date();
        if day < from {
            // The last one before the range is what the customer owed when it
            // opened, so the column is followed rather than summed again.
            opening = line.balance_after;
        } else if day <= to {
            inside.push(line);
        }
    }
    let closing = inside
        .last()
        .map_or(opening, |line: &LedgerLine| line.balance_after);
    let cited: Vec<i32> = inside
        .iter()
        .filter_map(|line| line.entry.document_id)
        .collect();
    let named = documents_repo::kinds_and_numbers(conn, shop_id, &cited)?;
    let entries = inside
        .into_iter()
        .map(|line| StatementEntry {
            document: line.entry.document_id.and_then(|id| {
                named
                    .iter()
                    .find(|(found, _, _)| *found == id)
                    .map(|(_, kind, number)| DocumentRef {
                        kind: *kind,
                        number: *number,
                    })
            }),
            entry: line.entry,
            balance_after: line.balance_after,
        })
        .collect();
    Ok(RangedStatement {
        from,
        to,
        opening,
        entries,
        closing,
    })
}

/// What a customer owes now and the movements that are still worth showing
/// them: the newest `limit` rows, each with the balance as of itself and the
/// document it cites.
///
/// `balance` is the whole ledger's, never the sum of the rows carried here. A
/// slip whose figure was the sum of the ten movements it printed would be
/// wrong for every customer who has bought more than ten times, and wrong in
/// the direction that says they owe less than they do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentStatement {
    pub balance: Money,
    /// Newest first, which is the order a counter paper is read in.
    pub entries: Vec<StatementEntry>,
}

/// The customer's account as a debt slip prints it (features.md §2). The same
/// running column `statement` builds, cut to its newest `limit` rows and with
/// each cited document named, so the slip adds nothing up on its way to paper.
pub fn recent(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
    limit: usize,
) -> Result<RecentStatement, CoreError> {
    let whole = statement(conn, shop_id, customer_id)?;
    let balance = whole.balance;
    let newest: Vec<LedgerLine> = whole.lines.into_iter().take(limit).collect();
    let cited: Vec<i32> = newest
        .iter()
        .filter_map(|line| line.entry.document_id)
        .collect();
    let named = documents_repo::kinds_and_numbers(conn, shop_id, &cited)?;
    let entries = newest
        .into_iter()
        .map(|line| StatementEntry {
            document: line.entry.document_id.and_then(|id| {
                named
                    .iter()
                    .find(|(found, _, _)| *found == id)
                    .map(|(_, kind, number)| DocumentRef {
                        kind: *kind,
                        number: *number,
                    })
            }),
            entry: line.entry,
            balance_after: line.balance_after,
        })
        .collect();
    Ok(RecentStatement { balance, entries })
}

/// What an adjustment left behind: the movement, what it took off the
/// customer's documents, and the ledger as the same transaction read it once
/// the movement had landed. The statement travels with the entry so that the
/// balance the caller answers, the balance the audit records and the balance
/// the ledger sums to are one figure read once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adjusted {
    pub entry: DebtEntry,
    /// Oldest document first, and empty on a correction that raises the debt:
    /// money owed that no paper asks for settles nothing.
    pub allocations: Vec<DebtAllocation>,
    pub statement: Statement,
}

/// Corrects a balance by writing a movement, never by editing one
/// (features.md §2). A positive amount raises what the customer owes, a
/// negative one lowers it; zero is refused, because a correction of nothing
/// is an empty form somebody submitted.
///
/// A closed fiche still takes one: a shop closes a fiche to stop selling to
/// somebody, not to stop correcting what they owe. The sale is what a closed
/// fiche refuses (`services::sales`).
///
/// A correction downwards settles the customer's documents oldest first,
/// through the same allocation a payment goes through (features.md §3): the
/// ledger is what a customer owes, so a document that is no longer owed in
/// full must not go on asking for the whole of it. A correction upwards
/// settles nothing: it is debt no paper carries, like an opening balance.
///
/// The movement and its audit entry are one transaction: a change to what
/// somebody owes with nobody's name on it is exactly what the log exists to
/// prevent (features.md §5). The two balances are stored in the entry so a
/// reader never has to re-derive them from the ledger.
pub fn adjust(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    customer_id: i32,
    amount: Money,
    note: Option<String>,
) -> Result<Adjusted, CoreError> {
    ensure_customer(conn, shop_id, customer_id)?;
    if amount == Money::ZERO {
        return Err(CoreError::validation(
            "amount",
            "a correction of nothing corrects nothing",
        ));
    }
    // Negated through checked arithmetic: `-amount` on the smallest i64 has
    // no positive to negate to.
    let (debit, credit) = if amount.is_negative() {
        (Money::ZERO, Money::ZERO.checked_sub(amount)?)
    } else {
        (amount, Money::ZERO)
    };
    conn.transaction(|conn| {
        let before = repo::balance(conn, shop_id, customer_id)?;
        let entry = append(
            conn,
            shop_id,
            NewDebtEntry {
                customer_id,
                document_id: None,
                kind: DebtKind::Adjustment,
                debit,
                credit,
                user_id,
                note,
            },
        )?;
        // A correction downwards is money off the papers, oldest first, the
        // same way a payment is. Upwards it is debt no document carries, so
        // there is nothing to place.
        let allocations = if amount.is_negative() {
            settle_oldest_first(conn, shop_id, customer_id, entry.id, credit)?
        } else {
            Vec::new()
        };
        // Read once, inside the transaction: what goes into the log below is
        // the same figure the caller is handed.
        let after = statement(conn, shop_id, customer_id)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_ADJUST_DEBT,
                entity: "customer_debt",
                entity_id: Some(customer_id),
                before: Some(
                    serde_json::json!({ "balance_centimes": before.as_centimes() }).to_string(),
                ),
                after: Some(
                    serde_json::json!({
                        "balance_centimes": after.balance.as_centimes(),
                        "amount_centimes": amount.as_centimes(),
                        "ledger_id": entry.id,
                        "note": entry.note,
                        "allocations": allocated_json(&allocations),
                    })
                    .to_string(),
                ),
            },
        )?;
        Ok(Adjusted {
            entry,
            allocations,
            statement: after,
        })
    })
}

/// What a payment left behind: the movement, what it settled and where the
/// balance stood once it had landed. All three are read inside the payment's
/// own transaction, so the figure the caller answers, the figure the audit
/// records and the figure the ledger sums to are one figure read once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payment {
    pub entry: DebtEntry,
    /// Oldest document first, which is the order the money filled them in.
    /// Empty when the customer owes on no document at all: an opening balance
    /// and a correction cite none, and money against those settles the
    /// balance without settling a piece of paper.
    pub allocations: Vec<DebtAllocation>,
    pub balance_after: Money,
}

/// Money against a debt (features.md §2). One transaction: the credit
/// movement, the allocations that say which documents it settled, the
/// remaining debt on each of those documents and the audit entry either all
/// land or none of them does.
///
/// A payment is never more than the customer owes. Money over the debt is not
/// a payment, it is a credit the shop is holding, and the only thing that
/// opens one is an avoir: a payment allowed to overshoot would open one
/// silently, with no document behind it and nothing on the statement saying
/// where it came from.
///
/// A closed fiche still takes one: a shop closes a fiche to stop selling to
/// somebody, not to stop collecting from them, and a customer who owed money
/// on the day their fiche was closed still walks in with it.
///
/// The documents are settled oldest first, each one taken to what is left on
/// it and no further (features.md §2). What a payment cannot place on a
/// document stays on the balance: an opening balance carries no document, and
/// a payment against one is a real payment that settles no paper.
#[allow(clippy::too_many_arguments)]
pub fn pay(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    customer_id: i32,
    amount: Money,
    mode: PaymentMethod,
    note: Option<String>,
    at: NaiveDateTime,
) -> Result<Payment, CoreError> {
    ensure_customer(conn, shop_id, customer_id)?;
    if amount.as_centimes() <= 0 {
        return Err(CoreError::validation(
            "amount_centimes",
            "a payment of nothing pays nothing",
        ));
    }
    let note = optional_field("note", note.as_deref())?;
    conn.transaction(|conn| {
        // Read inside the transaction, because it is what the amount is
        // measured against: a second payment landing between the check and
        // the write would otherwise take the customer into credit.
        let outstanding = repo::balance(conn, shop_id, customer_id)?;
        if amount > outstanding {
            return Err(CoreError::PaymentAboveDebt {
                // Nothing owed, or the shop owing the customer, is nothing
                // that can be paid; the payload says zero rather than a
                // negative a screen would have to explain.
                outstanding_centimes: outstanding.as_centimes().max(0),
            });
        }
        let entry = repo::append(
            conn,
            &DebtRowWrite {
                shop_id,
                customer_id,
                // A payment settles documents through its allocations, which
                // can be several: the column that names one document would
                // have to pick.
                document_id: None,
                kind: DebtKind::Payment,
                debit_centimes: 0,
                credit_centimes: amount.as_centimes(),
                user_id,
                note,
                payment_mode: Some(mode),
                created_at: Some(at),
            },
        )?;
        let allocations = settle_oldest_first(conn, shop_id, customer_id, entry.id, amount)?;
        let after = repo::balance(conn, shop_id, customer_id)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_PAY_DEBT,
                entity: "customer_debt",
                entity_id: Some(customer_id),
                before: Some(
                    serde_json::json!({ "balance_centimes": outstanding.as_centimes() })
                        .to_string(),
                ),
                after: Some(
                    serde_json::json!({
                        "balance_centimes": after.as_centimes(),
                        "amount_centimes": amount.as_centimes(),
                        "payment_mode": mode.as_str(),
                        "ledger_id": entry.id,
                        "allocations": allocated_json(&allocations),
                    })
                    .to_string(),
                ),
            },
        )?;
        Ok(Payment {
            entry,
            allocations,
            balance_after: after,
        })
    })
}

/// What the movement settled, document by document, as the log records it.
/// The amounts and not only the ids: a log saying a facture was settled
/// without saying by how much cannot be read against the facture.
fn allocated_json(allocations: &[DebtAllocation]) -> serde_json::Value {
    serde_json::Value::Array(
        allocations
            .iter()
            .map(|a| {
                serde_json::json!({
                    "document_id": a.document_id,
                    "amount_centimes": a.amount.as_centimes(),
                })
            })
            .collect(),
    )
}

/// One paper a credit movement can be placed on: which document, what it is
/// still asking for, and what it asked for when it was issued.
struct Target {
    document_id: i32,
    remaining: Money,
    net_to_pay: Money,
}

/// Spreads `amount` over `targets` in the order they are given, writing one
/// allocation per document it reaches and the new remaining on each. Runs
/// inside the transaction of whichever movement is settling paper.
///
/// The money can run out before the papers do, and the papers can run out
/// before the money does; both are ordinary. The second one is what happens
/// when part of the balance came from an opening row or a correction, which
/// cite no document at all.
///
/// The guard is written here and only here: Σ of what has been placed on a
/// document, this allocation included, is never above what the document
/// asked for. The remaining column alone would not catch it, because a row
/// written straight into `debt_allocations` moves no column and the document
/// would end up settled twice over with nothing saying so.
fn settle(
    conn: &mut SqliteConnection,
    shop_id: i32,
    payment_ledger_id: i32,
    targets: &[Target],
    amount: Money,
) -> Result<Vec<DebtAllocation>, CoreError> {
    let mut left = amount;
    let mut written = Vec::new();
    for target in targets {
        if left == Money::ZERO {
            break;
        }
        let take = left.min(target.remaining);
        // An allocation of nothing settles nothing and would sit against the
        // document forever.
        if take == Money::ZERO {
            continue;
        }
        let already = allocated_on(conn, shop_id, target.document_id)?;
        if already.checked_add(take)? > target.net_to_pay {
            return Err(CoreError::validation(
                "amount_centimes",
                "a document cannot be settled for more than it asked for",
            ));
        }
        written.push(allocate(
            conn,
            shop_id,
            NewDebtAllocation {
                payment_ledger_id,
                document_id: target.document_id,
                amount: take,
            },
        )?);
        documents_repo::set_remaining_debt(
            conn,
            shop_id,
            target.document_id,
            target.remaining.checked_sub(take)?.as_centimes(),
        )?;
        left = left.checked_sub(take)?;
    }
    Ok(written)
}

/// Spreads `amount` over the customer's unpaid documents, oldest first, and
/// writes back what is left on each. Runs inside the transaction of whichever
/// movement is settling paper: a payment, or a correction downwards.
pub fn settle_oldest_first(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
    payment_ledger_id: i32,
    amount: Money,
) -> Result<Vec<DebtAllocation>, CoreError> {
    let targets: Vec<Target> = documents_repo::unpaid_of_customer(conn, shop_id, customer_id)?
        .into_iter()
        .map(
            |(document_id, remaining_centimes, net_to_pay_centimes)| Target {
                document_id,
                remaining: Money::centimes(remaining_centimes),
                net_to_pay: Money::centimes(net_to_pay_centimes),
            },
        )
        .collect();
    settle(conn, shop_id, payment_ledger_id, &targets, amount)
}

/// Places part of a credit movement on one named document and writes back
/// what is left unpaid on it. Runs inside the caller's transaction.
///
/// `settle_oldest_first` picks the documents by age, which is what a payment
/// wants: the customer hands money over and the oldest paper is filled first.
/// An avoir does not get to pick, because it was written against one facture
/// and the money is going back on that paper whatever its age; only what that
/// facture cannot take spreads by age afterwards.
///
/// An amount of nothing writes no row: an allocation of zero settles nothing
/// and would sit against the document forever.
pub fn settle_document(
    conn: &mut SqliteConnection,
    shop_id: i32,
    payment_ledger_id: i32,
    document_id: i32,
    amount: Money,
) -> Result<Option<DebtAllocation>, CoreError> {
    if amount == Money::ZERO {
        return Ok(None);
    }
    let document = documents_repo::get(conn, shop_id, document_id)?;
    let remaining = document.balance.map_or(Money::ZERO, |b| b.remaining_debt);
    // Refused here rather than quietly taking what fits: a caller that names
    // one document is saying the whole amount belongs on it, and the rest
    // spreading by age is the caller's own next step, not this one's.
    if amount > remaining {
        return Err(CoreError::validation(
            "amount",
            "a document cannot be settled for more than it is still asking for",
        ));
    }
    let written = settle(
        conn,
        shop_id,
        payment_ledger_id,
        &[Target {
            document_id,
            remaining,
            net_to_pay: document.totals.net_to_pay,
        }],
        amount,
    )?;
    // One target and an amount it can take in full, so `settle` wrote
    // exactly one row.
    Ok(written.into_iter().next())
}

/// What the shop is holding for the customer, read off a balance: an amount
/// below zero turned round, and nothing at all when they owe money.
///
/// Turned round through checked arithmetic, because `-balance` on the
/// smallest i64 has no positive to negate to.
pub fn credit_held(balance: Money) -> Result<Money, CoreError> {
    if balance.is_negative() {
        Ok(Money::ZERO.checked_sub(balance)?)
    } else {
        Ok(Money::ZERO)
    }
}

/// Settles a document out of credit the customer was already holding, oldest
/// credit first. Runs inside the caller's transaction, beside the document it
/// is settling.
///
/// This is the other half of `settle_oldest_first`. That one has money and
/// looks for paper; this one has paper and looks for money, which is what a
/// credit sale to a customer in credit needs: the document has to be handed
/// over asking for what is really left on it, and the credit that covers the
/// rest has to say on the ledger which paper it went to.
///
/// The rows are drawn on in the order they arrived, so the oldest credit is
/// the first to be used up. The caller decides the amount, because only the
/// caller knows what the document asked for; what is refused here is being
/// asked for more credit than the customer has, which would place money on
/// paper that the ledger never received.
pub fn settle_from_credit(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
    document_id: i32,
    amount: Money,
) -> Result<Vec<DebtAllocation>, CoreError> {
    if amount == Money::ZERO {
        return Ok(Vec::new());
    }
    let mut left = amount;
    let mut written = Vec::new();
    for (ledger_id, available) in unallocated_credit(conn, shop_id, customer_id)? {
        if left == Money::ZERO {
            break;
        }
        let take = left.min(available);
        written.push(allocate(
            conn,
            shop_id,
            NewDebtAllocation {
                payment_ledger_id: ledger_id,
                document_id,
                amount: take,
            },
        )?);
        left = left.checked_sub(take)?;
    }
    if left != Money::ZERO {
        return Err(CoreError::validation(
            "amount",
            "more credit was asked of this customer's ledger than it holds",
        ));
    }
    Ok(written)
}

/// Every credit movement of the customer that no document has taken whole,
/// with what is left on it, oldest first.
///
/// An avoir past what its facture was still owed leaves credit here, and so
/// does a correction downwards taken while nothing was outstanding. A payment
/// can leave some too, when part of the balance came from an opening row that
/// cites no document at all.
fn unallocated_credit(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<Vec<(i32, Money)>, CoreError> {
    let mut rows = Vec::new();
    // `ledger` answers newest first, and credit is drawn on in the order it
    // arrived: the same reason a payment fills the oldest document first.
    for entry in repo::ledger(conn, shop_id, customer_id)?.into_iter().rev() {
        if entry.credit == Money::ZERO {
            continue;
        }
        let mut placed = Money::ZERO;
        for allocation in repo::allocations_of_payment(conn, shop_id, entry.id)? {
            placed = placed.checked_add(allocation.amount)?;
        }
        let left = entry.credit.checked_sub(placed)?;
        if left != Money::ZERO {
            rows.push((entry.id, left));
        }
    }
    Ok(rows)
}

/// What every payment so far has placed on one document.
fn allocated_on(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
) -> Result<Money, CoreError> {
    let mut sum = Money::ZERO;
    for allocation in repo::allocations(conn, shop_id, document_id)? {
        sum = sum.checked_add(allocation.amount)?;
    }
    Ok(sum)
}

/// The payments of one customer, newest first, each with what it settled.
pub fn payments(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<Vec<Payment>, CoreError> {
    let statement = statement(conn, shop_id, customer_id)?;
    let mut found = Vec::new();
    for line in statement.lines {
        if line.entry.kind != DebtKind::Payment {
            continue;
        }
        let allocations = repo::allocations_of_payment(conn, shop_id, line.entry.id)?;
        found.push(Payment {
            entry: line.entry,
            allocations,
            // The balance as of the payment, which is what the movement left
            // behind; the newest one's is what the customer owes now.
            balance_after: line.balance_after,
        });
    }
    Ok(found)
}

/// Writes one movement. Runs inside the caller's transaction.
///
/// A movement raises the debt or lowers it, never both and never neither: the
/// migration's CHECK refuses the first, and this refuses the second, because
/// a row that moves nothing is a mistake somebody makes with an empty form
/// and it would sit in a statement forever.
pub fn append(
    conn: &mut SqliteConnection,
    shop_id: i32,
    entry: NewDebtEntry,
) -> Result<DebtEntry, CoreError> {
    append_at(conn, shop_id, entry, None)
}

/// The same movement, stamped with the moment it belongs to rather than with
/// the moment it was written. `None` is now, on the shop's clock, which is
/// what every caller ringing something up wants; a caller that knows when the
/// money moved says so, and the statement's date range then reads the day the
/// shop would say it was.
///
/// The column's own `CURRENT_TIMESTAMP` default is never used: it is UTC,
/// while a document's `issued_at` is on the shop's calendar, and one hour a
/// day the two disagree about which day a movement landed on.
pub fn append_at(
    conn: &mut SqliteConnection,
    shop_id: i32,
    entry: NewDebtEntry,
    at: Option<NaiveDateTime>,
) -> Result<DebtEntry, CoreError> {
    ensure_customer(conn, shop_id, entry.customer_id)?;
    // The document a movement cites is checked the same way the allocation's
    // is: a statement that names a document this shop never issued is worse
    // than one that names none.
    if let Some(document_id) = entry.document_id {
        ensure_document(conn, shop_id, document_id)?;
    }
    if entry.debit.is_negative() {
        return Err(CoreError::validation(
            "debit",
            "a movement is written in the direction its column names, never as a negative",
        ));
    }
    if entry.credit.is_negative() {
        return Err(CoreError::validation(
            "credit",
            "a movement is written in the direction its column names, never as a negative",
        ));
    }
    if entry.debit != Money::ZERO && entry.credit != Money::ZERO {
        return Err(CoreError::validation(
            "debit",
            "a movement raises the debt or lowers it, not both",
        ));
    }
    if entry.debit == Money::ZERO && entry.credit == Money::ZERO {
        return Err(CoreError::validation(
            "debit",
            "a movement of nothing moves no debt",
        ));
    }
    let note = optional_field("note", entry.note.as_deref())?;
    repo::append(
        conn,
        &DebtRowWrite {
            shop_id,
            customer_id: entry.customer_id,
            document_id: entry.document_id,
            kind: entry.kind,
            debit_centimes: entry.debit.as_centimes(),
            credit_centimes: entry.credit.as_centimes(),
            user_id: entry.user_id,
            note,
            // A movement that is not a payment was not handed over in
            // anything. `pay` is the one writer that fills the mode in.
            payment_mode: None,
            created_at: Some(at.unwrap_or_else(clock::now)),
        },
    )
}

/// Records that part of a payment settled one document. Runs inside the
/// caller's transaction, next to the `payment` row it belongs to.
pub fn allocate(
    conn: &mut SqliteConnection,
    shop_id: i32,
    allocation: NewDebtAllocation,
) -> Result<DebtAllocation, CoreError> {
    if allocation.amount.as_centimes() <= 0 {
        return Err(CoreError::validation(
            "amount",
            "an allocation of nothing settles nothing",
        ));
    }
    ensure_entry(conn, shop_id, allocation.payment_ledger_id)?;
    ensure_document(conn, shop_id, allocation.document_id)?;
    repo::allocate(
        conn,
        &DebtAllocationRowWrite {
            shop_id,
            payment_ledger_id: allocation.payment_ledger_id,
            document_id: allocation.document_id,
            amount_centimes: allocation.amount.as_centimes(),
        },
    )
}

/// What a payment has settled against one document, oldest first.
pub fn allocations(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
) -> Result<Vec<DebtAllocation>, CoreError> {
    ensure_document(conn, shop_id, document_id)?;
    repo::allocations(conn, shop_id, document_id)
}

/// The foreign key would accept another shop's customer, so the shop is
/// checked here the way a document's product is (rule 3).
/// Both foreign keys an allocation carries would take another shop's row, so
/// each is checked against the shop asking (rule 3).
fn ensure_entry(conn: &mut SqliteConnection, shop_id: i32, entry_id: i32) -> Result<(), CoreError> {
    if repo::entry_belongs_to_shop(conn, shop_id, entry_id)? {
        return Ok(());
    }
    Err(CoreError::NotFound {
        entity: "debt_entry",
        id: entry_id,
    })
}

fn ensure_document(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
) -> Result<(), CoreError> {
    if documents_repo::belongs_to_shop(conn, shop_id, document_id)? {
        return Ok(());
    }
    Err(CoreError::NotFound {
        entity: "document",
        id: document_id,
    })
}

fn ensure_customer(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<(), CoreError> {
    if customers_repo::belongs_to_shop(conn, shop_id, customer_id)? {
        return Ok(());
    }
    Err(CoreError::NotFound {
        entity: "customer",
        id: customer_id,
    })
}
