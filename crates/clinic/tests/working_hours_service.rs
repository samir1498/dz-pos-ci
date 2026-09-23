// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `services::working_hours`, item 1 of the book tools (C5b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! a booking or a move fits whole inside one open range of its day, a
//! cabinet with no hours set is refused nothing, and a week is written
//! whole. The week is Algerian, Sunday first. Every expected value is
//! written out here, never read back off the service.

use chrono::Weekday;
use diesel::SqliteConnection;
use dzpos_clinic::audit_actions::ACTION_WORKING_HOURS_SET;
use dzpos_clinic::services::appointments;
use dzpos_clinic::services::slot_length;
use dzpos_clinic::services::working_hours::{self, OpenRange, WorkingWeek};
use dzpos_kernel::services::audit;

mod common;

use common::book::{at, book, invalid, next, open, range};
use common::{open_temp, second_shop, OWNER, SHOP};

/// Sunday to Thursday 08:00-12:00 and 14:00-18:00, Friday and Saturday
/// closed: the Algerian working week with a lunch break.
fn with_lunch() -> Vec<Vec<OpenRange>> {
    let day = vec![range((8, 0), (12, 0)), range((14, 0), (18, 0))];
    vec![
        day.clone(),
        day.clone(),
        day.clone(),
        day.clone(),
        day,
        vec![],
        vec![],
    ]
}

fn set(conn: &mut SqliteConnection, days: Vec<Vec<OpenRange>>) -> WorkingWeek {
    working_hours::set_week(conn, SHOP, OWNER, days).unwrap()
}

fn audit_rows(conn: &mut SqliteConnection) -> usize {
    audit::by_action(conn, SHOP, ACTION_WORKING_HOURS_SET)
        .unwrap()
        .len()
}

#[test]
fn a_cabinet_with_no_hours_set_refuses_no_time_of_any_day() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    assert_eq!(working_hours::week(&mut conn, SHOP).unwrap(), None);
    book(&mut conn, &benali, at(next(Weekday::Fri), 3, 0)).unwrap();
    book(&mut conn, &benali, at(next(Weekday::Sun), 23, 45)).unwrap();
}

#[test]
fn a_booking_outside_the_hours_is_refused_and_one_ending_on_the_close_is_not() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    set(&mut conn, with_lunch());
    let sunday = next(Weekday::Sun);
    for refused in [
        at(sunday, 7, 45),
        at(sunday, 12, 0),
        at(sunday, 13, 45),
        at(sunday, 18, 0),
    ] {
        let (field, message) = invalid(book(&mut conn, &benali, refused));
        assert_eq!(field, "starts_at", "{refused}");
        assert_eq!(message, "this time is outside the cabinet's working hours");
    }
    // The first quarter of each range and the last one, which ends on the
    // close.
    for taken in [
        at(sunday, 8, 0),
        at(sunday, 11, 45),
        at(sunday, 14, 0),
        at(sunday, 17, 45),
    ] {
        book(&mut conn, &benali, taken).unwrap();
    }
}

#[test]
fn a_slot_that_runs_into_the_lunch_break_is_refused() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    set(&mut conn, with_lunch());
    assert_eq!(
        slot_length::set_slot_minutes(&mut conn, SHOP, OWNER, 120).unwrap(),
        120
    );
    let monday = next(Weekday::Mon);
    // 10:00 to 12:00 fits the morning; 12:00 to 14:00 is the break itself.
    book(&mut conn, &benali, at(monday, 10, 0)).unwrap();
    invalid(book(&mut conn, &benali, at(monday, 12, 0)));
    assert_eq!(
        slot_length::set_slot_minutes(&mut conn, SHOP, OWNER, 60).unwrap(),
        60
    );
    // 11:00 starts inside the morning and would end at 12:00: it fits. 13:00
    // starts inside the break.
    let tuesday = next(Weekday::Tue);
    book(&mut conn, &benali, at(tuesday, 11, 0)).unwrap();
    invalid(book(&mut conn, &benali, at(tuesday, 13, 0)));
    // On a 25-minute grid 11:40 starts inside the morning and runs five
    // minutes into the break; 11:15 ends at 11:40. The same at the close:
    // 17:55 would end at 18:20.
    assert_eq!(
        slot_length::set_slot_minutes(&mut conn, SHOP, OWNER, 25).unwrap(),
        25
    );
    let wednesday = next(Weekday::Wed);
    invalid(book(&mut conn, &benali, at(wednesday, 11, 40)));
    invalid(book(&mut conn, &benali, at(wednesday, 17, 55)));
    book(&mut conn, &benali, at(wednesday, 11, 15)).unwrap();
}

#[test]
fn friday_and_saturday_with_no_range_are_closed_and_sunday_opens_the_week() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    set(&mut conn, with_lunch());
    invalid(book(&mut conn, &benali, at(next(Weekday::Fri), 9, 0)));
    invalid(book(&mut conn, &benali, at(next(Weekday::Sat), 9, 0)));
    book(&mut conn, &benali, at(next(Weekday::Sun), 9, 0)).unwrap();
    book(&mut conn, &benali, at(next(Weekday::Thu), 9, 0)).unwrap();
}

