// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `services::queue`, C4 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`:
//! add a patient on arrival, call the next or one out of order, mark seen or
//! gone, one day at a time, and never see another shop's. Every expected
//! value is written out here, never read back off the service under test.

use diesel::RunQueryDsl;
use dzpos_clinic::audit_actions::{
    ACTION_QUEUE_ADD, ACTION_QUEUE_CALL, ACTION_QUEUE_LEFT, ACTION_QUEUE_SEEN,
};
use dzpos_clinic::services::patients::{self, Patient};
use dzpos_clinic::services::queue::{self, QueuedPatient};
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::audit;
use dzpos_kernel::services::permissions::Role;

mod common;

use common::{named, open_temp, second_shop, OWNER, SHOP};

fn open(conn: &mut diesel::SqliteConnection, first: &str, last: &str) -> Patient {
    patients::create(conn, SHOP, OWNER, named(first, last), Role::Owner).unwrap()
}

fn names(found: &[QueuedPatient]) -> Vec<String> {
    found
        .iter()
        .map(|q| format!("{} {}", q.first_name, q.last_name))
        .collect()
}

/// The field a conflict names; anything else fails the test.
fn conflict_on(result: Result<QueuedPatient, CoreError>) -> String {
    match result {
        Err(CoreError::Conflict { field, .. }) => field,
        other => panic!("expected a conflict, got {other:?}"),
    }
}

fn not_found(result: Result<QueuedPatient, CoreError>) -> &'static str {
    match result {
        Err(CoreError::NotFoundText { entity, .. }) => entity,
        other => panic!("expected not found, got {other:?}"),
    }
}

fn audit_rows(conn: &mut diesel::SqliteConnection, shop: i32, action: &str) -> usize {
    audit::by_action(conn, shop, action).unwrap().len()
}

#[test]
fn the_day_is_in_arrival_order_whatever_order_the_files_were_opened_in() {
    let (_dir, mut conn) = open_temp();
    // Files opened Zidane, Benali, Haddad; the waiting room fills Haddad,
    // Benali, Zidane. Ordering by the patient (or by name) reads it back
    // the other way.
    let zidane = open(&mut conn, "Nadia", "Zidane");
    let benali = open(&mut conn, "Amina", "Benali");
    let haddad = open(&mut conn, "Karim", "Haddad");
    for p in [&haddad, &benali, &zidane] {
        queue::add(&mut conn, SHOP, OWNER, &p.id).unwrap();
    }
    assert_eq!(
        names(&queue::today(&mut conn, SHOP).unwrap()),
        ["Karim Haddad", "Amina Benali", "Nadia Zidane"]
    );
}

#[test]
fn call_next_skips_whoever_was_called_or_has_gone() {
    let (_dir, mut conn) = open_temp();
    let a = open(&mut conn, "Amina", "Benali");
    let b = open(&mut conn, "Karim", "Haddad");
    let c = open(&mut conn, "Nadia", "Zidane");
    let d = open(&mut conn, "Yacine", "Mansouri");
    let queued: Vec<QueuedPatient> = [&a, &b, &c, &d]
        .into_iter()
        .map(|p| queue::add(&mut conn, SHOP, OWNER, &p.id).unwrap())
        .collect();

    // The doctor wants Haddad first, out of order, and Zidane walks out.
    let out_of_order = queue::call(&mut conn, SHOP, OWNER, &queued[1].entry.id).unwrap();
    assert_eq!(out_of_order.entry.patient_id, b.id);
    assert!(out_of_order.entry.called_at.is_some());
    queue::mark_left(&mut conn, SHOP, OWNER, &queued[2].entry.id).unwrap();

    let first = queue::call_next(&mut conn, SHOP, OWNER).unwrap();
    assert_eq!(first.entry.patient_id, a.id);
    let second = queue::call_next(&mut conn, SHOP, OWNER).unwrap();
    assert_eq!(second.entry.patient_id, d.id);
    assert_eq!(second.first_name, "Yacine");
    assert_eq!(
        conflict_on(queue::call_next(&mut conn, SHOP, OWNER)),
        "queue"
    );
    // Calling a called patient again is refused by name too.
    assert_eq!(
        conflict_on(queue::call(&mut conn, SHOP, OWNER, &queued[0].entry.id)),
        "called_at"
    );
    // Three calls, three rows: the out-of-order one and two nexts.
    assert_eq!(audit_rows(&mut conn, SHOP, ACTION_QUEUE_CALL), 3);
}

