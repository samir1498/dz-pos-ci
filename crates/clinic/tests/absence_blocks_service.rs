// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `services::absence_blocks`, item 2 of the book tools (C5b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! a block refuses what shares a minute with it and takes what only touches
//! it, answers on creation with the live appointments it lands on, and
//! leaves each to the desk to move or cancel. Every expected value is
//! written out here, never read back off the service.

use chrono::{Duration, NaiveDateTime};
use diesel::SqliteConnection;
use dzpos_clinic::audit_actions::{ACTION_ABSENCE_BLOCK_CREATE, ACTION_ABSENCE_BLOCK_REMOVE};
use dzpos_clinic::services::absence_blocks::{self, AbsenceBlock, BlockMade, NewBlock};
use dzpos_clinic::services::appointments::{self, BookedPatient};
use dzpos_clinic::services::slot_length;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::audit;

mod common;

use common::book::{at, book, conflict, day_ahead, invalid, open};
use common::{open_temp, second_shop, OWNER, SHOP};

fn block(
    conn: &mut SqliteConnection,
    from: NaiveDateTime,
    to: NaiveDateTime,
    label: Option<&str>,
) -> Result<BlockMade, CoreError> {
    absence_blocks::create(
        conn,
        SHOP,
        OWNER,
        NewBlock {
            starts_at: from,
            ends_at: to,
            label: label.map(str::to_string),
        },
    )
}

/// Each hit as `HH:MM Name`.
fn hits(made: &BlockMade) -> Vec<String> {
    names(&made.hits)
}

fn names(found: &[BookedPatient]) -> Vec<String> {
    found
        .iter()
        .map(|b| {
            format!(
                "{} {}",
                b.appointment.starts_at.format("%H:%M"),
                b.last_name
            )
        })
        .collect()
}

#[test]
fn a_booking_inside_a_block_is_refused_and_one_touching_either_end_is_not() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let day = day_ahead(1);
    block(&mut conn, at(day, 10, 0), at(day, 11, 0), Some("congrès")).unwrap();
    for inside in [at(day, 10, 0), at(day, 10, 45)] {
        let (field, message) = conflict(book(&mut conn, &benali, inside));
        assert_eq!(field, "starts_at");
        assert_eq!(
            message,
            format!("the doctor is away from {day} 10:00 to {day} 11:00 (congrès)")
        );
    }
    // 09:45 ends as the block starts; 11:00 starts as it ends.
    book(&mut conn, &benali, at(day, 9, 45)).unwrap();
    book(&mut conn, &benali, at(day, 11, 0)).unwrap();
    // A slot that starts before a block and runs into it is refused too.
    let later = day_ahead(2);
    block(&mut conn, at(later, 10, 10), at(later, 10, 30), None).unwrap();
    let (_, message) = conflict(book(&mut conn, &benali, at(later, 10, 0)));
    assert_eq!(
        message,
        format!("the doctor is away from {later} 10:10 to {later} 10:30")
    );
    book(&mut conn, &benali, at(later, 10, 30)).unwrap();
}

