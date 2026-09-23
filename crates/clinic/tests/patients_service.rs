// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `services::patients`, C3 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`:
//! open a file, correct it, find it, archive it, and never see another
//! shop's. Every expected value is written out here, never read back off
//! the service under test.

use chrono::NaiveDate;
use diesel::RunQueryDsl;
use dzpos_clinic::audit_actions::{
    ACTION_PATIENT_ARCHIVE, ACTION_PATIENT_CREATE, ACTION_PATIENT_UPDATE,
};
use dzpos_clinic::services::patients::{self, NewPatient, Sex};
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::audit;

mod common;

use common::{named, open_temp, second_shop, OWNER, SHOP};

fn names(found: &[patients::Patient]) -> Vec<String> {
    found
        .iter()
        .map(|p| format!("{} {}", p.first_name, p.last_name))
        .collect()
}

fn field_of(err: CoreError) -> String {
    match err {
        CoreError::Validation { field, .. } => field,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn a_file_created_is_found_by_a_search_on_its_name() {
    let (_dir, mut conn) = open_temp();
    let made = patients::create(&mut conn, SHOP, OWNER, named("Amina", "Benali")).unwrap();
    patients::create(&mut conn, SHOP, OWNER, named("Karim", "Haddad")).unwrap();

    let found = patients::search(&mut conn, SHOP, Some("benal"), false).unwrap();
    assert_eq!(found, vec![made.clone()]);
    // Case-insensitive, both names in either order, and a blank box is the
    // whole list in surname order.
    assert_eq!(
        names(&patients::search(&mut conn, SHOP, Some("AMINA"), false).unwrap()),
        ["Amina Benali"]
    );
    assert_eq!(
        names(&patients::search(&mut conn, SHOP, Some("amina ben"), false).unwrap()),
        ["Amina Benali"]
    );
    assert_eq!(
        names(&patients::search(&mut conn, SHOP, Some("benali am"), false).unwrap()),
        ["Amina Benali"]
    );
    assert_eq!(
        names(&patients::search(&mut conn, SHOP, Some("   "), false).unwrap()),
        ["Amina Benali", "Karim Haddad"]
    );
    assert_eq!(patients::get(&mut conn, SHOP, &made.id).unwrap(), made);
}

#[test]
fn a_file_is_found_by_its_number_however_the_digits_were_spaced() {
    let (_dir, mut conn) = open_temp();
    let mut p = named("Yacine", "Mansouri");
    p.phone = Some(" 0555 12-34.56 ".to_string());
    let made = patients::create(&mut conn, SHOP, OWNER, p).unwrap();
    // Stored as its digits alone.
    assert_eq!(made.phone.as_deref(), Some("0555123456"));

    for typed in ["0555123456", "12 34", "555-12", "(0555)"] {
        assert_eq!(
            names(&patients::search(&mut conn, SHOP, Some(typed), false).unwrap()),
            ["Yacine Mansouri"],
            "{typed}"
        );
    }
    // A search box with no digit in it never reaches the phone column.
    assert!(patients::search(&mut conn, SHOP, Some("-"), false)
        .unwrap()
        .is_empty());
}

#[test]
fn a_wildcard_typed_in_the_box_is_a_character_and_not_the_whole_list() {
    let (_dir, mut conn) = open_temp();
    patients::create(&mut conn, SHOP, OWNER, named("Amina", "Benali")).unwrap();
    assert!(patients::search(&mut conn, SHOP, Some("%"), false)
        .unwrap()
        .is_empty());
    assert!(patients::search(&mut conn, SHOP, Some("_"), false)
        .unwrap()
        .is_empty());
}

#[test]
fn an_archived_file_leaves_the_search_unless_it_is_asked_for() {
    let (_dir, mut conn) = open_temp();
    let made = patients::create(&mut conn, SHOP, OWNER, named("Amina", "Benali")).unwrap();
    let archived = patients::archive(&mut conn, SHOP, OWNER, &made.id).unwrap();
    assert!(archived.archived_at.is_some());

    assert!(patients::search(&mut conn, SHOP, Some("benali"), false)
        .unwrap()
        .is_empty());
    assert!(patients::search(&mut conn, SHOP, None, false)
        .unwrap()
        .is_empty());
    assert_eq!(
        patients::search(&mut conn, SHOP, Some("benali"), true).unwrap(),
        vec![archived.clone()]
    );
    // Still readable by id: the queue and the book will point at it.
    assert_eq!(patients::get(&mut conn, SHOP, &made.id).unwrap(), archived);
}

#[test]
fn archiving_twice_is_a_conflict_and_writes_one_audit_row() {
    let (_dir, mut conn) = open_temp();
    let made = patients::create(&mut conn, SHOP, OWNER, named("Amina", "Benali")).unwrap();
    let first = patients::archive(&mut conn, SHOP, OWNER, &made.id).unwrap();

    match patients::archive(&mut conn, SHOP, OWNER, &made.id) {
        Err(CoreError::Conflict { field, .. }) => assert_eq!(field, "archived_at"),
        other => panic!("expected a conflict, got {other:?}"),
    }
    assert_eq!(
        patients::get(&mut conn, SHOP, &made.id)
            .unwrap()
            .archived_at,
        first.archived_at
    );
    assert_eq!(
        audit::by_action(&mut conn, SHOP, ACTION_PATIENT_ARCHIVE)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn another_shops_file_is_invisible() {
    let (_dir, mut conn) = open_temp();
    let other = second_shop(&mut conn);
    let theirs = patients::create(&mut conn, other, 2, named("Amina", "Benali")).unwrap();

    assert!(patients::search(&mut conn, SHOP, Some("benali"), true)
        .unwrap()
        .is_empty());
    for result in [
        patients::get(&mut conn, SHOP, &theirs.id),
        patients::update(&mut conn, SHOP, OWNER, &theirs.id, named("X", "Y")),
        patients::archive(&mut conn, SHOP, OWNER, &theirs.id),
    ] {
        match result {
            Err(CoreError::NotFoundText { entity, id }) => {
                assert_eq!(entity, "patient");
                assert_eq!(id, theirs.id);
            }
            other => panic!("expected not found, got {other:?}"),
        }
    }
    // And the other shop's file is untouched by the refused writes.
    assert_eq!(patients::get(&mut conn, other, &theirs.id).unwrap(), theirs);
}

#[test]
fn two_ids_made_in_a_row_differ_and_sort_in_the_order_they_were_made() {
    let (_dir, mut conn) = open_temp();
    let mut ids = Vec::new();
    // Many in a row, so several land inside the same millisecond: v7's own
    // counter is what has to keep those in order, not the clock.
    for n in 0..50 {
        let made =
            patients::create(&mut conn, SHOP, OWNER, named("P", &format!("{n:02}"))).unwrap();
        assert_eq!(made.id.len(), 36, "{}", made.id);
        // Version 7, in the version nibble of the third group.
        assert_eq!(&made.id[14..15], "7", "{}", made.id);
        ids.push(made.id);
    }
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted, ids, "ids were not unique or not in creation order");
}

#[test]
fn an_update_rewrites_the_whole_file_and_a_blank_clears_a_column() {
    let (_dir, mut conn) = open_temp();
    let mut first = named("Amina", "Benali");
    first.sex = Some(Sex::Female);
    first.date_of_birth = NaiveDate::from_ymd_opt(1990, 5, 17);
    first.phone = Some("0555123456".to_string());
    first.notes = Some("allergique à la pénicilline".to_string());
    let made = patients::create(&mut conn, SHOP, OWNER, first).unwrap();
    assert_eq!(made.sex, Some(Sex::Female));
    assert_eq!(made.date_of_birth, NaiveDate::from_ymd_opt(1990, 5, 17));
    // Backdated by hand, so an update that forgot to stamp `updated_at`
    // cannot pass by landing in the same second as the create.
    let backdated = NaiveDate::from_ymd_opt(2000, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    diesel::sql_query("UPDATE patients SET updated_at = '2000-01-01 00:00:00'")
        .execute(&mut conn)
        .unwrap();

    let after = patients::update(
        &mut conn,
        SHOP,
        OWNER,
        &made.id,
        NewPatient {
            first_name: "  Amina ".to_string(),
            last_name: "Benali-Saidi".to_string(),
            sex: Some(Sex::Female),
            date_of_birth: None,
            phone: Some("   ".to_string()),
            notes: None,
        },
    )
    .unwrap();
    assert_eq!(after.id, made.id);
    assert_eq!(after.first_name, "Amina");
    assert_eq!(after.last_name, "Benali-Saidi");
    assert_eq!(after.date_of_birth, None);
    assert_eq!(after.phone, None);
    assert_eq!(after.notes, None);
    assert_eq!(after.created_at, made.created_at);
    assert_ne!(after.updated_at, backdated);
    assert!(after.updated_at >= made.updated_at);
}

#[test]
fn an_archived_file_may_still_be_corrected_and_stays_archived() {
    let (_dir, mut conn) = open_temp();
    let made = patients::create(&mut conn, SHOP, OWNER, named("Amina", "Benali")).unwrap();
    let archived = patients::archive(&mut conn, SHOP, OWNER, &made.id).unwrap();
    let after = patients::update(
        &mut conn,
        SHOP,
        OWNER,
        &made.id,
        named("Amina", "Benali-Saidi"),
    )
    .unwrap();
    assert_eq!(after.last_name, "Benali-Saidi");
    assert_eq!(after.archived_at, archived.archived_at);
    assert!(after.archived_at.is_some());
}

#[test]
fn the_list_is_by_surname_then_first_name_whatever_the_creation_order() {
    let (_dir, mut conn) = open_temp();
    // Created out of every order the list could fall back on: surnames
    // backwards, and the two Haddads' first names backwards too.
    for (first, last) in [
        ("Karim", "Zidane"),
        ("Yacine", "Haddad"),
        ("Amina", "Benali"),
        ("Farid", "Haddad"),
    ] {
        patients::create(&mut conn, SHOP, OWNER, named(first, last)).unwrap();
    }
    assert_eq!(
        names(&patients::search(&mut conn, SHOP, None, false).unwrap()),
        [
            "Amina Benali",
            "Farid Haddad",
            "Yacine Haddad",
            "Karim Zidane"
        ]
    );
}

#[test]
fn every_write_leaves_an_audit_row_naming_the_patient_without_the_notes() {
    let (_dir, mut conn) = open_temp();
    let mut p = named("Amina", "Benali");
    p.notes = Some("diabète de type 2".to_string());
    let made = patients::create(&mut conn, SHOP, OWNER, p.clone()).unwrap();
    patients::update(&mut conn, SHOP, OWNER, &made.id, p).unwrap();
    patients::archive(&mut conn, SHOP, OWNER, &made.id).unwrap();

    for action in [
        ACTION_PATIENT_CREATE,
        ACTION_PATIENT_UPDATE,
        ACTION_PATIENT_ARCHIVE,
    ] {
        let rows = audit::by_action(&mut conn, SHOP, action).unwrap();
        assert_eq!(rows.len(), 1, "{action}");
        let row = &rows[0];
        assert_eq!(row.entity, "patient");
        assert_eq!(row.entity_id, None);
        assert_eq!(row.user_id, OWNER);
        let after: serde_json::Value = serde_json::from_str(row.after.as_deref().unwrap()).unwrap();
        assert_eq!(after["id"], made.id.as_str(), "{action}");
        assert_eq!(after["has_notes"], true, "{action}");
        assert!(
            !row.after.as_deref().unwrap().contains("diabète"),
            "{action} copied the notes into the log"
        );
    }
    assert_eq!(
        patients::search(&mut conn, SHOP, None, true).unwrap().len(),
        1
    );
}

#[test]
fn a_refused_file_writes_nothing() {
    let (_dir, mut conn) = open_temp();
    let blank_first = named("  ", "Benali");
    assert_eq!(
        field_of(patients::create(&mut conn, SHOP, OWNER, blank_first).unwrap_err()),
        "first_name"
    );
    let blank_last = named("Amina", "");
    assert_eq!(
        field_of(patients::create(&mut conn, SHOP, OWNER, blank_last).unwrap_err()),
        "last_name"
    );
    let long_name = named(&"a".repeat(201), "Benali");
    assert_eq!(
        field_of(patients::create(&mut conn, SHOP, OWNER, long_name).unwrap_err()),
        "first_name"
    );

    for bad in [
        "0555 12 34 5x",
        "123",
        "+",
        "1234567890123456",
        "++0555123456",
    ] {
        let mut p = named("Amina", "Benali");
        p.phone = Some(bad.to_string());
        assert_eq!(
            field_of(patients::create(&mut conn, SHOP, OWNER, p).unwrap_err()),
            "phone",
            "{bad}"
        );
    }
    // The two ends of the digit count: four and fifteen, a + in front.
    for good in ["1234", "+213555123456", "123456789012345"] {
        let mut p = named("Bord", good);
        p.phone = Some(good.to_string());
        patients::create(&mut conn, SHOP, OWNER, p).unwrap();
    }

    let mut future = named("Amina", "Benali");
    future.date_of_birth = NaiveDate::from_ymd_opt(2999, 1, 1);
    assert_eq!(
        field_of(patients::create(&mut conn, SHOP, OWNER, future).unwrap_err()),
        "date_of_birth"
    );
    let mut ancient = named("Amina", "Benali");
    ancient.date_of_birth = NaiveDate::from_ymd_opt(1899, 12, 31);
    assert_eq!(
        field_of(patients::create(&mut conn, SHOP, OWNER, ancient).unwrap_err()),
        "date_of_birth"
    );
    let mut first_day = named("Doyenne", "Benali");
    first_day.date_of_birth = NaiveDate::from_ymd_opt(1900, 1, 1);
    patients::create(&mut conn, SHOP, OWNER, first_day).unwrap();

    let mut long_notes = named("Amina", "Benali");
    long_notes.notes = Some("n".repeat(patients::MAX_NOTES_CHARS + 1));
    assert_eq!(
        field_of(patients::create(&mut conn, SHOP, OWNER, long_notes).unwrap_err()),
        "notes"
    );
    let mut full_notes = named("Pleine", "Benali");
    full_notes.notes = Some("n".repeat(4000));
    patients::create(&mut conn, SHOP, OWNER, full_notes).unwrap();

    // Only the five accepted files above, and one audit row for each.
    assert_eq!(
        patients::search(&mut conn, SHOP, None, true).unwrap().len(),
        5
    );
    assert_eq!(
        audit::by_action(&mut conn, SHOP, ACTION_PATIENT_CREATE)
            .unwrap()
            .len(),
        5
    );
}

#[test]
fn an_id_nobody_made_is_not_found() {
    let (_dir, mut conn) = open_temp();
    match patients::get(&mut conn, SHOP, "0199a0c1-0000-7000-8000-000000000000") {
        Err(CoreError::NotFoundText { entity, .. }) => assert_eq!(entity, "patient"),
        other => panic!("expected not found, got {other:?}"),
    }
}