#[test]
fn seen_before_called_is_refused_and_nothing_follows_seen() {
    let (_dir, mut conn) = open_temp();
    let p = open(&mut conn, "Amina", "Benali");
    let added = queue::add(&mut conn, SHOP, OWNER, &p.id).unwrap();
    let id = added.entry.id.clone();

    assert_eq!(
        conflict_on(queue::mark_seen(&mut conn, SHOP, OWNER, &id)),
        "called_at"
    );
    assert_eq!(queue::get(&mut conn, SHOP, &id).unwrap(), added);
    assert_eq!(audit_rows(&mut conn, SHOP, ACTION_QUEUE_SEEN), 0);

    queue::call(&mut conn, SHOP, OWNER, &id).unwrap();
    let seen = queue::mark_seen(&mut conn, SHOP, OWNER, &id).unwrap();
    assert!(seen.entry.seen_at.is_some());
    assert_eq!(seen.entry.left_at, None);
    assert_eq!(audit_rows(&mut conn, SHOP, ACTION_QUEUE_SEEN), 1);

    // Seen is the end: not seen twice, not gone after, not called again.
    assert_eq!(
        conflict_on(queue::mark_seen(&mut conn, SHOP, OWNER, &id)),
        "seen_at"
    );
    assert_eq!(
        conflict_on(queue::mark_left(&mut conn, SHOP, OWNER, &id)),
        "seen_at"
    );
    assert_eq!(
        conflict_on(queue::call(&mut conn, SHOP, OWNER, &id)),
        "seen_at"
    );
    assert_eq!(queue::get(&mut conn, SHOP, &id).unwrap(), seen);
}

#[test]
fn a_patient_may_leave_before_being_seen_and_nothing_follows_that_either() {
    let (_dir, mut conn) = open_temp();
    let p = open(&mut conn, "Amina", "Benali");
    let q = open(&mut conn, "Karim", "Haddad");
    // One walks out while waiting, the other after a call they did not answer.
    let waiting = queue::add(&mut conn, SHOP, OWNER, &p.id).unwrap().entry.id;
    let called = queue::add(&mut conn, SHOP, OWNER, &q.id).unwrap().entry.id;
    queue::call(&mut conn, SHOP, OWNER, &called).unwrap();
    for id in [&waiting, &called] {
        let gone = queue::mark_left(&mut conn, SHOP, OWNER, id).unwrap();
        assert!(gone.entry.left_at.is_some());
        assert_eq!(gone.entry.seen_at, None);
        for refused in [
            queue::mark_left(&mut conn, SHOP, OWNER, id),
            queue::mark_seen(&mut conn, SHOP, OWNER, id),
            queue::call(&mut conn, SHOP, OWNER, id),
        ] {
            assert_eq!(conflict_on(refused), "left_at");
        }
    }
    assert_eq!(audit_rows(&mut conn, SHOP, ACTION_QUEUE_LEFT), 2);
}

#[test]
fn a_second_live_entry_the_same_day_is_refused_and_one_after_seen_or_gone_is_not() {
    let (_dir, mut conn) = open_temp();
    let p = open(&mut conn, "Amina", "Benali");
    let first = queue::add(&mut conn, SHOP, OWNER, &p.id).unwrap();
    assert_eq!(
        conflict_on(queue::add(&mut conn, SHOP, OWNER, &p.id)),
        "patient_id"
    );
    // Called is still live.
    queue::call(&mut conn, SHOP, OWNER, &first.entry.id).unwrap();
    assert_eq!(
        conflict_on(queue::add(&mut conn, SHOP, OWNER, &p.id)),
        "patient_id"
    );
    queue::mark_seen(&mut conn, SHOP, OWNER, &first.entry.id).unwrap();

    // Back in the afternoon: a new entry, which leaves in turn, and a third.
    let second = queue::add(&mut conn, SHOP, OWNER, &p.id).unwrap();
    assert_ne!(second.entry.id, first.entry.id);
    queue::mark_left(&mut conn, SHOP, OWNER, &second.entry.id).unwrap();
    queue::add(&mut conn, SHOP, OWNER, &p.id).unwrap();

    assert_eq!(
        names(&queue::today(&mut conn, SHOP).unwrap()),
        ["Amina Benali", "Amina Benali", "Amina Benali"]
    );
    assert_eq!(audit_rows(&mut conn, SHOP, ACTION_QUEUE_ADD), 3);
}

