//! The audit log of sensitive actions (features.md §5): a settings change,
//! a régime change, a credential reset. An ISO 27001 control that costs
//! almost nothing while the writes are being written, and a rewrite
//! afterwards.
//!
//! It records, it never decides. A caller writes the row in the same
//! transaction as the change, so an entry without its change cannot exist.
//!
//! This file's own `ACTION_*` constants are the vocabulary of a trade with
//! no shop concept in it: a row created or updated, a régime change, a
//! credential set or a session ended, a permission refused, a backup
//! restored. S4 of `a-kernel-crate-and-retail-as-the-first-module` moved
//! the twenty-four that named a shop operation (a sale's credit override, a
//! till opened or closed, a purchase received, a stock recount, and the
//! rest) to `dzpos_retail::audit_actions`, a file outside `services/` so it
//! is never read as a second `audit` service by the ring walk in
//! `crates/core/tests/services_go_through_services.rs`, and
//! `ACTION_SET_DISCOUNT_THRESHOLD` moved there too, with the same task's
//! move of `DISCOUNT_THRESHOLD_BPS` itself out of `services::settings`: the
//! two belong together, and a constant naming a setting should not outlive
//! the setting's own move by a commit. The service that writes and reads
//! every row, `record`, `by_action` and `read` below, is unchanged and
//! unmoved, because recording is still one job for the whole product.

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
/// How long a session survives unattended, changed (M4 T2). A preference and
/// not a dated setting, but still a control somebody answers for: lengthening
/// it is how a till is left open, and the theme beside it in the same table
/// writes no row for the reason this one does.
pub const ACTION_SET_SESSION_IDLE: &str = "set_session_idle";

/// The four below, plus `dzpos_retail::audit_actions`' whole list, are named
/// `<thing>.<what happened>`, and the thing is what the row's `entity` says:
/// a permission, a backup, a support bundle, an export. `create`, `update`
/// and `set_regime` above predate the scheme and are left as they are,
/// because a log is read for what it holds and renaming a row that is
/// already written is not something a migration can do honestly.
///
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
/// wants to read them back. `dzpos_retail::services::stock` is the caller:
/// the recount writes a `dzpos_retail::audit_actions::ACTION_STOCK_DRIFT`
/// row per product it corrected, and the screen that asks what the last run
/// found reads that run off the log rather than keeping a second copy of
/// it.
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
