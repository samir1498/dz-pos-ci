// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Migration 000017: the `shifts` table and the expense clock that ships
//! beside it.
//!
//! Every case here goes through a raw INSERT or UPDATE and never through a
//! service. `services::shifts` will repeat each of these checks for a kinder
//! message, and a test that went through it would stay green with the index
//! and both CHECKs deleted from `up.sql`, which is exactly the thing this
//! file exists to hold.
//!
//! In its own file and not in `tests/migration.rs` because that one is at
//! the length `scripts/file-sizes.json` pins it to. The helpers the cases
//! below use are the ones that file used to declare privately; they moved to
//! `tests/common/migrations.rs` in the same commit so both files read the
//! file the same way.

use diesel::prelude::*;

mod common;

use common::migrations::{count, insert_with, insert_with_all, open_before_migration, probe};
use common::open_temp;

/// The seeded shop and its owner, written by the first migration.
const SHOP: i32 = 1;
const OWNER: i32 = 1;

/// A second person at the same till. Two cashiers holding overlapping shifts
/// is the case the unique index is deliberately shaped for, and the seeded
/// file carries one user, so the cases below write the other.
const KARIM: i32 = 2;

fn seed_a_second_cashier(conn: &mut SqliteConnection) {
    diesel::sql_query(
        "INSERT INTO users (id, shop_id, name, role) VALUES (2, 1, 'Karim', 'cashier')",
    )
    .execute(conn)
    .unwrap();
}

/// A shift row written straight at the file, with whichever columns the case
/// is about swapped for the literals it wants.
fn write_shift(
    conn: &mut SqliteConnection,
    overrides: &[(&str, &str)],
) -> diesel::QueryResult<usize> {
    diesel::sql_query(insert_with_all("shifts", overrides)).execute(conn)
}

/// The four close columns, written together the way the file insists they
/// arrive. The note is the fifth argument because the second CHECK reads it
/// beside the other two.
fn closed(closer: i32, counted: &str, expected: &str, note: &str) -> Vec<(String, String)> {
    vec![
        ("closed_at".to_string(), "'2026-09-21 19:00:00'".to_string()),
        ("closed_by".to_string(), closer.to_string()),
        ("counted_centimes".to_string(), counted.to_string()),
        (
            "expected_at_close_centimes".to_string(),
            expected.to_string(),
        ),
        ("note".to_string(), note.to_string()),
    ]
}

fn borrowed(pairs: &[(String, String)]) -> Vec<(&str, &str)> {
    pairs
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect()
}

#[test]
fn one_person_holds_one_open_drawer_and_a_second_person_holds_their_own() {
    // Ruling 1: at most one open shift per user, and per user rather than per
    // shop. The index is what holds it; the service in the next task repeats
    // the check only to say it in a sentence.
    let (_dir, mut conn) = open_temp();
    seed_a_second_cashier(&mut conn);

    assert!(write_shift(&mut conn, &[("opened_by", "1")]).is_ok());
    assert!(
        write_shift(
            &mut conn,
            &[("opened_by", "1"), ("opened_at", "'2026-09-21 14:00:00'")]
        )
        .is_err(),
        "a second open drawer for one person was written"
    );
    assert!(
        write_shift(&mut conn, &[("opened_by", "2")]).is_ok(),
        "two cashiers may hold overlapping shifts: each one's cash is their own"
    );

    // Closing the first frees that person's drawer, which is what the partial
    // `WHERE closed_at IS NULL` buys over a plain unique index: somebody who
    // has worked every day for a year has a year of closed rows under the
    // same pair of columns.
    let close = closed(OWNER, "1200000", "1200000", "NULL");
    assert_eq!(
        diesel::sql_query(
            "UPDATE shifts SET closed_at = '2026-09-21 19:00:00', closed_by = 1, \
             counted_centimes = 1200000, expected_at_close_centimes = 1200000 \
             WHERE opened_by = 1"
        )
        .execute(&mut conn)
        .unwrap(),
        1
    );
    assert!(
        write_shift(
            &mut conn,
            &[("opened_by", "1"), ("opened_at", "'2026-09-22 09:00:00'")]
        )
        .is_ok(),
        "the drawer was still held after the shift closed"
    );
    // And a second closed row for the same person is no collision either.
    assert!(write_shift(&mut conn, &borrowed(&close)).is_ok());
    assert_eq!(
        count(
            &mut conn,
            &format!("SELECT COUNT(*) AS n FROM shifts WHERE shop_id = {SHOP}")
        ),
        4
    );
}