#[test]
fn a_new_block_answers_with_every_live_appointment_it_lands_on() {
    let (_dir, mut conn) = open_temp();
    let a = open(&mut conn, "Amina", "Achour");
    let b = open(&mut conn, "Karim", "Belkacem");
    let c = open(&mut conn, "Nadia", "Cherif");
    let d = open(&mut conn, "Omar", "Djebbar");
    let e = open(&mut conn, "Sara", "Elouahabi");
    let day = day_ahead(1);
    book(&mut conn, &a, at(day, 9, 0)).unwrap();
    book(&mut conn, &b, at(day, 9, 45)).unwrap();
    let cancelled = book(&mut conn, &c, at(day, 10, 0)).unwrap();
    appointments::cancel(&mut conn, SHOP, OWNER, &cancelled.appointment.id).unwrap();
    book(&mut conn, &d, at(day, 10, 15)).unwrap();
    assert_eq!(
        slot_length::set_slot_minutes(&mut conn, SHOP, OWNER, 60).unwrap(),
        60
    );
    book(&mut conn, &e, at(day, 12, 0)).unwrap();

    // 09:50 to 10:20 lands on Belkacem (09:45-10:00) and Djebbar
    // (10:15-10:30), not on the cancelled 10:00.
    let made = block(&mut conn, at(day, 9, 50), at(day, 10, 20), None).unwrap();
    assert_eq!(hits(&made), ["09:45 Belkacem", "10:15 Djebbar"]);
    // One that starts inside an hour-long visit already running.
    let made = block(&mut conn, at(day, 12, 30), at(day, 12, 45), None).unwrap();
    assert_eq!(hits(&made), ["12:00 Elouahabi"]);
    // 10:30 to 12:00 only touches Djebbar's end and Elouahabi's start.
    let made = block(&mut conn, at(day, 10, 30), at(day, 12, 0), None).unwrap();
    assert!(made.hits.is_empty(), "{:?}", hits(&made));
    // The block moved and cancelled nothing: the book reads as it did.
    assert_eq!(
        names(&appointments::day(&mut conn, SHOP, day).unwrap()),
        [
            "09:00 Achour",
            "09:45 Belkacem",
            "10:15 Djebbar",
            "12:00 Elouahabi"
        ]
    );
}

#[test]
fn the_desk_moves_a_hit_out_of_the_block_or_cancels_it() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let haddad = open(&mut conn, "Karim", "Haddad");
    let day = day_ahead(1);
    let first = book(&mut conn, &benali, at(day, 9, 0)).unwrap();
    let second = book(&mut conn, &haddad, at(day, 9, 15)).unwrap();
    let made = block(&mut conn, at(day, 8, 0), at(day, 12, 0), Some("absent")).unwrap();
    assert_eq!(hits(&made), ["09:00 Benali", "09:15 Haddad"]);

    let id = first.appointment.id;
    conflict(appointments::move_to(
        &mut conn,
        SHOP,
        OWNER,
        &id,
        at(day, 11, 45),
    ));
    let moved = appointments::move_to(&mut conn, SHOP, OWNER, &id, at(day, 12, 0)).unwrap();
    assert_eq!(moved.appointment.starts_at, at(day, 12, 0));
    appointments::cancel(&mut conn, SHOP, OWNER, &second.appointment.id).unwrap();
    assert_eq!(
        names(&appointments::overlapping(&mut conn, SHOP, at(day, 8, 0), at(day, 12, 0)).unwrap()),
        Vec::<String>::new()
    );
}

#[test]
fn a_block_ends_after_it_starts_on_whole_minutes_with_a_short_label() {
    let (_dir, mut conn) = open_temp();
    let day = day_ahead(1);
    let (field, _) = invalid(block(&mut conn, at(day, 10, 0), at(day, 10, 0), None));
    assert_eq!(field, "ends_at");
    let (field, _) = invalid(block(&mut conn, at(day, 10, 0), at(day, 9, 0), None));
    assert_eq!(field, "ends_at");
    let (field, _) = invalid(block(
        &mut conn,
        at(day, 10, 0) + Duration::seconds(30),
        at(day, 11, 0),
        None,
    ));
    assert_eq!(field, "starts_at");
    let (field, _) = invalid(block(
        &mut conn,
        at(day, 10, 0),
        at(day, 11, 0) + Duration::seconds(1),
        None,
    ));
    assert_eq!(field, "ends_at");
    let (field, _) = invalid(block(
        &mut conn,
        at(day, 10, 0),
        at(day, 11, 0),
        Some(&"x".repeat(61)),
    ));
    assert_eq!(field, "label");
    assert_eq!(
        audit::by_action(&mut conn, SHOP, ACTION_ABSENCE_BLOCK_CREATE)
            .unwrap()
            .len(),
        0
    );

    let sixty = "é".repeat(60);
    let made = block(&mut conn, at(day, 10, 0), at(day, 11, 0), Some(&sixty)).unwrap();
    assert_eq!(made.block.label.as_deref(), Some(sixty.as_str()));
    let made = block(&mut conn, at(day, 12, 0), at(day, 13, 0), Some("  congé  ")).unwrap();
    assert_eq!(made.block.label.as_deref(), Some("congé"));
    let made = block(&mut conn, at(day, 14, 0), at(day, 15, 0), Some("   ")).unwrap();
    assert_eq!(made.block.label, None);
    // Over midnight and several days: a congress.
    block(
        &mut conn,
        at(day, 18, 0),
        at(day_ahead(4), 8, 0),
        Some("congrès"),
    )
    .unwrap();
}

