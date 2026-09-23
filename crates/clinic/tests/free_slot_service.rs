// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `services::free_slot`, item 4 of the book tools (C5b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! the earliest start on the grid inside the hours, outside the blocks,
//! running into nothing and not in the past, over 60 days and no further.
//! Every expected start is written out here from the rule.

use chrono::{Duration, NaiveDate, NaiveDateTime, Weekday};
use diesel::SqliteConnection;
use dzpos_clinic::services::absence_blocks::{self, NewBlock};
use dzpos_clinic::services::appointments::{self, NewAppointment};
use dzpos_clinic::services::free_slot::{self, FreeSlot, FreeSlotQuery};
use dzpos_clinic::services::patients::Patient;
use dzpos_clinic::services::visit_types::{self, NewVisitType, VisitType};
use dzpos_clinic::services::working_hours::{self, OpenRange};
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::clock;

mod common;

use common::book::{at, book, conflict, day_ahead, invalid, next, open, range};
use common::{open_temp, second_shop, OWNER, SHOP};

fn search(
    conn: &mut SqliteConnection,
    from: Option<NaiveDate>,
    offset_days: u32,
    visit: Option<&VisitType>,
) -> Result<FreeSlot, CoreError> {
    free_slot::next_free(
        conn,
        SHOP,
        FreeSlotQuery {
            from,
            offset_days,
            visit_type_id: visit.map(|v| v.id.clone()),
        },
    )
}

fn first_free(
    conn: &mut SqliteConnection,
    from: NaiveDate,
    visit: Option<&VisitType>,
) -> Option<NaiveDateTime> {
    search(conn, Some(from), 0, visit).unwrap().starts_at
}

fn kind(conn: &mut SqliteConnection, name: &str, minutes: i32) -> VisitType {
    visit_types::create(
        conn,
        SHOP,
        OWNER,
        NewVisitType {
            name: name.into(),
            minutes,
        },
    )
    .unwrap()
}

fn block(conn: &mut SqliteConnection, from: NaiveDateTime, to: NaiveDateTime) {
    absence_blocks::create(
        conn,
        SHOP,
        OWNER,
        NewBlock {
            starts_at: from,
            ends_at: to,
            label: None,
        },
    )
    .unwrap();
}

fn book_as(
    conn: &mut SqliteConnection,
    patient: &Patient,
    starts_at: NaiveDateTime,
    visit: &VisitType,
) {
    appointments::book(
        conn,
        SHOP,
        OWNER,
        NewAppointment {
            patient_id: patient.id.clone(),
            starts_at,
            note: None,
            visit_type_id: Some(visit.id.clone()),
        },
    )
    .unwrap();
}

/// Sunday to Thursday 08:00-12:00 and 14:00-18:00; Friday and Saturday
/// have no range.
fn algerian_week(conn: &mut SqliteConnection) {
    let day: Vec<OpenRange> = vec![range((8, 0), (12, 0)), range((14, 0), (18, 0))];
    let mut week = vec![day; 5];
    week.extend([vec![], vec![]]);
    working_hours::set_week(conn, SHOP, OWNER, week).unwrap();
}

#[test]
fn with_no_hours_set_the_first_free_start_is_the_first_grid_minute() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let day = day_ahead(1);
    assert_eq!(first_free(&mut conn, day, None), Some(at(day, 0, 0)));
    book(&mut conn, &benali, at(day, 0, 0)).unwrap();
    book(&mut conn, &benali, at(day, 0, 15)).unwrap();
    assert_eq!(first_free(&mut conn, day, None), Some(at(day, 0, 30)));
    let found = search(&mut conn, Some(day), 0, None).unwrap();
    assert_eq!(
        (found.first_day, found.last_day, found.minutes),
        (day, day + Duration::days(59), 15)
    );
}

#[test]
fn from_today_the_answer_is_never_in_the_past() {
    let (_dir, mut conn) = open_temp();
    let before = clock::now();
    let found = search(&mut conn, None, 0, None).unwrap();
    let start = found.starts_at.unwrap();
    // The first quarter-hour of the grid at or after the moment asked.
    assert!(
        start >= before - Duration::seconds(1),
        "{start} before {before}"
    );
    assert!(
        start < before + Duration::minutes(15),
        "{start} after {before}"
    );
    // A day already past starts the search today, not sixty days back.
    let yesterday = search(&mut conn, Some(day_ahead(-1)), 0, None).unwrap();
    assert_eq!(yesterday.first_day, clock::now().date());
    assert!(yesterday.starts_at.unwrap() >= before - Duration::seconds(1));
}

