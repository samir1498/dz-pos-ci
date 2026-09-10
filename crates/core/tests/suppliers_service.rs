// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The supplier fiche (features.md §1, Supplier), against a real temp SQLite
//! file. The mirror of `customers_service` on the supply side: every query is
//! scoped by shop (rule 3), the opening debt is a ledger movement rather than
//! a column, and both writes leave an audit entry with what the row held
//! before and after (features.md §5).
//!
//! What differs from the customer side, and is tested here because of it: the
//! name is unique inside the shop, there is no credit limit to check, and a
//! fiche is closed through `close` as well as through an update.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::money::Money;
use dzpos_core::services::suppliers::{self, NewSupplier};
use dzpos_core::services::{audit, supplier_debt};

const SHOP: i32 = 1;
/// The owner the first migration seeds.
const OWNER: i32 = 1;
/// A user id no row carries. `supplier_ledger.user_id` and `audit_log.user_id`
/// both have a foreign key to `users`, so a write naming this fails at the
/// file, after the write before it has already landed.
const NO_SUCH_USER: i32 = 999;

mod common;

use common::{a_purchase_ledger_row, a_purchase_row, a_supplier_payment_row, open_temp};

fn fiche(name: &str) -> NewSupplier {
    NewSupplier {
        name: name.to_string(),
        phone: Some("0555 12 34 56".to_string()),
        address: Some("Zone industrielle, Rouiba".to_string()),
        rc: Some("16/00-7654321 B 22".to_string()),
        nif: Some("000216007654321".to_string()),
        nis: None,
        ai: None,
        notes: None,
        active: true,
    }
}

fn second_shop(conn: &mut SqliteConnection) {
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(conn)
        .unwrap();
}

#[test]
fn a_fiche_is_created_and_read_back_whole() {
    let (_dir, mut conn) = open_temp();
    let made = suppliers::create(&mut conn, SHOP, OWNER, fiche("Sarl Amrani"), None).unwrap();
    let read = suppliers::get(&mut conn, SHOP, made.id).unwrap();
    assert_eq!(read, made);
    assert_eq!(read.rc.as_deref(), Some("16/00-7654321 B 22"));
    assert_eq!(read.nis, None);
    assert!(read.active);
}

#[test]
fn a_second_fiche_under_the_same_name_is_refused_on_the_name() {
    // features.md §1 asks for a unique name, and the file says so
    // (`UNIQUE (shop_id, name)`). Without the mapping the shop would meet a
    // 500 saying "storage" for a fiche it can fix by typing another name.
    let (_dir, mut conn) = open_temp();
    suppliers::create(&mut conn, SHOP, OWNER, fiche("Sarl Amrani"), None).unwrap();
    let err = suppliers::create(&mut conn, SHOP, OWNER, fiche("Sarl Amrani"), None).unwrap_err();
    assert_eq!(err.code(), "validation", "{err}");
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "name"),
        "{err}"
    );
    // Another shop is free to buy from a supplier of the same name.
    second_shop(&mut conn);
    suppliers::create(&mut conn, 2, OWNER, fiche("Sarl Amrani"), None).unwrap();
}

#[test]
fn a_rename_onto_a_name_the_shop_already_has_is_refused_the_same_way() {
    let (_dir, mut conn) = open_temp();
    suppliers::create(&mut conn, SHOP, OWNER, fiche("Sarl Amrani"), None).unwrap();
    let other = suppliers::create(&mut conn, SHOP, OWNER, fiche("Bensalem"), None).unwrap();
    let err = suppliers::update(&mut conn, SHOP, OWNER, other.id, fiche("Sarl Amrani"), None)
        .unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "name"),
        "{err}"
    );
}

#[test]
fn an_opening_debt_is_one_ledger_row_and_the_balance_equals_it() {
    let (_dir, mut conn) = open_temp();
    let made = suppliers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Sarl Amrani"),
        Some(Money::centimes(250_000)),
    )
    .unwrap();
    let ledger = supplier_debt::ledger(&mut conn, SHOP, made.id).unwrap();
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].debit, Money::centimes(250_000));
    assert_eq!(ledger[0].credit, Money::ZERO);
    assert_eq!(
        ledger[0].purchase_id, None,
        "an opening balance is on no order"
    );
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, made.id).unwrap(),
        Money::centimes(250_000)
    );
    let with = suppliers::get_with_balance(&mut conn, SHOP, made.id).unwrap();
    assert_eq!(with.balance, Money::centimes(250_000));
}

