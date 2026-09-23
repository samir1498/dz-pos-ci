// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::repos::testdb::{open, SHOP};

fn draft(name: &str) -> SupplierRowWrite {
    SupplierRowWrite {
        shop_id: SHOP,
        name: name.to_string(),
        phone: None,
        address: None,
        rc: None,
        nif: None,
        nis: None,
        ai: None,
        notes: None,
        active: true,
        updated_at: dzpos_kernel::services::clock::now(),
    }
}

#[test]
fn a_supplier_is_read_back_with_every_field_it_was_written_with() {
    let (_dir, mut conn) = open();
    let mut write = draft("Sarl Amrani");
    write.phone = Some("0550112233".to_string());
    write.rc = Some("16/00-7654321 B 22".to_string());
    write.notes = Some("livre le mardi".to_string());
    let made = insert(&mut conn, &write).unwrap();
    let read = get(&mut conn, SHOP, made.id).unwrap();
    assert_eq!(read, made);
    assert_eq!(read.phone.as_deref(), Some("0550112233"));
    assert_eq!(read.rc.as_deref(), Some("16/00-7654321 B 22"));
    assert_eq!(read.notes.as_deref(), Some("livre le mardi"));
    assert!(read.active);
}

#[test]
fn another_shops_supplier_is_not_found_and_is_not_listed() {
    // Rule 3. The id alone would answer, which is the whole reason every
    // query here carries the shop.
    let (_dir, mut conn) = open();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(&mut conn)
        .unwrap();
    let mine = insert(&mut conn, &draft("Sarl Amrani")).unwrap();
    let mut theirs = draft("Sarl Amrani");
    theirs.shop_id = 2;
    insert(&mut conn, &theirs).unwrap();
    assert!(get(&mut conn, 2, mine.id).is_err());
    assert_eq!(list(&mut conn, SHOP, None).unwrap().len(), 1);
    assert_eq!(list(&mut conn, 2, None).unwrap().len(), 1);
}

#[test]
fn the_list_puts_the_ones_still_dealt_with_first_then_the_name() {
    // The list is read by somebody looking for a supplier to order from,
    // and a deactivated fiche is kept for its ledger rather than for that.
    let (_dir, mut conn) = open();
    let mut closed = draft("Alpha");
    closed.active = false;
    insert(&mut conn, &closed).unwrap();
    insert(&mut conn, &draft("Zoubir")).unwrap();
    insert(&mut conn, &draft("Bensalem")).unwrap();
    let names: Vec<String> = list(&mut conn, SHOP, None)
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, vec!["Bensalem", "Zoubir", "Alpha"]);
}

#[test]
fn the_search_reads_a_piece_of_the_name_or_of_the_phone_and_escapes_a_wildcard() {
    let (_dir, mut conn) = open();
    let mut with_phone = draft("Bensalem");
    with_phone.phone = Some("0660998877".to_string());
    insert(&mut conn, &with_phone).unwrap();
    insert(&mut conn, &draft("Sarl Amrani")).unwrap();
    assert_eq!(list(&mut conn, SHOP, Some("amra")).unwrap().len(), 1);
    assert_eq!(list(&mut conn, SHOP, Some("0660")).unwrap().len(), 1);
    // A `%` typed by accident used to answer the whole list.
    assert!(list(&mut conn, SHOP, Some("%")).unwrap().is_empty());
}

#[test]
fn a_second_supplier_under_one_name_is_the_rule_and_not_a_sql_fault() {
    let (_dir, mut conn) = open();
    insert(&mut conn, &draft("Sarl Amrani")).unwrap();
    let err = insert(&mut conn, &draft("Sarl Amrani")).unwrap_err();
    assert_eq!(err.code(), "conflict", "{err}");
}

#[test]
fn an_update_reaches_every_field_and_can_clear_one() {
    // The screen sends the whole fiche back, so a phone number somebody
    // deleted has to reach the column rather than be read as "leave it".
    let (_dir, mut conn) = open();
    let mut write = draft("Sarl Amrani");
    write.phone = Some("0550112233".to_string());
    let made = insert(&mut conn, &write).unwrap();
    let mut edited = draft("Sarl Amrani et fils");
    edited.active = false;
    let after = update(&mut conn, SHOP, made.id, &edited).unwrap();
    assert_eq!(after.name, "Sarl Amrani et fils");
    assert_eq!(after.phone, None);
    assert!(!after.active);
    assert!(update(&mut conn, 2, made.id, &edited).is_err());
}

#[test]
fn belonging_to_the_shop_is_a_question_the_foreign_key_cannot_answer() {
    let (_dir, mut conn) = open();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(&mut conn)
        .unwrap();
    let mine = insert(&mut conn, &draft("Sarl Amrani")).unwrap();
    assert!(belongs_to_shop(&mut conn, SHOP, mine.id).unwrap());
    assert!(!belongs_to_shop(&mut conn, 2, mine.id).unwrap());
}
