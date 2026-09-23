use super::{close, get, insert, list_between, list_for_user, open_for};
use crate::models::shift::{ShiftCloseWrite, ShiftRowWrite};
use crate::money::Money;
use crate::repos::testdb::{open, OWNER, SHOP};
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;

fn moment(day: u32, hour: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, day)
        .and_then(|d| d.and_hms_opt(hour, 0, 0))
        .unwrap()
}

fn opening(user_id: i32, day: u32, hour: u32, centimes: i64) -> ShiftRowWrite {
    ShiftRowWrite {
        shop_id: SHOP,
        opened_by: user_id,
        opened_at: moment(day, hour),
        opening_cash_centimes: centimes,
    }
}

/// A second person at the same till. The seeded file carries one user, so
/// the overlapping-drawer cases have to write their own.
fn second_cashier(conn: &mut SqliteConnection) -> i32 {
    diesel::sql_query(
        "INSERT INTO users (id, shop_id, name, role) VALUES (2, 1, 'Karim', 'cashier')",
    )
    .execute(conn)
    .unwrap();
    2
}

#[test]
fn a_shift_is_read_back_open_with_the_float_it_was_given() {
    let (_dir, mut conn) = open();
    let made = insert(&mut conn, &opening(OWNER, 21, 9, 500_000)).unwrap();
    let read = get(&mut conn, SHOP, made.id).unwrap();
    assert_eq!(read, made);
    assert_eq!(read.opening_cash, Money::centimes(500_000));
    assert!(read.is_open());
    assert_eq!(read.close, None);
    assert_eq!(open_for(&mut conn, SHOP, OWNER).unwrap(), Some(read));
    // Another shop's id reads nothing, the way every query in this
    // directory does (rule 3).
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(&mut conn)
        .unwrap();
    assert!(get(&mut conn, 2, made.id).is_err());
    assert_eq!(open_for(&mut conn, 2, OWNER).unwrap(), None);
}

#[test]
fn a_second_drawer_for_the_same_person_is_a_conflict_and_one_for_another_person_is_not() {
    let (_dir, mut conn) = open();
    insert(&mut conn, &opening(OWNER, 21, 9, 500_000)).unwrap();
    let again = insert(&mut conn, &opening(OWNER, 21, 14, 500_000));
    assert!(
        matches!(again, Err(crate::error::CoreError::Conflict { ref field, .. }) if field == "opened_by"),
        "a second open drawer for one person came back as {again:?}"
    );
    let karim = second_cashier(&mut conn);
    // Overlapping on purpose: each person's cash is physically their own.
    assert!(insert(&mut conn, &opening(karim, 21, 10, 300_000)).is_ok());
    assert_eq!(
        open_for(&mut conn, SHOP, karim)
            .unwrap()
            .map(|s| s.opening_cash),
        Some(Money::centimes(300_000))
    );
}

#[test]
fn closing_fills_the_four_columns_and_a_second_close_finds_nothing() {
    let (_dir, mut conn) = open();
    let made = insert(&mut conn, &opening(OWNER, 21, 9, 500_000)).unwrap();
    let karim = second_cashier(&mut conn);
    let write = ShiftCloseWrite {
        closed_at: moment(21, 19),
        // Closed by somebody else, which is the case `closed_by` exists
        // for: the column has to come back holding 2 and not 1.
        closed_by: karim,
        counted_centimes: 1_180_000,
        expected_at_close_centimes: 1_200_000,
        note: Some("20 000 remis au patron".to_string()),
    };
    let closed = close(&mut conn, SHOP, made.id, &write).unwrap();
    let held = closed.close.clone().unwrap();
    assert_eq!(held.closed_by, karim);
    assert_eq!(held.counted, Money::centimes(1_180_000));
    assert_eq!(held.expected, Money::centimes(1_200_000));
    assert_eq!(held.difference().ok(), Some(Money::centimes(-20_000)));
    assert_eq!(closed.note.as_deref(), Some("20 000 remis au patron"));
    assert!(!closed.is_open());
    assert_eq!(open_for(&mut conn, SHOP, OWNER).unwrap(), None);
    // The drawer is free again once it is closed, which is what the
    // partial index buys over a plain one.
    assert!(insert(&mut conn, &opening(OWNER, 22, 9, 1_180_000)).is_ok());
    assert!(close(&mut conn, SHOP, made.id, &write).is_err());
}

