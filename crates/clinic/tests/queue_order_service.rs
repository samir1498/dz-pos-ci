// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The desk's order of the waiting room, C6b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`
//! (Samir 2026-09-23 19:32): a new arrival goes at the end, the desk puts
//! the day in any order, and calling the next patient follows that order.
//! No rule of the software moves anybody.

use diesel::RunQueryDsl;
use dzpos_clinic::audit_actions::ACTION_QUEUE_REORDER;
use dzpos_clinic::services::queue::{self, QueuedPatient};
use dzpos_kernel::services::audit;

mod common;

use common::book::{at, book, invalid, open};
use common::{open_temp, second_shop, OWNER, SHOP};

fn names(list: &[QueuedPatient]) -> Vec<&str> {
    list.iter().map(|q| q.first_name.as_str()).collect()
}

fn places(list: &[QueuedPatient]) -> Vec<i32> {
    list.iter().map(|q| q.entry.position).collect()
}

/// Three walk-ins, Amina then Karim then Sami, and their entry ids.
fn three(conn: &mut diesel::SqliteConnection) -> [String; 3] {
    ["Amina", "Karim", "Sami"].map(|first| {
        let p = open(conn, first, "Test");
        queue::add(conn, SHOP, OWNER, &p.id).unwrap().entry.id
    })
}

#[test]
fn a_new_arrival_goes_after_the_last_place_of_the_desks_order() {
    let (_dir, mut conn) = open_temp();
    let [amina, karim, _] = three(&mut conn);
    let today = queue::today(&mut conn, SHOP).unwrap();
    assert_eq!(places(&today), [1, 2, 3]);

    let moved = queue::reorder(&mut conn, SHOP, OWNER, &[karim, amina]).unwrap();
    assert_eq!(names(&moved), ["Karim", "Amina", "Sami"]);
    assert_eq!(places(&moved), [1, 2, 3]);

    let rania = open(&mut conn, "Rania", "Test");
    let added = queue::add(&mut conn, SHOP, OWNER, &rania.id).unwrap();
    assert_eq!(added.entry.position, 4);
    // A booked patient checking in goes at the end too; the desk places them.
    let booked_patient = open(&mut conn, "Yacine", "Test");
    let booked = book(
        &mut conn,
        &booked_patient,
        at(common::book::day_ahead(1), 9, 0),
    )
    .unwrap();
    let arrived = queue::check_in(&mut conn, SHOP, OWNER, &booked.appointment.id).unwrap();
    assert_eq!(arrived.entry.position, 5);
    let today = queue::today(&mut conn, SHOP).unwrap();
    assert_eq!(names(&today), ["Karim", "Amina", "Sami", "Rania", "Yacine"]);
}

#[test]
fn calling_the_next_follows_the_desks_order_and_not_arrival() {
    let (_dir, mut conn) = open_temp();
    let [_, _, sami] = three(&mut conn);
    queue::reorder(&mut conn, SHOP, OWNER, std::slice::from_ref(&sami)).unwrap();
    let called = queue::call_next(&mut conn, SHOP, OWNER).unwrap();
    assert_eq!(called.entry.id, sami);
    let next = queue::call_next(&mut conn, SHOP, OWNER).unwrap();
    assert_eq!(next.first_name, "Amina");
}

#[test]
fn an_entry_left_out_of_the_list_keeps_its_order_after_the_ones_listed() {
    let (_dir, mut conn) = open_temp();
    let [amina, karim, sami] = three(&mut conn);
    // The desk's order first differs from arrival: Sami, Amina, Karim.
    queue::reorder(&mut conn, SHOP, OWNER, &[sami, amina, karim.clone()]).unwrap();
    // Karim alone to the top: Sami and Amina keep the desk's order after
    // him, not the arrival order that would put Amina before Sami.
    let moved = queue::reorder(&mut conn, SHOP, OWNER, &[karim]).unwrap();
    assert_eq!(names(&moved), ["Karim", "Sami", "Amina"]);
    assert_eq!(places(&moved), [1, 2, 3]);
}

#[test]
fn the_audit_keeps_both_orders_and_an_unchanged_order_writes_nothing() {
    let (_dir, mut conn) = open_temp();
    let [amina, karim, sami] = three(&mut conn);
    let same = [amina.clone(), karim.clone(), sami.clone()];
    queue::reorder(&mut conn, SHOP, OWNER, &same).unwrap();
    assert_eq!(
        audit::by_action(&mut conn, SHOP, ACTION_QUEUE_REORDER)
            .unwrap()
            .len(),
        0
    );
    queue::reorder(&mut conn, SHOP, OWNER, &[sami.clone(), amina.clone()]).unwrap();
    let rows = audit::by_action(&mut conn, SHOP, ACTION_QUEUE_REORDER).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0]
        .before
        .as_deref()
        .unwrap()
        .contains(&format!(r#""ids":["{amina}","{karim}","{sami}"]"#)));
    assert!(rows[0]
        .after
        .as_deref()
        .unwrap()
        .contains(&format!(r#""ids":["{sami}","{amina}","{karim}"]"#)));
}

/// Malformed lists are a 422 on `ids` and write nothing: an id twice, an id
/// nobody made, another shop's entry, and yesterday's.
#[test]
fn a_list_naming_an_entry_twice_or_one_not_in_todays_queue_is_refused() {
    let (_dir, mut conn) = open_temp();
    let [amina, karim, _] = three(&mut conn);
    let other = second_shop(&mut conn);
    diesel::sql_query(
        "INSERT INTO patients (id, shop_id, first_name, last_name, created_at, updated_at) \
         VALUES ('0199a0c1-0000-7000-8000-00000000b001', 2, 'Other', 'Shop', \
         '2026-09-01 08:00:00', '2026-09-01 08:00:00')",
    )
    .execute(&mut conn)
    .unwrap();
    let theirs = queue::add(&mut conn, other, 2, "0199a0c1-0000-7000-8000-00000000b001")
        .unwrap()
        .entry
        .id;
    let yesterday = "0199a0c1-0000-7000-8000-00000000e0ff";
    let amina_patient = queue::get(&mut conn, SHOP, &amina)
        .unwrap()
        .entry
        .patient_id;
    diesel::sql_query(format!(
        "INSERT INTO queue_entries (id, shop_id, patient_id, day, arrived_at, created_at, \
         updated_at, position) VALUES ('{yesterday}', 1, '{amina_patient}', '2026-01-01', \
         '2026-01-01 09:00:00', '2026-01-01 09:00:00', '2026-01-01 09:00:00', 1)"
    ))
    .execute(&mut conn)
    .unwrap();

    for list in [
        vec![karim.clone(), amina.clone(), karim.clone()],
        vec!["nobody".to_string()],
        vec![theirs],
        vec![yesterday.to_string(), amina.clone()],
    ] {
        let (field, _) = invalid(queue::reorder(&mut conn, SHOP, OWNER, &list));
        assert_eq!(field, "ids", "{list:?}");
    }
    let today = queue::today(&mut conn, SHOP).unwrap();
    assert_eq!(names(&today), ["Amina", "Karim", "Sami"]);
    assert_eq!(
        audit::by_action(&mut conn, SHOP, ACTION_QUEUE_REORDER)
            .unwrap()
            .len(),
        0
    );
}
