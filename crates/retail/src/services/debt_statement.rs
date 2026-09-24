//! A customer's account cut for paper (features.md §2): over a range of days
//! for the statement, and the newest movements for the debt slip. Both read
//! the one running column `services::debt::statement` builds and name the
//! documents it cites, so neither page adds anything up on its way out.
//!
//! Split out of `services::debt` so the ledger and its settlement fit one
//! file under the size ratchet. It reads through the debt service and never
//! its repo, and the debt service does not import it back.

use chrono::NaiveDate;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::money::Money;
use crate::services::debt::{statement, DocumentRef, LedgerLine, StatementEntry};
use crate::services::documents;

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
    let named = documents::kinds_and_numbers(conn, shop_id, &cited)?;
    let entries = inside
        .into_iter()
        .map(|line| StatementEntry {
            document: line.entry.document_id.and_then(|id| {
                named
                    .iter()
                    .find(|(found, _, _, _)| *found == id)
                    .map(|(_, kind, year, number)| DocumentRef {
                        kind: *kind,
                        year: *year,
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
    let named = documents::kinds_and_numbers(conn, shop_id, &cited)?;
    let entries = newest
        .into_iter()
        .map(|line| StatementEntry {
            document: line.entry.document_id.and_then(|id| {
                named
                    .iter()
                    .find(|(found, _, _, _)| *found == id)
                    .map(|(_, kind, year, number)| DocumentRef {
                        kind: *kind,
                        year: *year,
                        number: *number,
                    })
            }),
            entry: line.entry,
            balance_after: line.balance_after,
        })
        .collect();
    Ok(RecentStatement { balance, entries })
}
