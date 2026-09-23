// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `services::day_list`, item 6 of the book tools (C5b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! one day's live appointments in time order beside that day's walk-ins in
//! arrival order, and nothing of another day or another shop.
//!
//! The queue only takes arrivals today, and the book only takes starts from
//! now on, so today's appointments here are raw rows at fixed hours.

use chrono::NaiveDate;
use diesel::{RunQueryDsl, SqliteConnection};
use dzpos_clinic::services::day_list::{self, DayList};
use dzpos_clinic::services::patients::{self, Patient};
use dzpos_clinic::services::queue;
use dzpos_kernel::services::clock;
use dzpos_kernel::services::permissions::Role;

mod common;

use common::book::{at, book, day_ahead, open};
use common::{named, open_temp, second_shop, SHOP};

fn raw(
    conn: &mut SqliteConnection,
    n: u32,
    shop: i32,
    patient: &Patient,
    hh: u32,
    cancelled: bool,
) {
    let starts = at(clock::now().date(), hh, 30).format("%Y-%m-%d %H:%M:%S");
    let cancelled_at = if cancelled {
        "'2026-09-01 08:00:00'"
    } else {
        "NULL"
    };
    diesel::sql_query(format!(
        "INSERT INTO appointments (id, shop_id, patient_id, starts_at, slot_minutes, \
         cancelled_at, created_at, updated_at) VALUES \
         ('0199a0c1-0000-7000-8000-0000000f000{n}', {shop}, '{}', '{starts}', 15, \
         {cancelled_at}, '2026-09-01 08:00:00', '2026-09-01 08:00:00')",
        patient.id
    ))
    .execute(conn)
    .unwrap();
}

/// The list as `HH:MM Name` for the book and `Name` for the queue.
fn read(list: &DayList) -> (Vec<String>, Vec<String>) {
    (
        list.appointments
            .iter()
            .map(|b| {
                format!(
                    "{} {}",
                    b.appointment.starts_at.format("%H:%M"),
                    b.last_name
                )
            })
            .collect(),
        list.walk_ins.iter().map(|q| q.last_name.clone()).collect(),
    )
}

fn day(conn: &mut SqliteConnection, day: NaiveDate) -> DayList {
    day_list::day(conn, SHOP, day).unwrap()
}

#[test]
fn a_day_holds_its_appointments_in_time_order_and_its_walk_ins_in_arrival_order() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let haddad = open(&mut conn, "Karim", "Haddad");
    let cherif = open(&mut conn, "Nadia", "Cherif");
    raw(&mut conn, 1, SHOP, &haddad, 10, false);
    raw(&mut conn, 2, SHOP, &benali, 8, false);
    raw(&mut conn, 3, SHOP, &cherif, 9, true);
    queue::add(&mut conn, SHOP, 1, &cherif.id).unwrap();
    queue::add(&mut conn, SHOP, 1, &benali.id).unwrap();
    // Seen or gone, a walk-in stays on the day's list.
    queue::call_next(&mut conn, SHOP, 1).unwrap();
    let today = clock::now().date();
    let list = day(&mut conn, today);
    assert_eq!(list.day, today);
    assert_eq!(
        read(&list),
        (
            vec!["08:30 Benali".to_string(), "10:30 Haddad".to_string()],
            vec!["Cherif".to_string(), "Benali".to_string()]
        )
    );
    assert!(list.walk_ins[0].entry.called_at.is_some());
}

#[test]
fn another_day_and_another_shop_add_nothing() {
    let (_dir, mut conn) = open_temp();
    let benali = open(&mut conn, "Amina", "Benali");
    let tomorrow = day_ahead(1);
    book(&mut conn, &benali, at(tomorrow, 9, 0)).unwrap();
    queue::add(&mut conn, SHOP, 1, &benali.id).unwrap();
    assert_eq!(
        read(&day(&mut conn, tomorrow)),
        (vec!["09:00 Benali".to_string()], vec![])
    );
    assert_eq!(read(&day(&mut conn, day_ahead(2))), (vec![], vec![]));

    let other = second_shop(&mut conn);
    let theirs =
        patients::create(&mut conn, other, 2, named("Omar", "Ailleurs"), Role::Owner).unwrap();
    raw(&mut conn, 4, other, &theirs, 11, false);
    queue::add(&mut conn, other, 2, &theirs.id).unwrap();
    assert_eq!(
        read(&day(&mut conn, clock::now().date())),
        (vec![], vec!["Benali".to_string()])
    );
}