#[test]
fn a_range_open_to_midnight_takes_the_last_slot_and_nothing_runs_past_it() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    // Wednesday open until midnight, Thursday from midnight: two ranges on
    // two days, which never join into one.
    let mut days = vec![vec![]; 7];
    days[3] = vec![range((22, 0), (24, 0))];
    days[4] = vec![range((0, 0), (2, 0))];
    set(&mut conn, days);
    let wednesday = next(Weekday::Wed);
    book(&mut conn, &benali, at(wednesday, 23, 45)).unwrap();
    // A 25-minute grid does not divide the day: 23:45 is on it (57 x 25)
    // and runs ten minutes into Thursday; 23:20 ends on midnight.
    assert_eq!(
        slot_length::set_slot_minutes(&mut conn, SHOP, OWNER, 25).unwrap(),
        25
    );
    let next_week = wednesday + chrono::Duration::days(7);
    invalid(book(&mut conn, &benali, at(next_week, 23, 45)));
    book(&mut conn, &benali, at(next_week, 23, 20)).unwrap();
    book(
        &mut conn,
        &benali,
        at(next_week + chrono::Duration::days(1), 0, 0),
    )
    .unwrap();
}

#[test]
fn a_move_outside_the_hours_is_refused_like_a_booking() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    set(&mut conn, with_lunch());
    let sunday = next(Weekday::Sun);
    let made = book(&mut conn, &benali, at(sunday, 9, 0)).unwrap();
    let id = made.appointment.id;
    invalid(appointments::move_to(
        &mut conn,
        SHOP,
        OWNER,
        &id,
        at(sunday, 12, 30),
    ));
    invalid(appointments::move_to(
        &mut conn,
        SHOP,
        OWNER,
        &id,
        at(next(Weekday::Sat), 9, 0),
    ));
    let moved = appointments::move_to(&mut conn, SHOP, OWNER, &id, at(sunday, 15, 0)).unwrap();
    assert_eq!(moved.appointment.starts_at, at(sunday, 15, 0));
}

#[test]
fn a_week_is_seven_days_of_ordered_ranges_with_at_least_one_open() {
    let (_dir, mut conn) = open_temp();
    let mut refused: Vec<Vec<Vec<OpenRange>>> =
        vec![vec![vec![]; 6], vec![vec![]; 8], vec![vec![]; 7]];
    let one_day = |ranges: Vec<OpenRange>| {
        let mut days = vec![vec![]; 7];
        days[0] = ranges;
        days
    };
    refused.push(one_day(vec![range((9, 0), (9, 0))]));
    refused.push(one_day(vec![range((10, 0), (9, 0))]));
    refused.push(one_day(vec![range((20, 0), (24, 5))]));
    refused.push(one_day(vec![OpenRange {
        opens_minute: -5,
        closes_minute: 60,
    }]));
    refused.push(one_day(vec![
        range((14, 0), (18, 0)),
        range((8, 0), (12, 0)),
    ]));
    refused.push(one_day(vec![
        range((8, 0), (12, 0)),
        range((11, 0), (18, 0)),
    ]));
    refused.push(one_day(vec![
        range((8, 0), (12, 0)),
        range((12, 0), (18, 0)),
    ]));
    for days in refused {
        let (field, _) = invalid(working_hours::set_week(
            &mut conn,
            SHOP,
            OWNER,
            days.clone(),
        ));
        assert_eq!(field, "days", "{days:?}");
    }
    assert_eq!(working_hours::week(&mut conn, SHOP).unwrap(), None);
    assert_eq!(audit_rows(&mut conn), 0);

    let taken = one_day(vec![range((0, 0), (12, 0)), range((12, 5), (24, 0))]);
    let week = set(&mut conn, taken.clone());
    assert_eq!(week.days.to_vec(), taken);
    assert_eq!(working_hours::week(&mut conn, SHOP).unwrap(), Some(week));
}

#[test]
fn a_new_week_replaces_the_old_one_whole_and_the_same_week_writes_nothing() {
    let (_dir, mut conn) = open_temp();
    set(&mut conn, with_lunch());
    set(&mut conn, with_lunch());
    assert_eq!(audit_rows(&mut conn), 1);
    let mut saturday_morning = vec![vec![]; 7];
    saturday_morning[6] = vec![range((9, 0), (13, 0))];
    set(&mut conn, saturday_morning.clone());
    assert_eq!(audit_rows(&mut conn), 2);
    let week = working_hours::week(&mut conn, SHOP).unwrap().unwrap();
    assert_eq!(week.days.to_vec(), saturday_morning);
    let rows = audit::by_action(&mut conn, SHOP, ACTION_WORKING_HOURS_SET).unwrap();
    let last = rows.iter().max_by_key(|r| r.id).unwrap();
    assert_eq!(last.user_id, OWNER);
    assert_eq!(
        last.after.as_deref(),
        Some(r#"{"days":[[],[],[],[],[],[],[[540,780]]]}"#)
    );
    assert_eq!(
        last.before.as_deref(),
        Some(
            r#"{"days":[[[480,720],[840,1080]],[[480,720],[840,1080]],[[480,720],[840,1080]],[[480,720],[840,1080]],[[480,720],[840,1080]],[],[]]}"#
        )
    );
}

#[test]
fn another_shops_hours_do_not_close_this_one() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let other = second_shop(&mut conn);
    working_hours::set_week(&mut conn, other, 2, with_lunch()).unwrap();
    assert_eq!(working_hours::week(&mut conn, SHOP).unwrap(), None);
    book(&mut conn, &benali, at(next(Weekday::Fri), 21, 0)).unwrap();
}
