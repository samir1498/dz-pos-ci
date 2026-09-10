//! What the shop owes its suppliers (features.md §1). The mirror of
//! `services::debt` on the supply side: the ledger is append-only and the
//! balance is its sum, so there is nothing here that edits or deletes a
//! movement. A mistake is corrected by an `adjustment` row a comptable can
//! read, and editing a supplier's fiche never touches the ledger.
//!
//! Two ledgers, one shape, and the duplication is on purpose (plan lens,
//! 2026-09-10): one `parties` table with a role would have made every
//! customer-keyed query, CHECK, index and screen party-keyed for a party that
//! never buys at the till. What is not duplicated is the settlement itself,
//! which differs where the two sides really differ: a document carries a
//! stored `remaining_debt` column a payment writes back, while what is still
//! owed on a purchase is derived here from the ledger and the allocations, so
//! there is no column to write and no column that can disagree with the sum.
//!
//! `append` runs in whatever transaction the caller opened and does not open
//! one of its own: T3's receipt writes the stock, the line and the ledger row
//! together or writes none of them. `pay` and `adjust` open one, because the
//! movement and its audit entry are one change.

use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime};
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::supplier_debt::{SupplierAllocationRowWrite, SupplierDebtRowWrite};
use crate::money::Money;
use crate::repos::purchases as purchases_repo;
use crate::repos::supplier_debt as repo;
use crate::repos::suppliers as suppliers_repo;
use crate::services::{audit, clock, optional_field};

pub use crate::models::supplier_debt::{
    NewSupplierAllocation, NewSupplierEntry, PaymentMethod, SupplierAllocation, SupplierDebtKind,
    SupplierEntry,
};

/// What the shop owes this supplier right now: the sum of the ledger, never a
/// stored number. Below zero is the supplier owing the shop after an advance
/// or a return past what was due.
pub fn balance(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<Money, CoreError> {
    ensure_supplier(conn, shop_id, supplier_id)?;
    repo::balance(conn, shop_id, supplier_id)
}

/// What the shop owes every supplier, by supplier id. A supplier with no
/// movement is not a key: no rows is no debt, and the caller reads a missing
/// one as nothing owed.
pub fn balances(
    conn: &mut SqliteConnection,
    shop_id: i32,
) -> Result<HashMap<i32, Money>, CoreError> {
    let mut sums = HashMap::new();
    for (supplier_id, debit, credit) in repo::balances(conn, shop_id)? {
        let balance = Money::centimes(debit).checked_sub(Money::centimes(credit))?;
        sums.insert(supplier_id, balance);
    }
    Ok(sums)
}

/// The supplier's movements, newest first.
pub fn ledger(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<Vec<SupplierEntry>, CoreError> {
    ensure_supplier(conn, shop_id, supplier_id)?;
    repo::ledger(conn, shop_id, supplier_id)
}

/// One movement and what the shop owed once it had landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerLine {
    pub entry: SupplierEntry,
    /// The balance as of this movement: every older movement counted, no
    /// newer one. The newest line's is the balance the shop owes now.
    pub balance_after: Money,
}

/// A supplier's ledger as the fiche reads it: the movements newest first,
/// each with the balance as of itself, and the balance the whole thing sums
/// to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub balance: Money,
    pub lines: Vec<LedgerLine>,
}