#[test]
fn the_search_crosses_friday_and_saturday_to_sunday_morning() {
    let (_dir, mut conn) = open_temp();
    algerian_week(&mut conn);
    let friday = next(Weekday::Fri);
    let sunday = friday + Duration::days(2);
    assert_eq!(first_free(&mut conn, friday, None), Some(at(sunday, 8, 0)));
    assert_eq!(
        first_free(&mut conn, friday + Duration::days(1), None),
        Some(at(sunday, 8, 0))
    );
    // Thursday closed by a block from its opening: the week edge again.
    let thursday = friday - Duration::days(1);
    block(&mut conn, at(thursday, 8, 0), at(thursday, 18, 0));
    assert_eq!(
        first_free(&mut conn, thursday, None),
        Some(at(sunday, 8, 0))
    );
}

#[test]
fn the_search_skips_what_is_booked_blocked_and_the_lunch_break() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    algerian_week(&mut conn);
    let long = kind(&mut conn, "Bilan", 180);
    let half = kind(&mut conn, "Contrôle", 30);
    let sunday = next(Weekday::Sun);
    assert_eq!(
        first_free(&mut conn, sunday, Some(&long)),
        Some(at(sunday, 8, 0))
    );
    // With 08:00-09:15 away, no three hours fit the morning, and the break
    // is not open: the afternoon's first start.
    block(&mut conn, at(sunday, 8, 0), at(sunday, 9, 15));
    assert_eq!(
        first_free(&mut conn, sunday, Some(&long)),
        Some(at(sunday, 14, 0))
    );
    assert_eq!(first_free(&mut conn, sunday, None), Some(at(sunday, 9, 15)));
    // 09:15 plain, 09:30 half an hour: a half hour next fits at 10:00, a
    // quarter at 10:00 too, and 09:45 is inside the 30-minute visit.
    book(&mut conn, &benali, at(sunday, 9, 15)).unwrap();
    book_as(&mut conn, &benali, at(sunday, 9, 30), &half);
    assert_eq!(first_free(&mut conn, sunday, None), Some(at(sunday, 10, 0)));
    // 10:15 taken leaves 10:00-10:15, too short for half an hour.
    book(&mut conn, &benali, at(sunday, 10, 15)).unwrap();
    assert_eq!(
        first_free(&mut conn, sunday, Some(&half)),
        Some(at(sunday, 10, 30))
    );
    assert_eq!(first_free(&mut conn, sunday, None), Some(at(sunday, 10, 0)));
}

#[test]
fn a_slot_ending_where_a_block_starts_is_free() {
    let (_dir, mut conn) = open_temp();
    algerian_week(&mut conn);
    let half = kind(&mut conn, "Contrôle", 30);
    let sunday = next(Weekday::Sun);
    block(&mut conn, at(sunday, 8, 0), at(sunday, 9, 30));
    block(&mut conn, at(sunday, 10, 0), at(sunday, 12, 0));
    // 09:30-10:00 ends the minute the second block starts: they share none.
    assert_eq!(
        first_free(&mut conn, sunday, Some(&half)),
        Some(at(sunday, 9, 30))
    );
}

#[test]
fn a_four_hour_visit_is_seen_to_its_last_minute() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let four_hours = kind(&mut conn, "Bilan complet", 240);
    let day = day_ahead(2);
    // 08:00 for 240 minutes holds the book to 12:00.
    book_as(&mut conn, &benali, at(day, 8, 0), &four_hours);
    let (field, _) = conflict(book(&mut conn, &benali, at(day, 11, 30)));
    assert_eq!(field, "starts_at");
    let made = absence_blocks::create(
        &mut conn,
        SHOP,
        OWNER,
        NewBlock {
            starts_at: at(day, 11, 30),
            ends_at: at(day, 11, 45),
            label: None,
        },
    )
    .unwrap();
    let hits: Vec<NaiveDateTime> = made.hits.iter().map(|h| h.appointment.starts_at).collect();
    assert_eq!(hits, vec![at(day, 8, 0)]);
    // 22:00 the evening before for 240 minutes holds the day to 02:00, so
    // the search from the day's midnight answers 02:00.
    let next_day = day + Duration::days(1);
    book_as(&mut conn, &benali, at(day, 22, 0), &four_hours);
    assert_eq!(
        first_free(&mut conn, next_day, None),
        Some(at(next_day, 2, 0))
    );
}

