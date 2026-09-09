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

use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::debt::{DebtAllocationRowWrite, DebtRowWrite};
use crate::money::Money;
use crate::repos::customers as customers_repo;
use crate::repos::debt as repo;
use crate::repos::documents as documents_repo;
use crate::services::{audit, optional_field};

pub use crate::models::debt::{
    DebtAllocation, DebtEntry, DebtKind, NewDebtAllocation, NewDebtEntry,
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

/// What an adjustment left behind: the movement, and what the customer owes
/// now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adjusted {
    pub entry: DebtEntry,
    pub balance: Money,
}

/// Corrects a balance by writing a movement, never by editing one
/// (features.md §2). A positive amount raises what the customer owes, a
/// negative one lowers it; zero is refused, because a correction of nothing
/// is an empty form somebody submitted.
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
        let after = repo::balance(conn, shop_id, customer_id)?;
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
                        "balance_centimes": after.as_centimes(),
                        "amount_centimes": amount.as_centimes(),
                        "ledger_id": entry.id,
                        "note": entry.note,
                    })
                    .to_string(),
                ),
            },
        )?;
        Ok(Adjusted {
            entry,
            balance: after,
        })
    })
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
