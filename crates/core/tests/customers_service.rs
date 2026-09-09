// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The customer fiche (features.md §2), against a real temp SQLite file.
//! Every query is scoped by shop (rule 3), the opening debt is a ledger
//! movement rather than a column, and both writes leave an audit entry with
//! what the row held before and after (features.md §5).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::money::Money;
use dzpos_core::services::customers::{self, NewCustomer, PartyKind};
use dzpos_core::services::{audit, debt};

const SHOP: i32 = 1;
/// The owner the first migration seeds.
const OWNER: i32 = 1;

fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let conn = dzpos_core::db::open(&path).unwrap();
    (dir, conn)
}

fn fiche(name: &str) -> NewCustomer {
    NewCustomer {
        name: name.to_string(),
        party_kind: PartyKind::Company,
        phone: Some("0555 12 34 56".to_string()),
        address: Some("Zone industrielle, Rouiba".to_string()),
        rc: Some("16/00-7654321 B 22".to_string()),
        nif: Some("000216007654321".to_string()),
        nis: None,
        ai: None,
        credit_limit: Some(Money::centimes(5_000_000)),
        warn_threshold: Some(Money::centimes(4_000_000)),
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
    let made = customers::create(&mut conn, SHOP, OWNER, fiche("Entreprise Benali"), None).unwrap();
    let read = customers::get(&mut conn, SHOP, made.id).unwrap();
    assert_eq!(read, made);
    assert_eq!(read.party_kind, PartyKind::Company);
    assert_eq!(read.credit_limit, Some(Money::centimes(5_000_000)));
    assert_eq!(read.nis, None);
    assert!(read.active);
}

#[test]
fn two_customers_may_share_a_name() {
    // Two "Ahmed" walk into the same shop, so there is no UNIQUE on the name
    // and the second create is not an error.
    let (_dir, mut conn) = open_temp();
    let first = customers::create(&mut conn, SHOP, OWNER, fiche("Ahmed"), None).unwrap();
    let second = customers::create(&mut conn, SHOP, OWNER, fiche("Ahmed"), None).unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(customers::list(&mut conn, SHOP).unwrap().len(), 2);
}

#[test]
fn an_opening_debt_is_one_ledger_row_and_the_balance_equals_it() {
    let (_dir, mut conn) = open_temp();
    let made = customers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Entreprise Benali"),
        Some(Money::centimes(250_000)),
    )
    .unwrap();

    let rows = debt::ledger(&mut conn, SHOP, made.id).unwrap();
    assert_eq!(rows.len(), 1, "an opening debt wrote {} rows", rows.len());
    assert_eq!(rows[0].kind, debt::DebtKind::Opening);
    assert_eq!(rows[0].debit, Money::centimes(250_000));
    assert_eq!(rows[0].credit, Money::ZERO);
    assert_eq!(
        debt::balance(&mut conn, SHOP, made.id).unwrap(),
        Money::centimes(250_000)
    );
}

#[test]
fn an_opening_debt_of_nothing_writes_no_row() {
    // A movement of nothing would sit in every statement the customer is
    // handed and say nothing.
    let (_dir, mut conn) = open_temp();
    let made = customers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Entreprise Benali"),
        Some(Money::ZERO),
    )
    .unwrap();
    assert!(debt::ledger(&mut conn, SHOP, made.id).unwrap().is_empty());
}

#[test]
fn a_negative_opening_debt_is_refused_and_no_fiche_is_left_behind() {
    let (_dir, mut conn) = open_temp();
    let err = customers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Entreprise Benali"),
        Some(Money::centimes(-1)),
    )
    .unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "opening_debt"),
        "{err}"
    );
    assert!(customers::list(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn an_update_replaces_the_fiche_and_never_touches_the_ledger() {
    // features.md §2: the ledger is append-only, so a wrong opening debt is
    // corrected by an `adjustment` movement (T2), never by editing the fiche.
    let (_dir, mut conn) = open_temp();
    let made = customers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Entreprise Benali"),
        Some(Money::centimes(250_000)),
    )
    .unwrap();
    let mut changed = fiche("Entreprise Benali et fils");
    changed.credit_limit = None;
    changed.party_kind = PartyKind::Consumer;
    let after = customers::update(&mut conn, SHOP, OWNER, made.id, changed).unwrap();

    assert_eq!(after.name, "Entreprise Benali et fils");
    assert_eq!(after.credit_limit, None, "no limit is not a limit of zero");
    assert_eq!(after.party_kind, PartyKind::Consumer);
    assert_eq!(after.created_at, made.created_at);
    assert_eq!(debt::ledger(&mut conn, SHOP, made.id).unwrap().len(), 1);
    assert_eq!(
        debt::balance(&mut conn, SHOP, made.id).unwrap(),
        Money::centimes(250_000)
    );
}

#[test]
fn another_shop_gets_not_found_on_a_read_and_on_an_update() {
    // Rule 3: the id alone is never enough.
    let (_dir, mut conn) = open_temp();
    second_shop(&mut conn);
    let made = customers::create(&mut conn, SHOP, OWNER, fiche("Entreprise Benali"), None).unwrap();

    for err in [
        customers::get(&mut conn, 2, made.id).unwrap_err(),
        customers::update(&mut conn, 2, OWNER, made.id, fiche("Volé")).unwrap_err(),
    ] {
        assert!(
            matches!(
                err,
                CoreError::NotFound {
                    entity: "customer",
                    ..
                }
            ),
            "{err}"
        );
    }
    assert!(customers::list(&mut conn, 2).unwrap().is_empty());
    assert_eq!(
        customers::get(&mut conn, SHOP, made.id).unwrap().name,
        "Entreprise Benali",
        "the refused update wrote anyway"
    );
    assert!(!customers::customer_belongs_to_shop(&mut conn, 2, made.id).unwrap());
    assert!(customers::customer_belongs_to_shop(&mut conn, SHOP, made.id).unwrap());
}

#[test]
fn a_field_longer_than_a_facture_prints_is_refused() {
    let (_dir, mut conn) = open_temp();
    let mut too_long = fiche("Entreprise Benali");
    too_long.address = Some("é".repeat(201));
    let err = customers::create(&mut conn, SHOP, OWNER, too_long, None).unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "address"),
        "{err}"
    );

    let mut just_fits = fiche("Entreprise Benali");
    just_fits.address = Some("é".repeat(200));
    customers::create(&mut conn, SHOP, OWNER, just_fits, None).unwrap();
}