#[test]
fn an_opening_debt_of_nothing_writes_no_movement() {
    let (_dir, mut conn) = open_temp();
    let made = suppliers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Sarl Amrani"),
        Some(Money::ZERO),
    )
    .unwrap();
    assert!(supplier_debt::ledger(&mut conn, SHOP, made.id)
        .unwrap()
        .is_empty());
    // And the log says nothing about one either: a zero in that field reads
    // as an opening balance that was set to nothing, which is a movement
    // somebody would go looking for.
    let entry = audit::list(&mut conn, SHOP).unwrap().remove(0);
    let after = entry.after.unwrap_or_default();
    assert!(!after.contains("opening_debt"), "{after}");
}

#[test]
fn an_opening_balance_the_supplier_owes_the_shop_is_refused() {
    // A debt carried over is money the shop owes. An advance sitting with the
    // supplier is a movement somebody writes, not a fiche field.
    let (_dir, mut conn) = open_temp();
    let err = suppliers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Sarl Amrani"),
        Some(Money::centimes(-1)),
    )
    .unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "opening_debt"),
        "{err}"
    );
}

#[test]
fn a_create_that_fails_after_the_fiche_leaves_no_fiche() {
    // The fiche goes in first and the opening movement second. Without the
    // transaction the shop would keep a supplier whose opening debt was never
    // written and whose creation is in no log.
    let (_dir, mut conn) = open_temp();
    let err = suppliers::create(
        &mut conn,
        SHOP,
        NO_SUCH_USER,
        fiche("Sarl Amrani"),
        Some(Money::centimes(250_000)),
    )
    .unwrap_err();
    assert_eq!(err.code(), "storage", "{err}");
    assert!(suppliers::list(&mut conn, SHOP, None).unwrap().is_empty());
    assert!(audit::list(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn the_create_and_the_update_are_both_logged_with_the_fiche_on_each_side() {
    let (_dir, mut conn) = open_temp();
    let made = suppliers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Sarl Amrani"),
        Some(Money::centimes(250_000)),
    )
    .unwrap();
    suppliers::update(
        &mut conn,
        SHOP,
        OWNER,
        made.id,
        NewSupplier {
            phone: Some("0770 00 00 00".to_string()),
            ..fiche("Sarl Amrani")
        },
        None,
    )
    .unwrap();

    let log = audit::list(&mut conn, SHOP).unwrap();
    assert_eq!(log.len(), 2, "{log:?}");
    assert_eq!(log[0].action, "create");
    assert_eq!(log[0].entity, "supplier");
    assert_eq!(log[0].entity_id, Some(made.id));
    assert_eq!(log[0].before, None);
    let after = log[0].after.as_deref().unwrap();
    assert!(after.contains("Sarl Amrani"), "{after}");
    assert!(
        after.contains("\"opening_debt_centimes\":250000"),
        "the opening debt is not in the entry that carried it: {after}"
    );

    assert_eq!(log[1].action, "update");
    assert_eq!(log[1].entity, "supplier");
    let before = log[1].before.as_deref().unwrap();
    assert!(before.contains("0555 12 34 56"), "{before}");
    let after = log[1].after.as_deref().unwrap();
    assert!(after.contains("0770 00 00 00"), "{after}");
    assert!(
        !after.contains("opening_debt"),
        "an update writes no opening movement, so it says nothing about one: {after}"
    );
}

#[test]
fn an_update_never_touches_the_ledger() {
    let (_dir, mut conn) = open_temp();
    let made = suppliers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Sarl Amrani"),
        Some(Money::centimes(250_000)),
    )
    .unwrap();
    suppliers::update(
        &mut conn,
        SHOP,
        OWNER,
        made.id,
        fiche("Sarl Amrani et fils"),
        None,
    )
    .unwrap();
    assert_eq!(
        supplier_debt::ledger(&mut conn, SHOP, made.id)
            .unwrap()
            .len(),
        1,
        "the opening movement is the only one, and the edit wrote none"
    );
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, made.id).unwrap(),
        Money::centimes(250_000)
    );
}

