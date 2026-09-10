//! The audit log of sensitive actions (features.md §5): a price change, a
//! settings change, a régime change. An ISO 27001 control that costs almost
//! nothing while the writes are being written, and a rewrite afterwards.
//!
//! It records, it never decides. A caller writes the row in the same
//! transaction as the change, so an entry without its change cannot exist.

use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::audit::{AuditEntry, AuditRowWrite};
use crate::repos::audit as repo;

/// A row that did not exist before, such as a customer fiche.
pub const ACTION_CREATE: &str = "create";
/// A settings or shop block that was replaced.
pub const ACTION_UPDATE: &str = "update";
/// A régime change appended to the dated series.
pub const ACTION_SET_REGIME: &str = "set_regime";
/// The actions below are named `<thing>.<what happened>`, and the thing is
/// what the row's `entity` says: a document, a debt, a customer. `create`,
/// `update` and `set_regime` predate the scheme and are left as they are,
/// because a log is read for what it holds and renaming a row that is
/// already written is not something a migration can do honestly.
///
/// Money against a customer's debt, written as a ledger movement with the
/// documents it settled. The entry carries the balance before and after and
/// the documents the money landed on, so the log reads as the settlement it
/// was without anyone summing the ledger again.
pub const ACTION_PAY_DEBT: &str = "debt.pay";
/// A correction to what a customer owes, written as a ledger movement. The
/// entry carries the balance before and after, so the log reads as the
/// change it was without anyone summing the ledger again.
pub const ACTION_ADJUST_DEBT: &str = "debt.adjust";
/// A credit sale taken past the customer's credit limit on purpose. The
/// entry carries the balance and the limit the rule refused on, and the
/// document the decision produced, so the log reads as the decision it was.
/// Until M4 there are no roles and anyone may take it (features.md §1).
pub const ACTION_CREDIT_OVERRIDE: &str = "document.issue_override";

/// A credit note written against a facture. The entry names the facture that
/// changed, because that is the paper a reader is holding when they ask why it
/// stopped asking for its amount, and carries the avoir it produced, what the
/// avoir was worth and both balances, so the log reads as the reversal it was.
pub const ACTION_AVOIR: &str = "document.avoir";
/// A document annulled. It keeps its number and its row, so what the log adds
/// is when, by whom, why, and the avoir the cancellation issued when it issued
/// one (features.md §3).
pub const ACTION_CANCEL: &str = "document.cancel";

/// Money out that is not stock (features.md §1, Expense). An expense is
/// never edited and never deleted in M3, so `create` is the whole of its
/// life and this row is the only trace of who spent what on which day. The
/// entry carries the category's key rather than its id, because the log is
/// read by a person and an id is a number they would have to look up.
pub const ACTION_CREATE_EXPENSE: &str = "expense.create";

/// A fiche closed while it was still carrying something: a balance either
/// way, or a document still asking to be paid. The entry carries the reason
/// the caller had to give, the balance at the moment of the close and how
/// many documents were still open, because a shop that stops trading with a
/// customer who owes it money has taken a decision and the log is where it
/// is written down. A close over an account that was already settled is an
/// ordinary update and is logged as one.
pub const ACTION_CLOSE_CUSTOMER: &str = "customer.close";

/// A supplier fiche closed while its account was still open: a balance either
/// way, or an order still asking to be paid. The same decision the customer
/// one records, on the side the shop owes rather than the side that owes it,
/// and the entry carries the reason, the balance and how many orders were
/// still open.
pub const ACTION_CLOSE_SUPPLIER: &str = "supplier.close";
/// Money paid to a supplier, written as a ledger movement with the orders it
/// settled. The entry carries the balance before and after, so the log reads
/// as the settlement it was without anyone summing the ledger again.
pub const ACTION_PAY_SUPPLIER: &str = "supplier_debt.pay";
/// A correction to what the shop owes a supplier, written as a ledger
/// movement. The entry carries the balance before and after.
pub const ACTION_ADJUST_SUPPLIER: &str = "supplier_debt.adjust";

/// An order placed with a supplier. The entry carries what the order is
/// worth once the extra costs are landed on its lines, so the log says what
/// the shop committed to before any of it arrived.
pub const ACTION_CREATE_PURCHASE: &str = "purchase.create";
/// A delivery taken in against an order. The entry carries the bon de
/// réception it was written on, the value that arrived at landed cost and the
/// state the order moved to, because this is the moment the stock and the
/// supplier's account both move.
pub const ACTION_RECEIVE_PURCHASE: &str = "purchase.receive";
/// Goods handed back to the supplier. It writes no document, so the log and
/// the two rows it names (a stock movement out and a credit on the ledger)
/// are the whole record of it.
pub const ACTION_RETURN_PURCHASE: &str = "purchase.return";
/// An order cancelled before anything arrived, with the reason the caller
/// had to give.
pub const ACTION_CANCEL_PURCHASE: &str = "purchase.cancel";
/// An order closed after a partial delivery: the rest will never come and is
/// written off. A decision, so the reason is in the entry.
pub const ACTION_CLOSE_SHORT_PURCHASE: &str = "purchase.close_short";

/// What changed, as the log stores it. `before` and `after` are JSON
/// documents the caller writes; the log never guesses a shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub action: &'static str,
    pub entity: &'static str,
    pub entity_id: Option<i32>,
    pub before: Option<String>,
    pub after: Option<String>,
}

pub fn record(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    change: Change,
) -> Result<(), CoreError> {
    repo::insert(
        conn,
        &AuditRowWrite {
            shop_id,
            user_id,
            action: change.action.to_string(),
            entity: change.entity.to_string(),
            entity_id: change.entity_id,
            before: change.before,
            after: change.after,
        },
    )
}

pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<AuditEntry>, CoreError> {
    repo::list(conn, shop_id)
}