#[test]
fn a_fiche_with_no_name_is_refused_and_a_blank_identifier_is_stored_as_nothing() {
    let (_dir, mut conn) = open_temp();
    let mut nameless = fiche("   ");
    nameless.nis = Some("   ".to_string());
    let err = customers::create(&mut conn, SHOP, OWNER, nameless, None).unwrap_err();
    assert!(
        matches!(err, CoreError::Validation { ref field, .. } if field == "name"),
        "{err}"
    );

    let mut blanks = fiche("Entreprise Benali");
    blanks.rc = Some("  ".to_string());
    blanks.phone = Some("  0555 12 34 56  ".to_string());
    let made = customers::create(&mut conn, SHOP, OWNER, blanks, None).unwrap();
    assert_eq!(made.rc, None, "a cleared field is stored as nothing");
    assert_eq!(made.phone.as_deref(), Some("0555 12 34 56"));
}

#[test]
fn a_create_and_an_update_each_leave_an_audit_entry_with_before_and_after() {
    let (_dir, mut conn) = open_temp();
    let made = customers::create(
        &mut conn,
        SHOP,
        OWNER,
        fiche("Entreprise Benali"),
        Some(Money::centimes(250_000)),
    )
    .unwrap();
    let mut changed = fiche("Entreprise Benali");
    changed.credit_limit = Some(Money::centimes(9_000_000));
    customers::update(&mut conn, SHOP, OWNER, made.id, changed).unwrap();

    let log = audit::list(&mut conn, SHOP).unwrap();
    assert_eq!(log.len(), 2, "{log:?}");

    let created = &log[0];
    assert_eq!(created.action, "create");
    assert_eq!(created.entity, "customer");
    assert_eq!(created.entity_id, Some(made.id));
    assert_eq!(
        created.before, None,
        "a fiche that did not exist had no before"
    );
    let after = created.after.as_deref().unwrap();
    assert!(after.contains("Entreprise Benali"), "{after}");
    assert!(
        after.contains("\"opening_debt_centimes\":250000"),
        "the opening debt is not in the entry that carried it: {after}"
    );

    let updated = &log[1];
    assert_eq!(updated.action, "update");
    assert_eq!(updated.entity_id, Some(made.id));
    let before = updated.before.as_deref().unwrap();
    let after = updated.after.as_deref().unwrap();
    assert!(
        before.contains("\"credit_limit_centimes\":5000000"),
        "{before}"
    );
    assert!(
        after.contains("\"credit_limit_centimes\":9000000"),
        "{after}"
    );
    assert!(
        !after.contains("opening_debt_centimes"),
        "an update reported an opening debt it never wrote: {after}"
    );
}

#[test]
fn a_refused_fiche_leaves_no_audit_entry() {
    // The entry and the change are one transaction. A log of attempts is not
    // a log of what happened.
    let (_dir, mut conn) = open_temp();
    let mut too_long = fiche("Entreprise Benali");
    too_long.notes = Some("é".repeat(201));
    customers::create(&mut conn, SHOP, OWNER, too_long, None).unwrap_err();
    assert!(audit::list(&mut conn, SHOP).unwrap().is_empty());
}

/// A user id no row carries. `debt_ledger.user_id` and `audit_log.user_id`
/// both have a foreign key to `users`, so a write naming this fails at the
/// file, after the write before it has already landed.
const NO_SUCH_USER: i32 = 999;

#[test]
fn a_create_that_fails_after_the_fiche_leaves_no_fiche() {
    // The fiche goes in first and the opening movement second. Without the
    // transaction the shop would keep a customer whose opening debt was
    // never written and whose creation is in no log.
    let (_dir, mut conn) = open_temp();
    let err = customers::create(
        &mut conn,
        SHOP,
        NO_SUCH_USER,
        fiche("Entreprise Benali"),
        Some(Money::centimes(250_000)),
    )
    .unwrap_err();
    assert_eq!(err.code(), "storage", "{err}");
    assert!(
        customers::list(&mut conn, SHOP).unwrap().is_empty(),
        "the fiche outlived the movement that failed"
    );
    assert!(audit::list(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn an_update_that_fails_at_the_audit_entry_leaves_the_fiche_as_it_was() {
    // The audit entry is the second write of the update, so it is what proves
    // the fiche and its log move together (features.md §5).
    let (_dir, mut conn) = open_temp();
    let made = customers::create(&mut conn, SHOP, OWNER, fiche("Entreprise Benali"), None).unwrap();
    let err = customers::update(
        &mut conn,
        SHOP,
        NO_SUCH_USER,
        made.id,
        fiche("Entreprise Benali et fils"),
    )
    .unwrap_err();
    assert_eq!(err.code(), "storage", "{err}");
    assert_eq!(
        customers::get(&mut conn, SHOP, made.id).unwrap().name,
        "Entreprise Benali",
        "the fiche changed while its audit entry did not"
    );
    assert_eq!(
        audit::list(&mut conn, SHOP).unwrap().len(),
        1,
        "the create's entry is the only one that should be there"
    );
}