#[test]
fn a_clean_close_clears_a_note_the_open_row_was_carrying() {
    // Diesel skips a `None` field on an UPDATE unless the changeset says
    // otherwise, which would close a drawer against a sentence written
    // when it opened. The file would accept that: the note is not null,
    // so `shifts_a_difference_carries_a_reason` is satisfied by a reason
    // for something else entirely.
    let (_dir, mut conn) = open();
    let made = insert(&mut conn, &opening(OWNER, 21, 9, 500_000)).unwrap();
    diesel::sql_query("UPDATE shifts SET note = 'ouverte en retard'")
        .execute(&mut conn)
        .unwrap();
    let closed = close(
        &mut conn,
        SHOP,
        made.id,
        &ShiftCloseWrite {
            closed_at: moment(21, 19),
            closed_by: OWNER,
            counted_centimes: 1_200_000,
            expected_at_close_centimes: 1_200_000,
            note: None,
        },
    )
    .unwrap();
    assert_eq!(closed.note, None);
    assert_eq!(get(&mut conn, SHOP, made.id).unwrap().note, None);
}

#[test]
fn a_short_close_with_no_reason_cannot_borrow_the_one_the_open_row_held() {
    // The case the changeset exists for, and the one that goes red without
    // it: a drawer 20 000 centimes short, closed with no reason given, over
    // a row carrying a sentence about the opening. Skipping the `None`
    // would leave `ouverte en retard` standing in as the explanation and
    // `shifts_a_difference_carries_a_reason` would be satisfied by it, so
    // the close would land. Nulling the column is what makes the file
    // refuse the row.
    let (_dir, mut conn) = open();
    let made = insert(&mut conn, &opening(OWNER, 21, 9, 500_000)).unwrap();
    diesel::sql_query("UPDATE shifts SET note = 'ouverte en retard'")
        .execute(&mut conn)
        .unwrap();
    let refused = close(
        &mut conn,
        SHOP,
        made.id,
        &ShiftCloseWrite {
            closed_at: moment(21, 19),
            closed_by: OWNER,
            counted_centimes: 1_180_000,
            expected_at_close_centimes: 1_200_000,
            note: None,
        },
    );
    assert!(
        refused.is_err(),
        "a drawer 20 000 short closed with no reason, against the note the \
         shift opened with"
    );
    // And the row is left as it was, still open.
    let after = get(&mut conn, SHOP, made.id).unwrap();
    assert!(after.is_open());
    assert_eq!(after.note.as_deref(), Some("ouverte en retard"));
}

#[test]
fn the_lists_are_newest_first_and_the_user_list_holds_only_that_person() {
    let (_dir, mut conn) = open();
    let karim = second_cashier(&mut conn);
    let mine = insert(&mut conn, &opening(OWNER, 21, 9, 100)).unwrap().id;
    let theirs = insert(&mut conn, &opening(karim, 22, 9, 100)).unwrap().id;
    let ids: Vec<i32> = list_between(&mut conn, SHOP, moment(21, 0), moment(23, 0), None)
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(ids, vec![theirs, mine]);
    // Half open: the first moment of the window is in, the moment the
    // window ends is out.
    let ids: Vec<i32> = list_between(&mut conn, SHOP, moment(21, 9), moment(22, 9), None)
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(ids, vec![mine]);
    // The manager's list narrowed to one person, over the same window a
    // narrower one already proved: the whole-shop answer above holds
    // both rows, and naming a user here is what tells that filter apart
    // from the window one.
    let ids: Vec<i32> = list_between(&mut conn, SHOP, moment(21, 0), moment(23, 0), Some(karim))
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(ids, vec![theirs]);
    let ids: Vec<i32> = list_for_user(&mut conn, SHOP, karim)
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(ids, vec![theirs]);
}
