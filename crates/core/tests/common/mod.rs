// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// Every test binary compiles this whole file and calls part of it, so the
// helpers a given binary has no use for are dead code from where it stands.
#![allow(dead_code)]

//! What a test in this directory needs before it can say anything: a
//! database of its own, somebody to sell to, and where the golden pages
//! live.
//!
//! Nothing here decides anything. A helper that made a choice the test
//! should be making would hide the very thing the test is about, so the
//! fiches below are as empty as a fiche is allowed to be and a test that
//! needs a limit, a note or an opening balance sets it itself.

use std::path::PathBuf;

use diesel::SqliteConnection;
use dzpos_core::models::shop::StoreBlock;
use dzpos_core::services::customers::{self, NewCustomer, PartyKind};
use dzpos_core::services::shops;

/// The seeded shop and the user who owns it, both written by the first
/// migration. Every test file declares them again for its own direct calls;
/// these are the ones the helpers below use.
const SHOP: i32 = 1;
const OWNER: i32 = 1;

/// A database of this test's own, migrated and seeded, in a directory that
/// is deleted when the returned handle drops. The handle comes back first
/// because a test that drops it keeps the connection open on a file that is
/// no longer there.
pub fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let conn = dzpos_core::db::open(&path).unwrap();
    (dir, conn)
}

/// The same database with the seller's own identifiers in the settings. A
/// facture is refused until the shop carries RC and NIS
/// (`a_shop_whose_settings_carry_no_nis_cannot_issue_a_facture_at_all`), so
/// a file whose every test issues one starts here.
pub fn open_temp_selling_factures() -> (tempfile::TempDir, SqliteConnection) {
    let (dir, mut conn) = open_temp();
    shops::update_store(
        &mut conn,
        SHOP,
        OWNER,
        StoreBlock {
            name: "Mon magasin".to_string(),
            rc: Some("16/00-1234567 B 25".to_string()),
            nif: None,
            nis: Some("000216001234567 00".to_string()),
            ai: None,
            address: None,
            phone: None,
        },
    )
    .unwrap();
    (dir, conn)
}

/// A company fiche with nothing optional filled in: no identifiers, no
/// limit, no opening balance. What a test that only needs somebody to owe
/// money asks for.
pub fn a_fiche(name: &str) -> NewCustomer {
    NewCustomer {
        name: name.to_string(),
        party_kind: PartyKind::Company,
        phone: None,
        address: None,
        rc: None,
        nif: None,
        nis: None,
        ai: None,
        credit_limit: None,
        warn_threshold: None,
        notes: None,
        active: true,
    }
}

/// The same fiche with the identifiers décret 05-468 art. 3 asks of a
/// buyer. A facture made out to `a_fiche` is refused for want of them, so a
/// test about factures, avoirs or cancellations starts here instead.
pub fn an_identified_fiche(name: &str) -> NewCustomer {
    NewCustomer {
        rc: Some("16/00-7654321 B 22".to_string()),
        nis: Some("000216007654321 00".to_string()),
        ..a_fiche(name)
    }
}

/// `a_fiche`, written, and its id.
pub fn a_customer(conn: &mut SqliteConnection, name: &str) -> i32 {
    customers::create(conn, SHOP, OWNER, a_fiche(name), None)
        .unwrap()
        .id
}

/// `an_identified_fiche`, written, and its id.
pub fn an_identified_customer(conn: &mut SqliteConnection, name: &str) -> i32 {
    customers::create(conn, SHOP, OWNER, an_identified_fiche(name), None)
        .unwrap()
        .id
}

/// Where the golden pages of one template live. The name is the directory
/// under `fixtures/print/`, which is also the name of the print module that
/// renders it.
pub fn goldens_dir(template: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/print")
        .join(template)
}