#[test]
fn a_close_arrives_whole_or_not_at_all() {
    // `shifts_close_is_whole`: without it a row could carry a counted figure
    // and no moment, and every reader downstream would have to decide for
    // itself whether that row is open or closed.
    let (_dir, mut conn) = open_temp();

    for lone in [
        ("closed_at", "'2026-09-21 19:00:00'"),
        ("closed_by", "1"),
        ("counted_centimes", "1200000"),
        ("expected_at_close_centimes", "1200000"),
    ] {
        assert!(
            write_shift(&mut conn, &[lone]).is_err(),
            "{} alone was written onto an open shift",
            lone.0
        );
    }

    // Three of the four is the near miss the CHECK is really for.
    assert!(write_shift(
        &mut conn,
        &[
            ("closed_at", "'2026-09-21 19:00:00'"),
            ("closed_by", "1"),
            ("counted_centimes", "1200000"),
        ]
    )
    .is_err());

    // All four together, and the row stands.
    let whole = closed(OWNER, "1200000", "1200000", "NULL");
    assert!(write_shift(&mut conn, &borrowed(&whole)).is_ok());
    // An open shift with none of the four is the other accepted arm, and
    // the one every till writes first.
    assert!(write_shift(&mut conn, &[("opened_by", "1")]).is_ok());
}

#[test]
fn a_count_that_differs_from_the_expected_figure_carries_a_reason() {
    // Ruling 4. Both sides are asserted on purpose: the refusal on its own is
    // satisfied by an unconditional `note IS NOT NULL`, which would refuse
    // every clean close in the shop.
    let (_dir, mut conn) = open_temp();
    seed_a_second_cashier(&mut conn);

    let short_no_reason = closed(OWNER, "1180000", "1200000", "NULL");
    assert!(
        write_shift(&mut conn, &borrowed(&short_no_reason)).is_err(),
        "a drawer 20 000 centimes short was filed with no reason"
    );
    let over_no_reason = closed(OWNER, "1230000", "1200000", "NULL");
    assert!(
        write_shift(&mut conn, &borrowed(&over_no_reason)).is_err(),
        "a drawer over by 30 000 centimes was filed with no reason"
    );

    // A row of spaces is not a reason either. `optional_field` would have
    // turned it into NULL on the way through a service; this is what stops a
    // writer that skips one.
    for blank in ["''", "'   '"] {
        let short_blank_reason = closed(OWNER, "1180000", "1200000", blank);
        assert!(
            write_shift(&mut conn, &borrowed(&short_blank_reason)).is_err(),
            "a drawer 20 000 centimes short was filed with {blank} as its reason"
        );
    }

    // With a reason, the same short close stands.
    let short_with_reason = closed(OWNER, "1180000", "1200000", "'20 000 remis au patron'");
    assert!(
        write_shift(&mut conn, &borrowed(&short_with_reason)).is_ok(),
        "a difference with a written reason was refused"
    );
    // And a clean close needs none. Without this case the CHECK could be
    // `note IS NOT NULL` and every test above would still pass.
    let clean = closed(KARIM, "1200000", "1200000", "NULL");
    assert!(
        write_shift(&mut conn, &borrowed(&clean)).is_ok(),
        "a close whose two figures agree was made to explain itself"
    );
    // A note on a clean close is allowed too: somebody may want to say the
    // day was quiet.
    let chatty = closed(KARIM, "1200000", "1200000", "'journée calme'");
    assert!(write_shift(&mut conn, &borrowed(&chatty)).is_ok());

    // An open shift carrying a note and no figures is the third accepted
    // arm; the first clause of the CHECK is what lets it through.
    assert!(write_shift(&mut conn, &[("note", "'caisse ouverte tard'")]).is_ok());
}

