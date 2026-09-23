//! The `ACTION_*` tags a shop's own trade writes to
//! `dzpos_kernel::services::audit`, moved out of that crate's copy of this
//! file by S4 of `a-kernel-crate-and-retail-as-the-first-module`: the audit
//! service stays in the kernel and records every row exactly as it did
//! before, but the twenty-four action names below are a shop's own
//! vocabulary and belong to the module that raises each one, not to the
//! service that only ever writes down the string it is handed.
//!
//! Outside `crates/retail/src/services/` on purpose, rather than a
//! `services::audit_actions`: a file inside that folder is read as a second
//! `audit` service by the ring walk in
//! `crates/core/tests/services_go_through_services.rs`
//! (`service_files()` reads one directory deep and does not care what a
//! file holds, only where it sits), and a service with no repo of its own
//! and no siblings to reach past would be a strange new row on that walk
//! rather than the plain constant list this is.
//!
//! Named `<thing>.<what happened>`, the scheme the kernel's own copy of
//! this file still documents for the twenty constants that stayed there
//! (`create`, `update`, `set_regime` predate the scheme and are the
//! exception, not the rule). Every caller reaches these the way it already
//! reached `dzpos_kernel::services::audit::ACTION_CREATE`: an explicit
//! `crate::audit_actions::ACTION_X`, never a glob.

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
/// Only a user holding `Permission::OverrideCreditBlock` can take it (M4 T6);
/// the row's `user_id` is that person.
pub const ACTION_CREDIT_OVERRIDE: &str = "document.issue_override";

/// A credit sale the customer's limit refused. Written after the sale's
/// transaction has rolled back, on the same connection, because a row written
/// inside a transaction that unwinds unwinds with it: that is why the M2
/// review found a cashier could probe a customer's limit as many times as
/// they liked and leave nothing behind (M2 carry-in, 2026-09-09). The entry
/// carries the customer, what they would have owed and what they are allowed
/// to owe. There is no document to name: the refusal produced none, which is
/// the point of it.
///
/// The window this leaves is one process death wide, between the rollback and
/// this write. The alternative the M2 ruling proposed, a second connection,
/// leaves the same window and opens a second writer on the same SQLite file.
pub const ACTION_CREDIT_BLOCKED: &str = "sale.credit_blocked";

/// A sale refused because the till tried to take money off a price and the
/// person ringing it up may not. Both doors are covered: a price typed over
/// the one on the product's card, and a discount past the shop's threshold.
/// Written after the rollback for the same reason as the row above, and found
/// the same way: the two permission checks sit inside the sale's transaction,
/// so a row written where they refuse unwinds with the sale and the log sees
/// nothing at all (M4 closing review, 2026-09-11). The entry carries which of
/// the two was tried and what was asked for, because a percentage off a
/// basket and a price typed over a card read differently to an owner.
pub const ACTION_PRICE_CUT_BLOCKED: &str = "sale.price_cut_blocked";

/// A credit sale that landed at or past the customer's warn threshold. Not a
/// refusal and not a decision anybody took: the sale went through, and this
/// says the account crossed the line the shop asked to hear about. Written
/// inside the sale's transaction, unlike the blocked row, because a sale that
/// warns is a sale that happened.
///
/// An overridden sale writes `ACTION_CREDIT_OVERRIDE` instead and not both:
/// that row already carries the warning in its `after`, and a shop reading
/// its log wants one row per sale, not one per rule the sale touched.
pub const ACTION_CREDIT_WARNED: &str = "sale.credit_warned";

/// A sale discounted past the shop's dated threshold on purpose
/// (features.md §5 names "discount override" as its own audited action, so
/// it is not the credit override's row under another name). The entry
/// carries the basket before any discount, the threshold in force on the day
/// and what it allowed, against the discount actually given and how it was
/// split between the lines and the basket. Only a user holding
/// `Permission::DiscountAboveThreshold` can take it (M4 T6).
pub const ACTION_DISCOUNT_OVERRIDE: &str = "document.discount_override";

/// A line sold at a price that is not the product's own. The entry carries
/// the product, the price on its card and the price actually charged, so a
/// reader sees the negotiation rather than a total they cannot account for.
/// M1 shipped the negotiated price ungated because the till had one user and
/// asked for this gate in its review (M1 carry-in, 2026-09-09); only a user
/// holding `Permission::ChangePriceAtTheTill` can take it. Without it the
/// discount threshold is decoration: the same money comes off by typing a
/// lower price instead of a discount.
pub const ACTION_PRICE_OVERRIDE: &str = "document.price_override";

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
/// never edited and never deleted, so `create` is the whole of its
/// life and this row is the only trace of who spent what on which day. The
/// entry carries the category's key rather than its id, because the log is
/// read by a person and an id is a number they would have to look up.
pub const ACTION_CREATE_EXPENSE: &str = "expense.create";

/// A till opened with a float (features.md §1, the cash position). The entry
/// carries what was in the drawer when the person took it over. There is no
/// `before`: a shift is a row that did not exist a moment ago.
pub const ACTION_OPEN_TILL: &str = "till.open";

/// A till counted and closed. The entry carries what the shop expected that
/// person to be holding, what they counted, the difference between the two and
/// the reason given for it, and it names the opener whenever the closer is
/// somebody else, because a drawer closed by a second person is the one case
/// where "whose count was short" and "who signed for it" are different
/// answers.
///
/// The expected figure travels as the row's own copy of the snapshot the
/// column stores. It is not recomputed when the log is read: a ticket annulled
/// on Wednesday must not move a figure somebody signed on Monday.
pub const ACTION_CLOSE_TILL: &str = "till.close";

/// A sale rung while its ringer had no shift open, or rung by a phone whose
/// queue reached the server after they closed. Accepted and never refused, so
/// this row is the whole of what marks it: a shift is derived from the moments
/// either side of it and no column on `documents` says which shift a sale
/// belongs to. The entry carries the document and the moment it was issued, so
/// a close that reads over by exactly that amount has the row that explains
/// it.
pub const ACTION_SALE_OUTSIDE_SHIFT: &str = "till.sale_outside_shift";

/// A cached quantity on hand the ledger did not explain, corrected by the
/// recount (features.md §1). The entry carries the product's name beside its
/// id, both quantities and the difference between them, because it is the
/// only record a recount leaves: there is no table of runs, and the drift
/// list a shop owner reads is these rows read back. The day the run was
/// marked under travels in the entry too, because `services::stock` finds a
/// run by reading that day out of the entry rather than off the row's own
/// moment.
pub const ACTION_STOCK_DRIFT: &str = "stock.drift";

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

/// A discount threshold change appended to the dated series, the same shape
/// as the kernel's own `ACTION_SET_REGIME` (M4 T1, features.md §5). Moved
/// here with `DISCOUNT_THRESHOLD_BPS` itself, in the same commit that took
/// the key out of `dzpos_kernel::services::settings`
/// (`crate::services::discount_threshold`): the two belong together.
pub const ACTION_SET_DISCOUNT_THRESHOLD: &str = "set_discount_threshold";