#[test]
fn the_list_puts_the_ones_still_dealt_with_first_and_carries_each_balance() {
    let (_dir, mut conn) = open_temp();
    let mut closed = fiche("Alpha");
    closed.active = false;
    suppliers::create(&mut conn, SHOP, OWNER, closed, None).unwrap();
    suppliers::create(&mut conn, SHOP, OWNER, fiche("Zoubir"), None).unwrap();
    let owed = suppliers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Bensalem"),
        Some(Money::centimes(100_000)),
    )
    .unwrap();

    let rows = suppliers::list_with_balance(&mut conn, SHOP, None).unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.supplier.name.as_str()).collect();
    assert_eq!(names, vec!["Bensalem", "Zoubir", "Alpha"]);
    // A supplier with no movement is in no sum at all, and owes nothing.
    assert_eq!(rows[1].balance, Money::ZERO);
    assert_eq!(rows[0].supplier.id, owed.id);
    assert_eq!(rows[0].balance, Money::centimes(100_000));
}

#[test]
fn the_search_matches_a_piece_of_the_name_or_of_the_phone() {
    let (_dir, mut conn) = open_temp();
    suppliers::create(&mut conn, SHOP, OWNER, fiche("Sarl Amrani"), None).unwrap();
    suppliers::create(
        &mut conn,
        SHOP,
        OWNER,
        NewSupplier {
            phone: Some("0660 99 88 77".to_string()),
            ..fiche("Bensalem")
        },
        None,
    )
    .unwrap();

    let by_name = suppliers::list(&mut conn, SHOP, Some("amra")).unwrap();
    assert_eq!(by_name.len(), 1);
    assert_eq!(by_name[0].name, "Sarl Amrani");
    let by_phone = suppliers::list(&mut conn, SHOP, Some("0660")).unwrap();
    assert_eq!(by_phone.len(), 1);
    assert_eq!(by_phone[0].name, "Bensalem");
    // A box that has been emptied reads the whole list rather than nothing.
    assert_eq!(
        suppliers::list(&mut conn, SHOP, Some("  ")).unwrap().len(),
        2
    );
    // A wildcard typed by accident is a character to match, not the whole
    // list.
    assert!(suppliers::list(&mut conn, SHOP, Some("%"))
        .unwrap()
        .is_empty());
}

#[test]
fn another_shops_supplier_is_neither_found_nor_listed() {
    let (_dir, mut conn) = open_temp();
    second_shop(&mut conn);
    let mine = suppliers::create(&mut conn, SHOP, OWNER, fiche("Sarl Amrani"), None).unwrap();
    assert!(suppliers::get(&mut conn, 2, mine.id).is_err());
    assert!(suppliers::list(&mut conn, 2, None).unwrap().is_empty());
    assert!(!suppliers::supplier_belongs_to_shop(&mut conn, 2, mine.id).unwrap());
    assert!(suppliers::supplier_belongs_to_shop(&mut conn, SHOP, mine.id).unwrap());
}

#[test]
fn closing_a_fiche_that_still_owes_needs_a_reason_and_logs_it() {
    let (_dir, mut conn) = open_temp();
    let made = suppliers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Sarl Amrani"),
        Some(Money::centimes(150_000)),
    )
    .unwrap();

    let refused = suppliers::close(&mut conn, SHOP, OWNER, made.id, None).unwrap_err();
    assert!(
        matches!(&refused, CoreError::Validation { field, .. } if field == "reason"),
        "{refused}"
    );
    assert!(
        suppliers::get(&mut conn, SHOP, made.id).unwrap().active,
        "a refused close leaves the fiche open"
    );

    let closed = suppliers::close(
        &mut conn,
        SHOP,
        OWNER,
        made.id,
        Some("le fournisseur a fermé".to_string()),
    )
    .unwrap();
    assert!(!closed.active);

    let entry = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .find(|e| e.action == "supplier.close")
        .expect("a close over an open account is logged as one");
    assert_eq!(entry.entity, "supplier");
    assert_eq!(entry.entity_id, Some(made.id));
    let after: serde_json::Value = serde_json::from_str(&entry.after.unwrap_or_default()).unwrap();
    assert_eq!(after["close_reason"], "le fournisseur a fermé");
    assert_eq!(after["balance_centimes"], 150_000);
    assert_eq!(after["open_purchases"], 0);
    assert_eq!(after["active"], false);
}