#[test]
fn a_range_open_to_midnight_takes_the_last_half_hour_and_nothing_past_it() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let mut week = vec![vec![]; 7];
    week[3] = vec![range((22, 0), (24, 0))];
    working_hours::set_week(&mut conn, SHOP, OWNER, week).unwrap();
    let ninety = kind(&mut conn, "Longue", 90);
    let half = kind(&mut conn, "Contrôle", 30);
    let three_quarters = kind(&mut conn, "Bilan court", 45);
    let wednesday = next(Weekday::Wed);
    book_as(&mut conn, &benali, at(wednesday, 22, 0), &ninety);
    assert_eq!(
        first_free(&mut conn, wednesday, Some(&half)),
        Some(at(wednesday, 23, 30))
    );
    // 45 minutes from 23:30 would end at 00:15 on Thursday, which is closed.
    assert_eq!(
        first_free(&mut conn, wednesday, Some(&three_quarters)),
        Some(at(wednesday + Duration::days(7), 22, 0))
    );
}

#[test]
fn the_search_reads_sixty_days_and_answers_none_past_them() {
    let (_dir, mut conn) = open_temp();
    let from = day_ahead(1);
    // Away from the first minute of the search to the first of its 60th day:
    // that last day is still read.
    block(
        &mut conn,
        at(from, 0, 0),
        at(from + Duration::days(59), 0, 0),
    );
    assert_eq!(
        first_free(&mut conn, from, None),
        Some(at(from + Duration::days(59), 0, 0))
    );
    // One day more and the window has no room.
    block(
        &mut conn,
        at(from + Duration::days(59), 0, 0),
        at(from + Duration::days(60), 0, 0),
    );
    let found = search(&mut conn, Some(from), 0, None).unwrap();
    assert_eq!(found.starts_at, None);
    assert_eq!(found.last_day, from + Duration::days(59));
    // The day after the window is free, and a search from there finds it.
    assert_eq!(
        first_free(&mut conn, from + Duration::days(60), None),
        Some(at(from + Duration::days(60), 0, 0))
    );
}

#[test]
fn see_again_in_fifteen_days_starts_the_search_fifteen_days_on() {
    let (_dir, mut conn) = open_temp();
    let today = clock::now().date();
    let found = search(&mut conn, None, 15, None).unwrap();
    assert_eq!(found.first_day, today + Duration::days(15));
    assert_eq!(found.starts_at, Some(at(today + Duration::days(15), 0, 0)));
    let found = search(&mut conn, Some(day_ahead(2)), 15, None).unwrap();
    assert_eq!(found.starts_at, Some(at(day_ahead(17), 0, 0)));
    search(&mut conn, None, 366, None).unwrap();
    let (field, _) = invalid(search(&mut conn, None, 367, None));
    assert_eq!(field, "offset_days");
}

#[test]
fn another_shops_book_blocks_nothing_and_its_types_are_not_found() {
    let (_dir, mut conn) = open_temp();
    let day = day_ahead(1);
    let other = second_shop(&mut conn);
    absence_blocks::create(
        &mut conn,
        other,
        2,
        NewBlock {
            starts_at: at(day, 0, 0),
            ends_at: at(day, 12, 0),
            label: None,
        },
    )
    .unwrap();
    assert_eq!(first_free(&mut conn, day, None), Some(at(day, 0, 0)));
    let theirs = visit_types::create(
        &mut conn,
        other,
        2,
        NewVisitType {
            name: "Contrôle".into(),
            minutes: 30,
        },
    )
    .unwrap();
    match search(&mut conn, Some(day), 0, Some(&theirs)) {
        Err(CoreError::NotFoundText { entity, .. }) => assert_eq!(entity, "visit_type"),
        other => panic!("expected not found, got {other:?}"),
    }
}
