// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::repos::testdb::{open, OWNER, SHOP};

fn a_row_at(action: &str, created_at: NaiveDateTime) -> AuditRowWrite {
    AuditRowWrite {
        shop_id: SHOP,
        user_id: OWNER,
        action: action.to_string(),
        entity: "product".to_string(),
        entity_id: None,
        before: None,
        after: None,
        created_at,
    }
}

/// The shop clock, the same source `services::audit::record` uses. Most
/// of these tests read a page or an action back and never the moment on
/// it; the two that do pick their own.
fn a_row(action: &str) -> AuditRowWrite {
    a_row_at(action, crate::services::clock::now())
}

fn at(moment: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(moment, "%Y-%m-%d %H:%M:%S").unwrap()
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

/// The range is half open, and which end is which decides whether a row
/// written at a shop midnight is counted once or twice. `services::audit`
/// hands a day down as midnight and the next midnight, so the day before
/// this one ends on the same value this one starts on: a `le` on the
/// upper bound would show that row under both days. Those rows are real
/// and not a corner case, because the migration onto the shop clock lands
/// every row stored at 23:00 exactly on a midnight.
#[test]
fn a_day_takes_its_own_midnight_and_leaves_the_next_one_to_the_day_after() {
    let (_dir, mut conn) = open();
    for (action, moment) in [
        ("the night before", "2026-09-11 23:59:59"),
        ("midnight", "2026-09-12 00:00:00"),
        ("last second", "2026-09-12 23:59:59"),
        ("the next midnight", "2026-09-13 00:00:00"),
    ] {
        insert(&mut conn, &a_row_at(action, at(moment))).unwrap();
    }
    let day = SearchFilter {
        created_from: Some(at("2026-09-12 00:00:00")),
        created_to: Some(at("2026-09-13 00:00:00")),
        ..SearchFilter::default()
    };
    let found = search(&mut conn, SHOP, &day, 10, 0).unwrap();
    let mut actions: Vec<&str> = found.iter().map(|e| e.action.as_str()).collect();
    actions.sort_unstable();
    assert_eq!(actions, vec!["last second", "midnight"], "{found:?}");
    assert_eq!(count(&mut conn, SHOP, &day).unwrap(), 2);
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
