//! The only place `shifts` touches diesel. Every query is scoped by
//! `shop_id` (rule 3), and the ones a single person's drawer is read through
//! are scoped by `opened_by` as well: two cashiers hold overlapping shifts on
//! purpose, so a query that forgot the user would hand each of them the
//! other one's evening.
//!
//! Nothing outside this crate calls any of it yet. `services::shifts` is the
//! next task on the plan page and is what these are shaped for; the round
//! trips below are what says they work until it exists.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::shift::{Shift, ShiftCloseWrite, ShiftRow, ShiftRowWrite};
use crate::schema::shifts;

/// Opens a drawer. The unique index refuses a second open shift for the same
/// person, and that refusal comes back as a `Conflict` rather than a bare
/// query error so the caller has a field and a sentence to show.
#[cfg_attr(not(test), allow(dead_code))]
pub fn insert(conn: &mut SqliteConnection, write: &ShiftRowWrite) -> Result<Shift, CoreError> {
    let row: ShiftRow = match diesel::insert_into(shifts::table)
        .values(write)
        .returning(ShiftRow::as_returning())
        .get_result(conn)
    {
        Ok(row) => row,
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        )) => {
            return Err(CoreError::Conflict {
                field: "opened_by".to_string(),
                message: "that person already has a till open".to_string(),
            })
        }
        Err(other) => return Err(CoreError::Query(other)),
    };
    Ok(Shift::from(row))
}

/// One shift of this shop.
#[cfg_attr(not(test), allow(dead_code))]
pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Shift, CoreError> {
    let row: ShiftRow = shifts::table
        .filter(shifts::shop_id.eq(shop_id))
        .filter(shifts::id.eq(id))
        .select(ShiftRow::as_select())
        .first(conn)
        .optional()?
        .ok_or(CoreError::NotFound {
            entity: "shift",
            id,
        })?;
    Ok(Shift::from(row))
}

/// The one open shift this person holds, or None. The index makes "the one"
/// true, so this reads a single row rather than the newest of several.
#[cfg_attr(not(test), allow(dead_code))]
pub fn open_for(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
) -> Result<Option<Shift>, CoreError> {
    let row: Option<ShiftRow> = shifts::table
        .filter(shifts::shop_id.eq(shop_id))
        .filter(shifts::opened_by.eq(user_id))
        .filter(shifts::closed_at.is_null())
        .select(ShiftRow::as_select())
        .first(conn)
        .optional()?;
    Ok(row.map(Shift::from))
}

/// Writes the four close columns and the note in one UPDATE, and only onto a
/// shift of this shop that is still open. A second close finds no row and is
/// answered `NotFound` rather than overwriting a count somebody signed.
#[cfg_attr(not(test), allow(dead_code))]
pub fn close(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    write: &ShiftCloseWrite,
) -> Result<Shift, CoreError> {
    let row: ShiftRow = diesel::update(
        shifts::table
            .filter(shifts::shop_id.eq(shop_id))
            .filter(shifts::id.eq(id))
            .filter(shifts::closed_at.is_null()),
    )
    .set(write)
    .returning(ShiftRow::as_returning())
    .get_result(conn)
    .optional()?
    .ok_or(CoreError::NotFound {
        entity: "shift",
        id,
    })?;
    Ok(Shift::from(row))
}

/// The shop's shifts opened inside a half open window, newest first. Half
/// open on `opened_at` for the reason `clock::Period::moments` gives: every
/// moment of the last day counts and the next day's first does not.
#[cfg_attr(not(test), allow(dead_code))]
pub fn list_between(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Vec<Shift>, CoreError> {
    let rows: Vec<ShiftRow> = shifts::table
        .filter(shifts::shop_id.eq(shop_id))
        .filter(shifts::opened_at.ge(from))
        .filter(shifts::opened_at.lt(until))
        .order((shifts::opened_at.desc(), shifts::id.desc()))
        .select(ShiftRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(Shift::from).collect())
}

/// Every shift one person has held, newest first. This is the list a sale is
/// held against to see whether it fell inside any of that person's own
/// windows; a shop-wide list would say a sale belonged to a shift somebody
/// else was holding.
#[cfg_attr(not(test), allow(dead_code))]
pub fn list_for_user(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
) -> Result<Vec<Shift>, CoreError> {
    let rows: Vec<ShiftRow> = shifts::table
        .filter(shifts::shop_id.eq(shop_id))
        .filter(shifts::opened_by.eq(user_id))
        .order((shifts::opened_at.desc(), shifts::id.desc()))
        .select(ShiftRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(Shift::from).collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
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
    fn the_lists_are_newest_first_and_the_user_list_holds_only_that_person() {
        let (_dir, mut conn) = open();
        let karim = second_cashier(&mut conn);
        let mine = insert(&mut conn, &opening(OWNER, 21, 9, 100)).unwrap().id;
        let theirs = insert(&mut conn, &opening(karim, 22, 9, 100)).unwrap().id;
        let ids: Vec<i32> = list_between(&mut conn, SHOP, moment(21, 0), moment(23, 0))
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(ids, vec![theirs, mine]);
        // Half open: the first moment of the window is in, the moment the
        // window ends is out.
        let ids: Vec<i32> = list_between(&mut conn, SHOP, moment(21, 9), moment(22, 9))
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(ids, vec![mine]);
        let ids: Vec<i32> = list_for_user(&mut conn, SHOP, karim)
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(ids, vec![theirs]);
    }
}