#[test]
fn a_settled_fiche_is_closed_without_a_reason_and_logged_as_an_update() {
    // The rule is about the account and not about the fiche: a supplier the
    // shop owes nothing is closed the way any field is changed.
    let (_dir, mut conn) = open_temp();
    let made = suppliers::create(&mut conn, SHOP, OWNER, fiche("Sarl Amrani"), None).unwrap();
    let closed = suppliers::close(&mut conn, SHOP, OWNER, made.id, None).unwrap();
    assert!(!closed.active);
    let actions: Vec<String> = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .map(|e| e.action)
        .collect();
    assert_eq!(actions, vec!["create", "update"]);
}

#[test]
fn an_order_still_asking_to_be_paid_keeps_the_account_open_at_a_nil_balance() {
    // The balance is not the whole answer: money paid that settled no order
    // leaves the shop owing nothing overall and one purchase still open, and
    // closing over that is a decision somebody took.
    let (_dir, mut conn) = open_temp();
    let made = suppliers::create(&mut conn, SHOP, OWNER, fiche("Sarl Amrani"), None).unwrap();
    let purchase = a_purchase_row(&mut conn, made.id, "2026-09-01");
    a_purchase_ledger_row(&mut conn, made.id, purchase, 100_000);
    a_supplier_payment_row(&mut conn, made.id, 100_000);
    assert_eq!(
        supplier_debt::balance(&mut conn, SHOP, made.id).unwrap(),
        Money::ZERO
    );

    let refused = suppliers::close(&mut conn, SHOP, OWNER, made.id, None).unwrap_err();
    assert!(
        matches!(&refused, CoreError::Validation { field, .. } if field == "reason"),
        "{refused}"
    );
    suppliers::close(
        &mut conn,
        SHOP,
        OWNER,
        made.id,
        Some("litige sur la livraison".to_string()),
    )
    .unwrap();
    let entry = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .find(|e| e.action == "supplier.close")
        .expect("the close is logged");
    let after: serde_json::Value = serde_json::from_str(&entry.after.unwrap_or_default()).unwrap();
    assert_eq!(after["balance_centimes"], 0);
    assert_eq!(after["open_purchases"], 1);
}

#[test]
fn an_update_that_closes_an_open_account_asks_for_the_same_reason() {
    // The rule lives in one place: the close route and the fiche's own save
    // reach it through the same check.
    let (_dir, mut conn) = open_temp();
    let made = suppliers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Sarl Amrani"),
        Some(Money::centimes(150_000)),
    )
    .unwrap();
    let mut closing = fiche("Sarl Amrani");
    closing.active = false;
    let refused =
        suppliers::update(&mut conn, SHOP, OWNER, made.id, closing.clone(), None).unwrap_err();
    assert!(
        matches!(&refused, CoreError::Validation { field, .. } if field == "reason"),
        "{refused}"
    );
    suppliers::update(
        &mut conn,
        SHOP,
        OWNER,
        made.id,
        closing,
        Some("compte soldé ailleurs".to_string()),
    )
    .unwrap();
    assert!(!suppliers::get(&mut conn, SHOP, made.id).unwrap().active);
}

#[test]
fn a_fiche_that_is_already_closed_is_not_closed_again() {
    let (_dir, mut conn) = open_temp();
    let made = suppliers::create(&mut conn, SHOP, OWNER, fiche("Sarl Amrani"), None).unwrap();
    suppliers::close(&mut conn, SHOP, OWNER, made.id, None).unwrap();
    let err = suppliers::close(&mut conn, SHOP, OWNER, made.id, None).unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "active"),
        "{err}"
    );
}

#[test]
fn a_closed_fiche_is_reopened_without_a_reason() {
    let (_dir, mut conn) = open_temp();
    let made = suppliers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Sarl Amrani"),
        Some(Money::centimes(150_000)),
    )
    .unwrap();
    suppliers::close(&mut conn, SHOP, OWNER, made.id, Some("erreur".to_string())).unwrap();
    let reopened =
        suppliers::update(&mut conn, SHOP, OWNER, made.id, fiche("Sarl Amrani"), None).unwrap();
    assert!(reopened.active);
}

#[test]
fn a_name_is_required_and_bounded() {
    let (_dir, mut conn) = open_temp();
    let mut blank = fiche("Sarl Amrani");
    blank.name = "   ".to_string();
    let err = suppliers::create(&mut conn, SHOP, OWNER, blank, None).unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "name"),
        "{err}"
    );

    let mut long = fiche("Sarl Amrani");
    long.name = "a".repeat(201);
    let err = suppliers::create(&mut conn, SHOP, OWNER, long, None).unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "name"),
        "{err}"
    );
}