#[test]
fn a_removed_block_frees_its_hours_and_leaves_its_audit_row() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let day = day_ahead(1);
    let made = block(&mut conn, at(day, 10, 0), at(day, 11, 0), None).unwrap();
    conflict(book(&mut conn, &benali, at(day, 10, 0)));
    let removed = absence_blocks::remove(&mut conn, SHOP, OWNER, &made.block.id).unwrap();
    assert_eq!(removed, made.block);
    book(&mut conn, &benali, at(day, 10, 0)).unwrap();
    match absence_blocks::remove(&mut conn, SHOP, OWNER, &made.block.id) {
        Err(CoreError::NotFoundText { entity, .. }) => assert_eq!(entity, "absence_block"),
        other => panic!("expected not found, got {other:?}"),
    }
    let rows = audit::by_action(&mut conn, SHOP, ACTION_ABSENCE_BLOCK_REMOVE).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].user_id, OWNER);
    assert_eq!(rows[0].after, None);
    assert_eq!(
        rows[0].before.as_deref(),
        Some(
            format!(
                r#"{{"ends_at":"{day} 11:00:00","id":"{}","label":null,"starts_at":"{day} 10:00:00"}}"#,
                made.block.id
            )
            .as_str()
        )
    );
}

#[test]
fn upcoming_lists_the_blocks_not_yet_over_and_never_another_shops() {
    let (_dir, mut conn) = open_temp();
    let yesterday = day_ahead(-1);
    let day = day_ahead(1);
    // A past block is taken, and refuses nothing any more.
    block(
        &mut conn,
        at(yesterday, 9, 0),
        at(yesterday, 10, 0),
        Some("hier"),
    )
    .unwrap();
    block(
        &mut conn,
        at(day_ahead(3), 9, 0),
        at(day_ahead(3), 10, 0),
        Some("plus tard"),
    )
    .unwrap();
    block(&mut conn, at(day, 9, 0), at(day, 10, 0), Some("demain")).unwrap();
    // Started yesterday evening, ends tomorrow morning: still running now.
    block(
        &mut conn,
        at(yesterday, 18, 0),
        at(day, 8, 0),
        Some("en cours"),
    )
    .unwrap();
    let other = second_shop(&mut conn);
    absence_blocks::create(
        &mut conn,
        other,
        2,
        NewBlock {
            starts_at: at(day, 8, 0),
            ends_at: at(day, 18, 0),
            label: Some("ailleurs".into()),
        },
    )
    .unwrap();
    let labels = |found: Vec<AbsenceBlock>| -> Vec<String> {
        found.into_iter().filter_map(|b| b.label).collect()
    };
    assert_eq!(
        labels(absence_blocks::upcoming(&mut conn, SHOP).unwrap()),
        ["en cours", "demain", "plus tard"]
    );
    // The other shop's block closes nothing here.
    let benali = open(&mut conn, "Amina", "Benali");
    book(&mut conn, &benali, at(day, 12, 0)).unwrap();
    let theirs = absence_blocks::upcoming(&mut conn, other).unwrap();
    assert_eq!(theirs.len(), 1);
    match absence_blocks::remove(&mut conn, SHOP, OWNER, &theirs[0].id) {
        Err(CoreError::NotFoundText { .. }) => {}
        other => panic!("expected not found, got {other:?}"),
    }
}
