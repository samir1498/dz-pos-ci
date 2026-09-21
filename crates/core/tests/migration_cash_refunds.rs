// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Migration 000018: the `cash_refunds` table.
//!
//! Every case here goes through a raw INSERT and never through a service.
//! `services::cash_refunds` repeats the amount check for a kinder message,
//! and a test that went through it would stay green with the CHECK and the
//! unique index deleted from `up.sql`, which is exactly what this file is
//! here to hold.
//!
//! In its own file beside `migration_shifts.rs` and not in `migration.rs`,
//! which is at the length `scripts/file-sizes.json` pins it to.

use diesel::prelude::*;

mod common;

use common::migrations::{count, insert_with, insert_with_all, probe};
use common::open_temp;

/// A document for the refund to name. Written out of the shared template, so
/// the row this file hangs its cases on is spelled the way every other
/// migration test spells one.
fn seed_a_document(conn: &mut SqliteConnection) {
    assert_eq!(
        diesel::sql_query(insert_with("documents", "kind", "'ticket'"))
            .execute(conn)
            .unwrap(),
        1
    );
}

/// A second document, for the cases that need two papers.
fn seed_a_second_document(conn: &mut SqliteConnection) {
    assert_eq!(
        diesel::sql_query(insert_with_all(
            "documents",
            &[("kind", "'ticket'"), ("number", "2")]
        ))
        .execute(conn)
        .unwrap(),
        1
    );
}

/// The amount is money and nothing else. Zero is refused as well as
/// negative: a refund of nothing is not an event, and a row saying so would
/// read on the cash position as a drawer that opened when it did not.
///
/// Both sides, because a CHECK satisfied only by the refusals could be an
/// unconditional `0 = 1` and every one of these would still fail.
///
/// A lossless numeric text literal is not on the refused list, and that is
/// STRICT rather than a gap: SQLite converts `'300000'` to the integer before
/// either half of the CHECK is reached, which is what `migration.rs`'s
/// `a_lossless_real_is_stored_as_an_integer_by_strict_itself` pins. The
/// `typeof` half stays as a statement of intent beside the sign bound that
/// does the work.
#[test]
fn the_amount_is_a_positive_integer_and_the_file_says_so() {
    let (_dir, mut conn) = open_temp();
    seed_a_document(&mut conn);
    // Every accepted probe leaves a refund on document 1 behind, and the
    // unique index would then refuse the next one for a reason that has
    // nothing to do with the column under test. The table is emptied between
    // them so each probe is read on its own.
    let accept = |conn: &mut SqliteConnection, literal: &str| {
        let wrote = probe(conn, "cash_refunds", "amount_centimes", literal).is_ok();
        diesel::sql_query("DELETE FROM cash_refunds")
            .execute(conn)
            .unwrap();
        wrote
    };

    for bad in ["0", "-1", "19.99", "'19.99'", "'abc'", "NULL"] {
        assert!(
            !accept(&mut conn, bad),
            "cash_refunds.amount_centimes accepted {bad}"
        );
    }
    // One centime is a refund, so the bound is above zero and not above a
    // dinar.
    assert!(accept(&mut conn, "1"));
    // The top of the range: i64::MAX is an integer the column takes, and one
    // more than it is a real by the time SQLite has tokenised the literal, so
    // STRICT refuses it before the CHECK is reached. Nothing in the app can
    // write either, and the file is what says so rather than the service.
    assert!(accept(&mut conn, "9223372036854775807"));
    assert!(!accept(&mut conn, "9223372036854775808"));
}

/// One refund per document, held by a unique index rather than by the two
/// services that write here remembering to look first.
#[test]
fn one_document_is_refunded_once_and_the_index_is_what_says_so() {
    let (_dir, mut conn) = open_temp();
    seed_a_document(&mut conn);
    seed_a_second_document(&mut conn);

    assert_eq!(
        probe(&mut conn, "cash_refunds", "document_id", "1").unwrap(),
        1
    );
    assert!(
        probe(&mut conn, "cash_refunds", "document_id", "1").is_err(),
        "a second refund against document 1 was accepted"
    );
    // Per document and not per shop: the other paper still takes one.
    assert_eq!(
        probe(&mut conn, "cash_refunds", "document_id", "2").unwrap(),
        1
    );
}

/// The three foreign keys are real and RESTRICT holds the document and the
/// shop in place: a record of money that left the drawer is not something a
/// DELETE elsewhere may empty.
#[test]
fn the_row_names_a_real_shop_a_real_document_and_a_real_person() {
    let (_dir, mut conn) = open_temp();
    seed_a_document(&mut conn);

    for (column, bad) in [
        ("shop_id", "9999"),
        ("document_id", "9999"),
        ("user_id", "9999"),
    ] {
        assert!(
            probe(&mut conn, "cash_refunds", column, bad).is_err(),
            "{column} = {bad} was accepted"
        );
    }

    assert_eq!(
        probe(&mut conn, "cash_refunds", "amount_centimes", "300000").unwrap(),
        1
    );
    // The document it names cannot be deleted out from under it.
    assert!(
        diesel::sql_query("DELETE FROM documents WHERE id = 1")
            .execute(&mut conn)
            .is_err(),
        "the document a refund names was deleted"
    );
    assert!(
        diesel::sql_query("DELETE FROM shops WHERE id = 1")
            .execute(&mut conn)
            .is_err(),
        "the shop a refund belongs to was deleted"
    );
    assert_eq!(
        count(&mut conn, "SELECT COUNT(*) AS n FROM cash_refunds"),
        1
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT COUNT(*) AS n FROM pragma_foreign_key_check"
        ),
        0
    );
}
