//! The audit log of sensitive actions (features.md §5): a price change, a
//! settings change, a régime change. An ISO 27001 control that costs almost
//! nothing while the writes are being written, and a rewrite afterwards.
//!
//! It records, it never decides. A caller writes the row in the same
//! transaction as the change, so an entry without its change cannot exist.

use std::collections::HashMap;

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::audit::{AuditEntry, AuditRowWrite};
use crate::repos::audit as repo;
use crate::services::clock;
use crate::services::user_names;

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

/// A user created.
pub const ACTION_CREATE_USER: &str = "user.create";
/// A shop's very first owner, claimed with a name and a password before
/// anybody has ever signed in (`services::users::claim_first_owner`). Its
/// own action rather than `ACTION_SET_PASSWORD`: an ordinary reset is a
/// person already inside the shop changing a credential, and this row is
/// the shop coming into existence as far as the till is concerned, written
/// with no session open and no actor but the owner it names. The door
/// refuses once any credential exists anywhere in the shop, so a shop's
/// log holds at most one of these, ever, and never more.
pub const ACTION_CLAIM_FIRST_OWNER: &str = "user.claim_first_owner";
/// A user renamed. Its own action rather than a generic update, because the
/// name is what the sign-in screen lists and the audit log prints.
pub const ACTION_RENAME_USER: &str = "user.rename";
/// A role changed. The entry carries the role before and after, which is the
/// whole of what a change of role is.
pub const ACTION_SET_ROLE: &str = "user.set_role";
/// A PIN set or reset. The entry says that one was set and never what it is:
/// a hash in a log is a hash in a backup and in every export of it.
pub const ACTION_SET_PIN: &str = "user.set_pin";
/// A password set or reset. The same, on the other credential.
pub const ACTION_SET_PASSWORD: &str = "user.set_password";
/// A user switched off. Never a deletion: the rows they wrote name them.
pub const ACTION_DEACTIVATE_USER: &str = "user.deactivate";
/// A user switched back on.
pub const ACTION_REACTIVATE_USER: &str = "user.reactivate";
/// A phone paired via QR (M6 T2). The device token is never logged, only the
/// device row id and its name — the token itself is a credential.
pub const ACTION_DEVICE_PAIRED: &str = "device.paired";
/// A paired phone revoked from settings (M6 T4).
pub const ACTION_DEVICE_REVOKED: &str = "device.revoked";
/// A user locked out by wrong credentials. Written on the crossing and not on
/// every attempt, so the log holds the event and not the noise: the entry
/// carries the count and the moment they may try again, because a lockout
/// nobody can see afterwards is a shop owner asking why the till would not
/// open and getting no answer.
pub const ACTION_LOCK_OUT_USER: &str = "user.locked_out";
/// A discount threshold change appended to the dated series, the same shape
/// as `ACTION_SET_REGIME` (M4 T1, features.md §5).
pub const ACTION_SET_DISCOUNT_THRESHOLD: &str = "set_discount_threshold";
/// How long a session survives unattended, changed (M4 T2). A preference and
/// not a dated setting, but still a control somebody answers for: lengthening
/// it is how a till is left open, and the theme beside it in the same table
/// writes no row for the reason this one does.
pub const ACTION_SET_SESSION_IDLE: &str = "set_session_idle";

/// A role was asked for something `services::permissions::can` refuses, on a
/// route the permission table names. Written once, by the
/// one seam every gated request passes through
/// (`crates/api/src/session.rs::require`), and never by a handler: a route
/// that forgot to ask would forget to log too, which is the same bug twice.
/// The entry carries the permission that was wanted and the route and method
/// that wanted it, in `after`; there is no `before` and no `entity_id`,
/// because a refusal changed no row.
pub const ACTION_PERMISSION_REFUSED: &str = "permission.refused";
/// The shop's data walked out on a USB stick: one of the four workbooks
/// (features.md §5, "walking out on a USB stick" is `Permission::
/// ExportAndImport`'s own doc). The entry carries which of the four and, when
/// the route already knows it, how many rows left with it.
pub const ACTION_EXPORT: &str = "export.download";
/// The shop file was replaced by one of its own copies. The entry carries the
/// copy's name, the safety copy taken of what it replaced, and what the
/// restored file holds; it is written to the file that copy became, after
/// the swap, because a row written before it would not survive being the
/// thing overwritten (`services::backup::record_restore`'s own doc says why).
pub const ACTION_RESTORE_BACKUP: &str = "backup.restore";
/// A support bundle was built and handed to whoever asked (M5 T3). Its own
/// action rather than `ACTION_EXPORT`'s, because what leaves the shop is
/// never a row of its data (`services::support_bundle`'s own doc names what
/// is in it and what is not) and the two are worth telling apart on the
/// log. Written for the same reason an export's row is: the file is on its
/// way out of the shop, and the point of this row is that it is there.
pub const ACTION_SUPPORT_BUNDLE: &str = "support_bundle.download";

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
            created_at: clock::now(),
        },
    )
}

pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<AuditEntry>, CoreError> {
    repo::list(conn, shop_id)
}

/// One action's rows, newest first, for a service that wrote them and now
/// wants to read them back. `stock.rs` is the caller: the recount writes a
/// `ACTION_STOCK_DRIFT` row per product it corrected, and the screen that
/// asks what the last run found reads that run off the log rather than
/// keeping a second copy of it.
///
/// Newest first is the repo's order and the reason it differs from `list`'s:
/// a caller asking for one action wants the last time it happened, and reads
/// backwards until that run ends.
///
/// Here rather than straight off `repos::audit` because a service reaching
/// into another domain's repo skips whatever that domain decides on the way
/// past. Nothing is decided here today; the point is that the next thing
/// decided about reading the log is decided once (architecture.md, the
/// layers section).
pub fn by_action(
    conn: &mut SqliteConnection,
    shop_id: i32,
    action: &str,
) -> Result<Vec<AuditEntry>, CoreError> {
    repo::by_action(conn, shop_id, action)
}

/// How many rows one person wrote under one action over a stretch of the
/// shop's clock. `from` is inclusive and `until` exclusive, the half-open
/// shape `repo::SearchFilter` already holds and `day_range` already hands it.
///
/// Here rather than off `repos::audit` for the reason `by_action` gives: a
/// service reaching into another domain's repo skips whatever that domain
/// decides on the way past.
///
/// `services::shifts::report` is the caller. A sale rung while nobody held a
/// drawer is tagged and stored nowhere else — `tag_if_outside_a_shift` writes
/// one `till.sale_outside_shift` row and no column — so the log is the only
/// place that count exists, and a screen that wants it has to come through
/// here.
pub fn count_for_user_between(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    action: &str,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<i64, CoreError> {
    repo::count(
        conn,
        shop_id,
        &repo::SearchFilter {
            user_id: Some(user_id),
            action: Some(action.to_owned()),
            created_from: Some(from),
            created_to: Some(until),
        },
    )
}

/// Rows a screen reads at a time (M4 T7). One shop's whole day almost never
/// fills a page; a shop's whole lifetime will, eventually, and this is the
/// number that keeps a screen open on it from asking for all of it.
pub const PAGE_SIZE: usize = 50;

/// One entry with the name behind its `user_id`, since there is no `/users`
/// list route yet for a screen to join it itself the way the purchases
/// screen joins a supplier's name off its own list (frontend-conventions.md,
/// "The design package"). Once one exists this becomes a plain `AuditEntry`
/// again and the screen does the join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryWithUser {
    pub entry: AuditEntry,
    pub user_name: String,
}

/// What the owner's screen may narrow the log by. `action` is matched
/// exactly against what a service actually wrote (`ACTION_*` above): the
/// screen offers only the values `Facets::actions` says are really in the
/// log, never a taxonomy invented on top of it.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub user_id: Option<i32>,
    pub action: Option<String>,
    /// A day on the shop's calendar, the same clock `record` stamps a row
    /// with: `day_range` below is the pair of moments it stands for.
    pub day: Option<NaiveDate>,
}

/// What the two dropdowns offer: every user the shop has, whether or not
/// they wrote a row, and every distinct action the log actually holds.
#[derive(Debug, Clone)]
pub struct Facets {
    pub users: Vec<(i32, String)>,
    pub actions: Vec<String>,
}

/// One screenful, and whether asking for the next `page` would answer more.
#[derive(Debug, Clone)]
pub struct Page {
    pub rows: Vec<EntryWithUser>,
    pub page: i64,
    pub has_more: bool,
}

/// `day`'s midnight to the next midnight, both on the shop's calendar,
/// which is what the column holds since `record` stamps it from
/// `services::clock`. It used to subtract the shop's offset here because the
/// column took SQLite's UTC default, which meant the filter and the date
/// printed beside it disagreed for the hour before midnight UTC.
fn day_range(day: NaiveDate) -> (NaiveDateTime, NaiveDateTime) {
    let midnight = NaiveDateTime::new(day, NaiveTime::MIN);
    (midnight, midnight + Duration::days(1))
}