#[test]
fn the_money_columns_of_a_shift_are_integer_centimes() {
    // Rule 6, and the zero and the negative the brief asks each money change
    // to state. A float is refused as a float and as the text of one.
    let (_dir, mut conn) = open_temp();
    // Every accepted probe leaves an open drawer for user 1 behind, and the
    // unique index would then refuse the next one for a reason that has
    // nothing to do with the column under test. The table is emptied between
    // them so each probe is read on its own.
    let accept = |conn: &mut SqliteConnection, literal: &str| {
        let wrote = probe(conn, "shifts", "opening_cash_centimes", literal).is_ok();
        diesel::sql_query("DELETE FROM shifts")
            .execute(conn)
            .unwrap();
        wrote
    };

    // A drawer starting empty is a real answer, so zero is accepted.
    assert!(accept(&mut conn, "0"));
    for bad in ["19.99", "'19.99'", "'abc'", "-1"] {
        assert!(
            probe(&mut conn, "shifts", "opening_cash_centimes", bad).is_err(),
            "shifts.opening_cash_centimes accepted {bad}"
        );
    }
    // The top of the range: i64::MAX is an integer the column takes, and one
    // more than it is a real by the time SQLite has tokenised the literal, so
    // STRICT refuses it before either half of the CHECK is reached. Same for
    // the three non-integer literals above: only `-1` gets far enough for the
    // sign bound to answer. The `typeof` halves stay as a statement of intent,
    // which is what `migration.rs`'s
    // `a_lossless_real_is_stored_as_an_integer_by_strict_itself` pins.
    assert!(accept(&mut conn, "9223372036854775807"));
    assert!(probe(
        &mut conn,
        "shifts",
        "opening_cash_centimes",
        "9223372036854775808"
    )
    .is_err());

    // The counted figure is what somebody held in their hands, so it is never
    // negative and never a fraction of a centime. It only travels with the
    // other three close columns, so each case writes all four.
    for (counted, expected, note, ok) in [
        ("0", "0", "NULL", true),
        ("1.5", "1", "'x'", false),
        ("'abc'", "1", "'x'", false),
        ("-1", "0", "'x'", false),
        // The expected figure's own type, refused by STRICT the same way.
        ("1", "1.5", "'x'", false),
        ("1", "'abc'", "'x'", false),
    ] {
        let row = closed(OWNER, counted, expected, note);
        assert_eq!(
            write_shift(&mut conn, &borrowed(&row)).is_ok(),
            ok,
            "counted_centimes {counted} was not handled as expected"
        );
    }

    // The expected figure carries no sign bound, on purpose: a cash refund
    // leaving the drawer subtracts from it, so a drawer that paid out more
    // than it took in is arithmetic and not an error. Its type is held by
    // STRICT alone, like every other integer column in the file; what is left
    // for this case to show is that a negative is taken where its sibling
    // refuses one.
    let paid_out = closed(OWNER, "0", "-5000", "'remboursement en espèces'");
    assert!(write_shift(&mut conn, &borrowed(&paid_out)).is_ok());
}

#[test]
fn the_expense_clock_moves_the_rows_already_in_the_file_and_puts_them_back() {
    // `expenses.created_at` took the column's own default, SQLite's
    // CURRENT_TIMESTAMP, which is UTC while Algiers is an hour ahead all
    // year. Seeded before the migration rather than after it, because a
    // migration that stamps new rows and leaves the old ones where they are
    // is the whole bug this half exists to stop.
    use diesel_migrations::MigrationHarness;
    let (_dir, mut conn) = open_before_migration("shifts");
    diesel::sql_query(
        "INSERT INTO expenses (id, shop_id, category_id, amount_centimes, expense_date, \
         user_id, created_at) VALUES \
         (1, 1, 1, 300000, '2026-09-11', 1, '2026-09-11 23:30:00'), \
         (2, 1, 1, 150000, '2026-09-11', 1, '2026-09-11 10:00:00')",
    )
    .execute(&mut conn)
    .unwrap();

    let at = |conn: &mut SqliteConnection, id: i32, moment: &str| {
        count(
            conn,
            &format!(
                "SELECT COUNT(*) AS n FROM expenses WHERE id = {id} AND created_at = '{moment}'"
            ),
        )
    };

    let pending = conn
        .pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    conn.run_migration(&pending[0]).unwrap();

    // 23:30 UTC on the eleventh is 00:30 on the twelfth in Algiers, which is
    // the row the owner filtering for the twelfth could not find.
    assert_eq!(at(&mut conn, 1, "2026-09-12 00:30:00"), 1);
    assert_eq!(at(&mut conn, 2, "2026-09-11 11:00:00"), 1);

    // The table the same migration adds is there beside them.
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'shifts'"
        ),
        1
    );

    conn.revert_last_migration(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    assert_eq!(
        at(&mut conn, 1, "2026-09-11 23:30:00"),
        1,
        "the down.sql did not put the hour back"
    );
    assert_eq!(at(&mut conn, 2, "2026-09-11 10:00:00"), 1);
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM sqlite_master \
             WHERE name IN ('shifts', 'idx_shifts_one_open_per_user', \
                            'idx_shifts_shop_opened_at')"
        ),
        0,
        "the down.sql left the table or one of its indexes behind"
    );

    // And once more up, so the two halves are read in both directions from
    // one seeded file.
    conn.run_pending_migrations(dzpos_retail::db::MIGRATIONS)
        .unwrap();
    assert_eq!(at(&mut conn, 1, "2026-09-12 00:30:00"), 1);
    assert!(diesel::sql_query(insert_with("shifts", "opened_by", "1"))
        .execute(&mut conn)
        .is_ok());
}