#[test]
fn another_shops_entry_and_patient_are_invisible() {
    let (_dir, mut conn) = open_temp();
    let theirs_shop = second_shop(&mut conn);
    let their_patient = patients::create(
        &mut conn,
        theirs_shop,
        2,
        named("Amina", "Benali"),
        Role::Owner,
    )
    .unwrap();
    let theirs = queue::add(&mut conn, theirs_shop, 2, &their_patient.id).unwrap();
    let id = theirs.entry.id.clone();

    assert_eq!(not_found(queue::get(&mut conn, SHOP, &id)), "queue entry");
    assert_eq!(
        not_found(queue::call(&mut conn, SHOP, OWNER, &id)),
        "queue entry"
    );
    assert_eq!(
        not_found(queue::mark_seen(&mut conn, SHOP, OWNER, &id)),
        "queue entry"
    );
    assert_eq!(
        not_found(queue::mark_left(&mut conn, SHOP, OWNER, &id)),
        "queue entry"
    );
    // Their patient cannot be queued here either, and their line is not ours.
    assert_eq!(
        not_found(queue::add(&mut conn, SHOP, OWNER, &their_patient.id)),
        "patient"
    );
    assert!(queue::today(&mut conn, SHOP).unwrap().is_empty());
    assert_eq!(
        conflict_on(queue::call_next(&mut conn, SHOP, OWNER)),
        "queue"
    );
    assert_eq!(audit_rows(&mut conn, SHOP, ACTION_QUEUE_ADD), 0);

    // Still waiting, untouched, in its own shop.
    assert_eq!(queue::get(&mut conn, theirs_shop, &id).unwrap(), theirs);
    assert_eq!(
        names(&queue::today(&mut conn, theirs_shop).unwrap()),
        ["Amina Benali"]
    );
}

#[test]
fn an_archived_patient_cannot_be_queued() {
    let (_dir, mut conn) = open_temp();
    let p = open(&mut conn, "Amina", "Benali");
    patients::archive(&mut conn, SHOP, OWNER, &p.id).unwrap();
    assert_eq!(
        conflict_on(queue::add(&mut conn, SHOP, OWNER, &p.id)),
        "patient_id"
    );
    assert!(queue::today(&mut conn, SHOP).unwrap().is_empty());
    assert_eq!(audit_rows(&mut conn, SHOP, ACTION_QUEUE_ADD), 0);
}

#[test]
fn yesterdays_entries_are_not_in_todays_line() {
    let (_dir, mut conn) = open_temp();
    let p = open(&mut conn, "Amina", "Benali");
    let q = open(&mut conn, "Karim", "Haddad");
    let left_over = queue::add(&mut conn, SHOP, OWNER, &p.id).unwrap();
    // Backdated by hand, the day and the arrival together (the table holds
    // the one to the other): Benali arrived yesterday and was never called.
    assert_eq!(
        diesel::sql_query(
            "UPDATE queue_entries SET day = date(day, '-1 day'), \
             arrived_at = datetime(arrived_at, '-1 day')"
        )
        .execute(&mut conn)
        .unwrap(),
        1
    );
    queue::add(&mut conn, SHOP, OWNER, &q.id).unwrap();

    assert_eq!(
        names(&queue::today(&mut conn, SHOP).unwrap()),
        ["Karim Haddad"]
    );
    // Not the next in today's line, and not called in by hand either.
    let next = queue::call_next(&mut conn, SHOP, OWNER).unwrap();
    assert_eq!(next.entry.patient_id, q.id);
    assert_eq!(
        conflict_on(queue::call_next(&mut conn, SHOP, OWNER)),
        "queue"
    );
    assert_eq!(
        conflict_on(queue::call(&mut conn, SHOP, OWNER, &left_over.entry.id)),
        "day"
    );
    // Yesterday's live entry does not hold today's place: Benali comes back.
    queue::add(&mut conn, SHOP, OWNER, &p.id).unwrap();
    assert_eq!(
        names(&queue::today(&mut conn, SHOP).unwrap()),
        ["Karim Haddad", "Amina Benali"]
    );
}