/// One page of the log, filtered in SQL (`WHERE` and `LIMIT`/`OFFSET`, not a
/// `Vec` sliced afterwards), and the two dropdowns' options. The dropdowns
/// read off the whole log regardless of `filter` (`repo::distinct_actions`,
/// `user_names`) — a filter narrow enough to empty the page must not also
/// empty the choices that would widen it back.
///
/// The lockout row is the one filter case worth naming here: `user.locked_out`
/// is written with `user_id` set to the person the attempts were made on
/// (`services::users::settle`'s own comment — "The actor is the user the
/// attempts were made on: nobody knows who was standing there, and claiming
/// otherwise in an audit log is worse than saying nothing"), because nobody
/// is signed in when the row is written. That is a plain equality on the
/// stored column, same as any other row's `user_id`, so a filter by that
/// person still finds it — filtering it out would be inventing an actor the
/// row never claimed to have. What keeps the row from reading as something
/// that person *did* is the action's own name, `user.locked_out`, which
/// `record` writes the way every service writes it and this function does
/// not touch.
pub fn read(
    conn: &mut SqliteConnection,
    shop_id: i32,
    filter: &Filter,
    page_number: i64,
) -> Result<(Page, Facets), CoreError> {
    let names: HashMap<i32, String> = user_names(conn, shop_id)?.into_iter().collect();
    let mut people: Vec<(i32, String)> =
        names.iter().map(|(id, name)| (*id, name.clone())).collect();
    people.sort_by(|a, b| a.1.cmp(&b.1));
    let actions = repo::distinct_actions(conn, shop_id)?;

    let range = filter.day.map(day_range);
    let repo_filter = repo::SearchFilter {
        user_id: filter.user_id,
        action: filter.action.clone(),
        created_from: range.map(|(start, _)| start),
        created_to: range.map(|(_, end)| end),
    };

    // `page_number` comes straight off the query string, so a caller can
    // send anything up to `i64::MAX`: the multiplication below has to
    // saturate rather than overflow, or a large enough page answers 500
    // instead of the empty page it should. Unlike the old in-memory slice,
    // an absurd offset costs SQLite nothing to refuse: it just matches no
    // row.
    let page_number = page_number.max(1);
    let limit = i64::try_from(PAGE_SIZE).unwrap_or(i64::MAX);
    let offset = (page_number - 1).saturating_mul(limit);

    // A second query rather than a `COUNT(*) OVER()` window function: both
    // read the same filter, and writing it once as a plain, typed diesel
    // query — the house style every other repo in this folder uses — beats
    // a raw SQL fragment for one extra indexed read on a page the owner
    // opens by hand, not on a hot path.
    let total = repo::count(conn, shop_id, &repo_filter)?;
    let has_more = total > offset.saturating_add(limit);
    let rows = repo::search(conn, shop_id, &repo_filter, limit, offset)?
        .into_iter()
        .map(|entry| {
            let user_name = names.get(&entry.user_id).cloned().unwrap_or_default();
            EntryWithUser { entry, user_name }
        })
        .collect();

    Ok((
        Page {
            rows,
            page: page_number,
            has_more,
        },
        Facets {
            users: people,
            actions,
        },
    ))
}

#[cfg(test)]
mod tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// The column is on the shop's calendar, so a day is its own midnight to
    /// the next, and the row written at 00:30 in Algiers falls in the day it
    /// was written rather than in the one before. That row is the whole
    /// reason this function exists: before `record` stamped from the shop
    /// clock it was stored as 23:30 the previous day and the filter had to
    /// reach back an hour to find it, while the screen printed the earlier
    /// date beside it.
    #[test]
    fn a_shop_day_runs_from_its_own_midnight_to_the_next() {
        let day = NaiveDate::from_ymd_opt(2026, 9, 11).unwrap();
        let (start, end) = day_range(day);
        assert_eq!(
            start,
            NaiveDate::from_ymd_opt(2026, 9, 11)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        );
        assert_eq!(
            end,
            NaiveDate::from_ymd_opt(2026, 9, 12)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        );

        let just_after_midnight_in_algiers = NaiveDate::from_ymd_opt(2026, 9, 11)
            .unwrap()
            .and_hms_opt(0, 30, 0)
            .unwrap();
        assert!(just_after_midnight_in_algiers >= start && just_after_midnight_in_algiers < end);
        let (prev_start, prev_end) = day_range(day.pred_opt().unwrap());
        assert!(
            !(just_after_midnight_in_algiers >= prev_start
                && just_after_midnight_in_algiers < prev_end)
        );
    }
}
