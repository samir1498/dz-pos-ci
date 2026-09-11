//! The only place the audit log touches diesel. Scoped by `shop_id` like
//! every other query (rule 3).

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sqlite::{Sqlite, SqliteConnection};

use crate::error::CoreError;
use crate::models::audit::{AuditEntry, AuditRow, AuditRowWrite};
use crate::schema::audit_log;

pub fn insert(conn: &mut SqliteConnection, write: &AuditRowWrite) -> Result<(), CoreError> {
    diesel::insert_into(audit_log::table)
        .values(write)
        .execute(conn)?;
    Ok(())
}

/// One action's rows, newest first. The order is the opposite of `list`'s on
/// purpose: a caller asking for one action wants the last time it happened,
/// and it reads backwards until the run it is looking at ends.
pub fn by_action(
    conn: &mut SqliteConnection,
    shop_id: i32,
    action: &str,
) -> Result<Vec<AuditEntry>, CoreError> {
    let rows: Vec<AuditRow> = audit_log::table
        .filter(audit_log::shop_id.eq(shop_id))
        .filter(audit_log::action.eq(action))
        .order(audit_log::id.desc())
        .select(AuditRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(AuditEntry::from).collect())
}

/// Oldest first: the log is read as a story, not as a feed.
pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<AuditEntry>, CoreError> {
    let rows: Vec<AuditRow> = audit_log::table
        .filter(audit_log::shop_id.eq(shop_id))
        .order(audit_log::id.asc())
        .select(AuditRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(AuditEntry::from).collect())
}

/// What `search` and `count` narrow the shop's log by. A plain description
/// rather than a diesel expression: `services::audit` hands this down and
/// never touches the query builder itself (rule 1 — this file is the only
/// place that does). `created_from`/`created_to` are a half-open UTC range;
/// the shop-calendar-to-UTC conversion (`services::audit::day_range_utc`) is
/// the service's job, not this one's.
#[derive(Debug, Clone, Default)]
pub struct SearchFilter {
    pub user_id: Option<i32>,
    pub action: Option<String>,
    pub created_from: Option<NaiveDateTime>,
    pub created_to: Option<NaiveDateTime>,
}

fn filtered<'a>(shop_id: i32, filter: &SearchFilter) -> audit_log::BoxedQuery<'a, Sqlite> {
    let mut query = audit_log::table
        .filter(audit_log::shop_id.eq(shop_id))
        .into_boxed();
    if let Some(user_id) = filter.user_id {
        query = query.filter(audit_log::user_id.eq(user_id));
    }
    if let Some(action) = &filter.action {
        query = query.filter(audit_log::action.eq(action.clone()));
    }
    if let Some(from) = filter.created_from {
        query = query.filter(audit_log::created_at.ge(from));
    }
    if let Some(to) = filter.created_to {
        query = query.filter(audit_log::created_at.lt(to));
    }
    query
}

/// One page of the shop's log, filtered and newest first: the shape the
/// owner's screen reads (M4 T7), the opposite of `list`'s story order for
/// the same reason `by_action` is. The `WHERE` and the `LIMIT`/`OFFSET` are
/// both in the query, not in a `Vec` a caller slices afterwards — the gap
/// this function closes (`services::purchases::list` still has it, over a
/// smaller table).
pub fn search(
    conn: &mut SqliteConnection,
    shop_id: i32,
    filter: &SearchFilter,
    limit: i64,
    offset: i64,
) -> Result<Vec<AuditEntry>, CoreError> {
    let rows: Vec<AuditRow> = filtered(shop_id, filter)
        .order(audit_log::id.desc())
        .limit(limit)
        .offset(offset)
        .select(AuditRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(AuditEntry::from).collect())
}

/// How many rows `search` would answer under the same filter, ignoring
/// `limit`/`offset`: what a screen needs to know whether asking for the next
/// page would answer more (`services::audit::read`'s `has_more`).
pub fn count(
    conn: &mut SqliteConnection,
    shop_id: i32,
    filter: &SearchFilter,
) -> Result<i64, CoreError> {
    let total = filtered(shop_id, filter).count().get_result(conn)?;
    Ok(total)
}

/// Every distinct action the shop's log actually holds, alphabetically: one
/// of the owner's screen's two dropdowns (`services::audit::Facets`), read
/// off the whole log and not narrowed by the filter in force, so a filter
/// that empties the page never empties the dropdown that would undo it.
pub fn distinct_actions(
    conn: &mut SqliteConnection,
    shop_id: i32,
) -> Result<Vec<String>, CoreError> {
    let actions = audit_log::table
        .filter(audit_log::shop_id.eq(shop_id))
        .select(audit_log::action)
        .distinct()
        .order(audit_log::action.asc())
        .load(conn)?;
    Ok(actions)
}

#[cfg(test)]
mod tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::repos::testdb::{open, OWNER, SHOP};

    fn a_row(action: &str) -> AuditRowWrite {
        AuditRowWrite {
            shop_id: SHOP,
            user_id: OWNER,
            action: action.to_string(),
            entity: "product".to_string(),
            entity_id: None,
            before: None,
            after: None,
        }
    }

    /// Five rows, a `limit` of two: what the old `list_desc` could not even
    /// be asked for, since it took no `limit` or `offset` at all — the whole
    /// point of the change this repo made. Deleting `search` and reaching
    /// for `list_desc` here instead does not just fail the assertion below;
    /// it fails to compile, because nothing about "the whole table" can be
    /// told to stop at two rows.
    #[test]
    fn search_answers_only_the_page_asked_for_newest_first() {
        let (_dir, mut conn) = open();
        for action in ["a", "b", "c", "d", "e"] {
            insert(&mut conn, &a_row(action)).unwrap();
        }
        let first = search(&mut conn, SHOP, &SearchFilter::default(), 2, 0).unwrap();
        let actions: Vec<&str> = first.iter().map(|e| e.action.as_str()).collect();
        assert_eq!(actions, vec!["e", "d"], "{first:?}");

        let second = search(&mut conn, SHOP, &SearchFilter::default(), 2, 2).unwrap();
        let actions: Vec<&str> = second.iter().map(|e| e.action.as_str()).collect();
        assert_eq!(actions, vec!["c", "b"], "{second:?}");
    }

    /// `count` answers every matching row under the filter, not the page: a
    /// `limit` of one still counts three, which is what a screen needs to
    /// know a second page exists at all.
    #[test]
    fn count_ignores_limit_and_offset_but_not_the_filter() {
        let (_dir, mut conn) = open();
        for action in ["a", "b", "c"] {
            insert(&mut conn, &a_row(action)).unwrap();
        }
        assert_eq!(count(&mut conn, SHOP, &SearchFilter::default()).unwrap(), 3);
        let narrowed = SearchFilter {
            action: Some("b".to_string()),
            ..SearchFilter::default()
        };
        assert_eq!(count(&mut conn, SHOP, &narrowed).unwrap(), 1);
        assert_eq!(
            search(&mut conn, SHOP, &SearchFilter::default(), 1, 0)
                .unwrap()
                .len(),
            1,
            "count must not be confused with a one-row page"
        );
    }

    #[test]
    fn distinct_actions_is_alphabetical_with_no_repeats() {
        let (_dir, mut conn) = open();
        for action in ["b", "a", "b", "c"] {
            insert(&mut conn, &a_row(action)).unwrap();
        }
        assert_eq!(
            distinct_actions(&mut conn, SHOP).unwrap(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }
}