/// The desk's place decides the order since C6b (`queue_order_service.rs`).
/// Two entries on the same place, which the service never writes, fall
/// back to the arrival stamp, not the id. In every other test the two agree
/// (a UUID v7 sorts in the order it was made), so here the second entry's
/// arrival is moved to the start of the same day by hand and both are put
/// on place 1.
#[test]
fn a_tie_in_place_falls_back_to_the_arrival_stamp_even_against_the_id() {
    let (_dir, mut conn) = open_temp();
    let p = open(&mut conn, "Amina", "Benali");
    let q = open(&mut conn, "Karim", "Haddad");
    queue::add(&mut conn, SHOP, OWNER, &p.id).unwrap();
    let later = queue::add(&mut conn, SHOP, OWNER, &q.id).unwrap();
    assert_eq!(
        diesel::sql_query(format!(
            "UPDATE queue_entries SET arrived_at = day || ' 00:00:00' WHERE id = '{}'",
            later.entry.id
        ))
        .execute(&mut conn)
        .unwrap(),
        1
    );
    assert_eq!(
        diesel::sql_query("UPDATE queue_entries SET position = 1")
            .execute(&mut conn)
            .unwrap(),
        2
    );

    assert_eq!(
        names(&queue::today(&mut conn, SHOP).unwrap()),
        ["Karim Haddad", "Amina Benali"]
    );
    let next = queue::call_next(&mut conn, SHOP, OWNER).unwrap();
    assert_eq!(next.entry.id, later.entry.id);
}

/// Seen and gone are not limited to today: a patient called in before
/// midnight is seen, or found gone, after it.
#[test]
fn yesterdays_called_entries_may_still_be_marked_seen_or_gone() {
    let (_dir, mut conn) = open_temp();
    let p = open(&mut conn, "Amina", "Benali");
    let q = open(&mut conn, "Karim", "Haddad");
    let seen = queue::add(&mut conn, SHOP, OWNER, &p.id).unwrap().entry.id;
    let gone = queue::add(&mut conn, SHOP, OWNER, &q.id).unwrap().entry.id;
    queue::call(&mut conn, SHOP, OWNER, &seen).unwrap();
    queue::call(&mut conn, SHOP, OWNER, &gone).unwrap();
    assert_eq!(
        diesel::sql_query(
            "UPDATE queue_entries SET day = date(day, '-1 day'), \
             arrived_at = datetime(arrived_at, '-1 day'), \
             called_at = datetime(called_at, '-1 day')"
        )
        .execute(&mut conn)
        .unwrap(),
        2
    );
    assert!(queue::today(&mut conn, SHOP).unwrap().is_empty());

    let after = queue::mark_seen(&mut conn, SHOP, OWNER, &seen).unwrap();
    assert!(after.entry.seen_at.is_some());
    let after = queue::mark_left(&mut conn, SHOP, OWNER, &gone).unwrap();
    assert!(after.entry.left_at.is_some());
}

#[test]
fn an_id_nobody_made_is_not_found() {
    let (_dir, mut conn) = open_temp();
    let nobody = "0199a0c1-0000-7000-8000-0000000000ff";
    assert_eq!(
        not_found(queue::get(&mut conn, SHOP, nobody)),
        "queue entry"
    );
    assert_eq!(
        not_found(queue::add(&mut conn, SHOP, OWNER, nobody)),
        "patient"
    );
}