/// The ledger with its running balance, computed here rather than on the
/// screen that shows it: a second place that works out what the shop owes is
/// a second answer, and the two would part company the day a kind of movement
/// is added.
pub fn statement(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<Statement, CoreError> {
    let entries = ledger(conn, shop_id, supplier_id)?;
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

/// A supplier's account over a range of days: what the shop owed on the
/// morning of `from`, every movement between the two days, and what it owed
/// on the evening of `to`.
///
/// The opening balance is the running balance of the newest movement before
/// the range and the closing balance is the newest one inside it, so neither
/// is a second sum of the ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangedStatement {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub opening: Money,
    /// Oldest first, which is the order a statement is read in.
    pub entries: Vec<LedgerLine>,
    pub closing: Money,
}

/// The supplier's account between two days, both included.
///
/// `to` is inclusive to the end of its day: a range asked for as one day is
/// that day's movements. The day a movement is compared by is the day on the
/// shop's calendar, because every row is stamped by the shop's clock.
pub fn statement_between(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<RangedStatement, CoreError> {
    if from > to {
        return Err(CoreError::validation(
            "to",
            "a range ends on the day it starts or later",
        ));
    }
    let whole = statement(conn, shop_id, supplier_id)?;
    let mut opening = Money::ZERO;
    let mut inside = Vec::new();
    // `statement` answers newest first; a statement is read the other way.
    for line in whole.lines.into_iter().rev() {
        let day = line.entry.created_at.date();
        if day < from {
            // The last one before the range is what the shop owed when it
            // opened, so the column is followed rather than summed again.
            opening = line.balance_after;
        } else if day <= to {
            inside.push(line);
        }
    }
    let closing = inside
        .last()
        .map_or(opening, |line: &LedgerLine| line.balance_after);
    Ok(RangedStatement {
        from,
        to,
        opening,
        entries: inside,
        closing,
    })
}

/// One order the shop still owes on: which purchase, what it is still asking
/// for, and what it was worth on the ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenPurchase {
    pub purchase_id: i32,
    /// What is left on it: its value on the ledger less everything placed on
    /// it. Always above zero here; an order nothing is owed on is left out.
    pub remaining: Money,
    /// What the order is worth on the ledger: the debits a receipt wrote for
    /// it, less the credits a return took off it.
    pub value: Money,
}

/// The supplier's orders that are still asking to be paid, oldest first
/// (features.md §2, oldest-first settlement). Three queries whatever the
/// number of orders: the orders in date order, the ledger summed per order,
/// and the allocations summed per order.
///
/// What is owed on an order is derived and never stored: value on the ledger
/// less what has been placed on it. A purchase nothing ever arrived against
/// has no ledger row and so is not here at all, which is what a cancelled
/// order is.
pub fn open_purchases(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<Vec<OpenPurchase>, CoreError> {
    ensure_supplier(conn, shop_id, supplier_id)?;
    let order = purchases_repo::of_supplier(conn, shop_id, supplier_id)?;
    let sums = repo::sums_by_purchase(conn, shop_id, supplier_id)?;
    let placed = repo::allocated_by_purchase(conn, shop_id, &order)?;
    let mut open = Vec::new();
    for purchase_id in order {
        let Some((_, debit, credit)) = sums.iter().find(|(id, _, _)| *id == purchase_id) else {
            // No movement cites this order, so nothing is owed on it: the
            // goods have not arrived, or never will.
            continue;
        };
        let value = Money::centimes(*debit).checked_sub(Money::centimes(*credit))?;
        let already = placed
            .iter()
            .find(|(id, _)| *id == purchase_id)
            .map_or(Money::ZERO, |(_, amount)| Money::centimes(*amount));
        let remaining = value.checked_sub(already)?;
        if remaining.as_centimes() > 0 {
            open.push(OpenPurchase {
                purchase_id,
                remaining,
                value,
            });
        }
    }
    Ok(open)
}

/// What a payment left behind: the movement, what it settled and where the
/// balance stood once it had landed. All three are read inside the payment's
/// own transaction, so the figure the caller answers, the figure the audit
/// records and the figure the ledger sums to are one figure read once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payment {
    pub entry: SupplierEntry,
    /// Oldest order first, which is the order the money filled them in.
    /// Empty when no order is owed on at all: an opening balance and a
    /// correction cite none, and money against those settles the balance
    /// without settling a piece of paper.
    pub allocations: Vec<SupplierAllocation>,
    pub balance_after: Money,
}

/// Money to a supplier, in cash or by card. One transaction: the credit
/// movement, the allocations that say which orders it settled and the audit
/// entry either all land or none of them does.
///
/// A payment is never more than the shop owes. Money over the debt is an
/// advance sitting with the supplier, which is a movement somebody writes on
/// purpose rather than something a payment opens silently with no paper
/// behind it.
///
/// A closed fiche still takes one: a shop closes a fiche to stop buying from
/// somebody, not to stop paying what it already owes them.
///
/// The orders are settled oldest first, each one taken to what is left on it
/// and no further. What a payment cannot place on an order stays on the
/// balance: an opening balance carries no order.
#[allow(clippy::too_many_arguments)]
pub fn pay(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    supplier_id: i32,
    amount: Money,
    mode: PaymentMethod,
    note: Option<String>,
    at: NaiveDateTime,
) -> Result<Payment, CoreError> {
    ensure_supplier(conn, shop_id, supplier_id)?;
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
        // the write would otherwise take the supplier into advance.
        let outstanding = repo::balance(conn, shop_id, supplier_id)?;
        if amount > outstanding {
            return Err(CoreError::PaymentAboveDebt {
                // Nothing owed, or the supplier owing the shop, is nothing
                // that can be paid; the payload says zero rather than a
                // negative a screen would have to explain.
                outstanding_centimes: outstanding.as_centimes().max(0),
            });
        }
        let entry = repo::append(
            conn,
            &SupplierDebtRowWrite {
                shop_id,
                supplier_id,
                // A payment settles orders through its allocations, which can
                // be several: the column that names one order would have to
                // pick.
                purchase_id: None,
                kind: SupplierDebtKind::Payment,
                debit_centimes: 0,
                credit_centimes: amount.as_centimes(),
                user_id,
                note,
                payment_mode: Some(mode),
                // Never left to the column's own default, which is
                // CURRENT_TIMESTAMP and so UTC: the cash position reads the
                // cash payments of one day off this column on the shop's
                // calendar, and one hour a day the two disagree about which
                // day the money left the drawer. `append_at` does the same
                // for every other kind.
                created_at: Some(at),
            },
        )?;
        let allocations = settle_oldest_first(conn, shop_id, supplier_id, entry.id, amount)?;
        let after = repo::balance(conn, shop_id, supplier_id)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_PAY_SUPPLIER,
                entity: "supplier_debt",
                entity_id: Some(supplier_id),
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

/// What an adjustment left behind: the movement, what it took off the orders,
/// and the ledger as the same transaction read it once the movement had
/// landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adjusted {
    pub entry: SupplierEntry,
    /// Oldest order first, and empty on a correction that raises the debt:
    /// money owed that no order asks for settles nothing.
    pub allocations: Vec<SupplierAllocation>,
    pub statement: Statement,
}

/// Corrects a balance by writing a movement, never by editing one. A positive
/// amount raises what the shop owes, a negative one lowers it; zero is
/// refused, because a correction of nothing is an empty form somebody
/// submitted.
///
/// A closed fiche still takes one: a shop closes a fiche to stop buying from
/// somebody, not to stop correcting what it owes them.
///
/// A correction downwards settles the orders oldest first, through the same
/// allocation a payment goes through: the ledger is what the shop owes, so an
/// order that is no longer owed in full must not go on asking for the whole
/// of it. A correction upwards settles nothing: it is debt no order carries,
/// like an opening balance.
pub fn adjust(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    supplier_id: i32,
    amount: Money,
    note: Option<String>,
) -> Result<Adjusted, CoreError> {
    ensure_supplier(conn, shop_id, supplier_id)?;
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
        let before = repo::balance(conn, shop_id, supplier_id)?;
        let entry = append(
            conn,
            shop_id,
            NewSupplierEntry {
                supplier_id,
                purchase_id: None,
                kind: SupplierDebtKind::Adjustment,
                debit,
                credit,
                user_id,
                note,
            },
        )?;
        let allocations = if amount.is_negative() {
            settle_oldest_first(conn, shop_id, supplier_id, entry.id, credit)?
        } else {
            Vec::new()
        };
        // Read once, inside the transaction: what goes into the log below is
        // the same figure the caller is handed.
        let after = statement(conn, shop_id, supplier_id)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_ADJUST_SUPPLIER,
                entity: "supplier_debt",
                entity_id: Some(supplier_id),
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

/// What the movement settled, order by order, as the log records it. The
/// amounts and not only the ids: a log saying an order was settled without
/// saying by how much cannot be read against the order.
fn allocated_json(allocations: &[SupplierAllocation]) -> serde_json::Value {
    serde_json::Value::Array(
        allocations
            .iter()
            .map(|a| {
                serde_json::json!({
                    "purchase_id": a.purchase_id,
                    "amount_centimes": a.amount.as_centimes(),
                })
            })
            .collect(),
    )
}

/// Spreads `amount` over the supplier's open orders, oldest first, writing
/// one allocation per order it reaches. Runs inside the transaction of
/// whichever movement is settling paper: a payment, or a correction
/// downwards.
///
/// The money can run out before the orders do, and the orders can run out
/// before the money does; both are ordinary. The second is what happens when
/// part of the balance came from an opening row or a correction, which cite
/// no order at all.
///
/// `services::debt::settle` guards each document against being settled past
/// what it asked for, because there the remaining is a stored column a row
/// written straight into the allocations would not move. Here `remaining`
/// is computed from the ledger and the allocations at the moment of reading,
/// inside this transaction, so it already counts every row ever placed on the
/// order and there is no second figure that could disagree with it.
pub fn settle_oldest_first(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
    payment_ledger_id: i32,
    amount: Money,
) -> Result<Vec<SupplierAllocation>, CoreError> {
    let mut left = amount;
    let mut written = Vec::new();
    for target in open_purchases(conn, shop_id, supplier_id)? {
        if left == Money::ZERO {
            break;
        }
        let take = left.min(target.remaining);
        // An allocation of nothing settles nothing and would sit against the
        // order forever.
        if take == Money::ZERO {
            continue;
        }
        written.push(allocate(
            conn,
            shop_id,
            NewSupplierAllocation {
                payment_ledger_id,
                purchase_id: target.purchase_id,
                amount: take,
            },
        )?);
        left = left.checked_sub(take)?;
    }
    Ok(written)
}

/// What the supplier is holding for the shop, read off a balance: an amount
/// below zero turned round, and nothing at all when the shop owes money.
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

/// Places credit the shop was already holding on one order, oldest credit
/// first. Runs inside the caller's transaction, beside the `purchase` row
/// that order has just been given.
///
/// This is the supply side of `debt::settle_from_credit`, and the reason it
/// exists is the same: credit is consumed at issue (M2 ruling, features.md
/// §3). An advance the shop paid, or a correction past what was owed, is
/// money the supplier is already holding, and an order written afterwards has
/// to be settled out of it. Without this the order goes on asking for its
/// whole value while the shop owes less than that, a payment of what is
/// really owed settles a different order, and the first one stays open
/// forever and keeps the fiche from closing without a reason.
///
/// **`services::purchases` calls this after every `purchase` row its receipt
/// path appends**, handing over the value of that row. The row is not
/// appended here because a receipt writes stock, a line and a ledger row in
/// one transaction of its own, and this is the last step of that transaction
/// rather than a second one.
///
/// What is placed is what was held before that row landed: the balance as it
/// stands now, less the row's own value, turned round. The rest of a
/// correction, the part that answered an opening balance or an older order,
/// is not credit at all; placing that too would show the order settled with
/// money that never went to it.
///
/// `just_appended` is the row's value and not the order's, because an order
/// received in parts carries one `purchase` row per delivery: the order's
/// whole value would count the earlier deliveries as if they had only just
/// been owed, and the second delivery would go looking for credit the ledger
/// had already spent on the first.
pub fn place_credit_on(
    conn: &mut SqliteConnection,
    shop_id: i32,
    purchase_id: i32,
    just_appended: Money,
) -> Result<Vec<SupplierAllocation>, CoreError> {
    let purchase = purchases_repo::get(conn, shop_id, purchase_id)?;
    let supplier_id = purchase.supplier_id;
    // An order nothing is owed on is not in this list, and an order nothing
    // has arrived against is not either: neither has anything to settle.
    let Some(target) = open_purchases(conn, shop_id, supplier_id)?
        .into_iter()
        .find(|open| open.purchase_id == purchase_id)
    else {
        return Ok(Vec::new());
    };
    let balance = repo::balance(conn, shop_id, supplier_id)?;
    let before = balance.checked_sub(just_appended)?;
    let take = credit_held(before)?.min(target.remaining);
    if take == Money::ZERO {
        return Ok(Vec::new());
    }
    let mut left = take;
    let mut written = Vec::new();
    for (ledger_id, available) in unallocated_credit(conn, shop_id, supplier_id)? {
        if left == Money::ZERO {
            break;
        }
        let amount = left.min(available);
        written.push(allocate(
            conn,
            shop_id,
            NewSupplierAllocation {
                payment_ledger_id: ledger_id,
                purchase_id,
                amount,
            },
        )?);
        left = left.checked_sub(amount)?;
    }
    if left != Money::ZERO {
        return Err(CoreError::validation(
            "amount",
            "more credit was asked of this supplier's ledger than it holds",
        ));
    }
    Ok(written)
}

/// Every credit movement of the supplier that no order has taken whole, with
/// what is left on it, oldest first: the order credit is drawn on is the
/// order it arrived in, the same reason a payment fills the oldest order
/// first.
fn unallocated_credit(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<Vec<(i32, Money)>, CoreError> {
    let mut rows = Vec::new();
    // `ledger` answers newest first, and credit is drawn on in the order it
    // arrived.
    for entry in repo::ledger(conn, shop_id, supplier_id)?.into_iter().rev() {
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

/// Writes one movement. Runs inside the caller's transaction.
///
/// A movement raises the debt or lowers it, never both and never neither: the
/// migration's CHECK refuses the first, and this refuses the second, because
/// a row that moves nothing is a mistake somebody makes with an empty form
/// and it would sit in a statement forever.
pub fn append(
    conn: &mut SqliteConnection,
    shop_id: i32,
    entry: NewSupplierEntry,
) -> Result<SupplierEntry, CoreError> {
    append_at(conn, shop_id, entry, None)
}

/// The same movement, stamped with the moment it belongs to rather than with
/// the moment it was written. `None` is now, on the shop's clock.
///
/// The column's own `CURRENT_TIMESTAMP` default is never used: it is UTC,
/// while a purchase's day is on the shop's calendar, and one hour a day the
/// two disagree about which day a movement landed on.
pub fn append_at(
    conn: &mut SqliteConnection,
    shop_id: i32,
    entry: NewSupplierEntry,
    at: Option<NaiveDateTime>,
) -> Result<SupplierEntry, CoreError> {
    ensure_supplier(conn, shop_id, entry.supplier_id)?;
    // The order a movement cites is checked the same way the allocation's is:
    // a statement that names an order this shop never placed is worse than
    // one that names none.
    if let Some(purchase_id) = entry.purchase_id {
        ensure_purchase(conn, shop_id, purchase_id)?;
    }
    if entry.debit.is_negative() || entry.credit.is_negative() {
        return Err(CoreError::validation(
            "debit",
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
        &SupplierDebtRowWrite {
            shop_id,
            supplier_id: entry.supplier_id,
            purchase_id: entry.purchase_id,
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

/// Records that part of a payment settled one order. Runs inside the caller's
/// transaction, next to the `payment` row it belongs to.
pub fn allocate(
    conn: &mut SqliteConnection,
    shop_id: i32,
    allocation: NewSupplierAllocation,
) -> Result<SupplierAllocation, CoreError> {
    if allocation.amount.as_centimes() <= 0 {
        return Err(CoreError::validation(
            "amount",
            "an allocation of nothing settles nothing",
        ));
    }
    ensure_entry(conn, shop_id, allocation.payment_ledger_id)?;
    ensure_purchase(conn, shop_id, allocation.purchase_id)?;
    repo::allocate(
        conn,
        &SupplierAllocationRowWrite {
            shop_id,
            payment_ledger_id: allocation.payment_ledger_id,
            purchase_id: allocation.purchase_id,
            amount_centimes: allocation.amount.as_centimes(),
        },
    )
}

/// What has been settled against one order, oldest first.
pub fn allocations(
    conn: &mut SqliteConnection,
    shop_id: i32,
    purchase_id: i32,
) -> Result<Vec<SupplierAllocation>, CoreError> {
    ensure_purchase(conn, shop_id, purchase_id)?;
    repo::allocations(conn, shop_id, purchase_id)
}

/// What one payment settled, oldest order first. The fiche shows it beside
/// the movement, because which order the money went to is the question asked
/// of a payment.
pub fn allocations_of_payment(
    conn: &mut SqliteConnection,
    shop_id: i32,
    payment_ledger_id: i32,
) -> Result<Vec<SupplierAllocation>, CoreError> {
    ensure_entry(conn, shop_id, payment_ledger_id)?;
    repo::allocations_of_payment(conn, shop_id, payment_ledger_id)
}

/// Both foreign keys an allocation carries would take another shop's row, so
/// each is checked against the shop asking (rule 3).
fn ensure_entry(conn: &mut SqliteConnection, shop_id: i32, entry_id: i32) -> Result<(), CoreError> {
    if repo::entry_belongs_to_shop(conn, shop_id, entry_id)? {
        return Ok(());
    }
    Err(CoreError::NotFound {
        entity: "supplier_entry",
        id: entry_id,
    })
}

fn ensure_purchase(
    conn: &mut SqliteConnection,
    shop_id: i32,
    purchase_id: i32,
) -> Result<(), CoreError> {
    purchases_repo::get(conn, shop_id, purchase_id)?;
    Ok(())
}

/// That the shop still buys from this supplier. **T3's receipt path asks this
/// before it writes anything**, the way a sale asks the same of a customer's
/// fiche (`services::sales`): closing says the shop has stopped buying, and
/// goods arriving on a closed fiche are either a mistake or a fiche somebody
/// has to reopen on purpose.
///
/// The money side never asks it. A payment and a correction land on a closed
/// fiche by design: the shop stopped buying, not paying.
pub fn ensure_active(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<(), CoreError> {
    if suppliers_repo::get(conn, shop_id, supplier_id)?.active {
        return Ok(());
    }
    Err(CoreError::validation(
        "supplier_id",
        "this supplier's fiche is closed; the shop no longer buys from them",
    ))
}

fn ensure_supplier(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<(), CoreError> {
    if suppliers_repo::belongs_to_shop(conn, shop_id, supplier_id)? {
        return Ok(());
    }
    Err(CoreError::NotFound {
        entity: "supplier",
        id: supplier_id,
    })
}
