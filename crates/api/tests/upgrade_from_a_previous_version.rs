// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! What happens the first time a new build opens a shop file an older build
//! left behind. The app migrates the file forward, and before it does it
//! puts a copy of the old file beside it, so an update that reads the books
//! wrong can be walked back to the file as it was that morning.
//!
//! Real temp SQLite files throughout, migrated a step at a time by the same
//! embedded migrations the app ships: a test that stubbed the migrator would
//! prove nothing about the one case this exists for.

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel_migrations::MigrationHarness;

const SHOP: i32 = 1;

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = BigInt)]
    n: i64,
}

/// A shop file as the version before this one left it: every migration this
/// build ships except the last, which is the one the upgrade will run.
fn a_file_from_the_previous_version(path: &std::path::Path) {
    let mut conn = SqliteConnection::establish(&path.to_string_lossy()).unwrap();
    conn.batch_execute("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .unwrap();
    let pending = conn.pending_migrations(dzpos_core::db::MIGRATIONS).unwrap();
    assert!(
        pending.len() > 1,
        "this build ships one migration, so there is no previous version to open from"
    );
    let all_but_the_last = pending.len() - 1;
    for migration in pending.iter().take(all_but_the_last) {
        conn.run_migration(migration).unwrap();
    }
    // A shop file with nothing in it is not the case this protects; the
    // copy has to be worth having.
    diesel::sql_query(
        "INSERT INTO products (shop_id, name, unit, cost_centimes, selling_centimes, rate_bps) \
         VALUES (1, 'un café', 'piece', 3000, 5000, 1900)",
    )
    .execute(&mut conn)
    .unwrap();
    dzpos_core::db::checkpoint(&mut conn).unwrap();
}

/// The copies sitting beside the shop file under the pre-upgrade name.
fn upgrade_copies(db: &std::path::Path) -> Vec<dzpos_core::services::backup::Backup> {
    dzpos_core::services::backup::list_upgrade(db).unwrap()
}

fn products_in(path: &std::path::Path) -> i64 {
    let mut conn = dzpos_core::db::open_unmigrated(path).unwrap();
    let row: Count = diesel::sql_query("SELECT count(*) AS n FROM products")
        .get_result(&mut conn)
        .unwrap();
    row.n
}

#[test]
fn opening_a_file_the_previous_version_wrote_leaves_one_copy_of_it_first() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    a_file_from_the_previous_version(&path);

    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    drop(state);

    let copies = upgrade_copies(&path);
    assert_eq!(
        copies.len(),
        1,
        "expected one pre-upgrade copy, got {copies:?}"
    );
    let copy = &copies[0];
    assert!(
        copy.name.starts_with("t.db.before-upgrade-") && copy.name.ends_with(".sqlite"),
        "the copy is named for what it is: {}",
        copy.name
    );

    // The copy is the old file, not a second copy of the new one: it still
    // has the last migration ahead of it, and the live file has none.
    let mut copied = dzpos_core::db::open_unmigrated(&copy.path).unwrap();
    assert_eq!(
        dzpos_core::db::pending_migrations(&mut copied)
            .unwrap()
            .len(),
        1,
        "the copy should be the file as the previous version left it"
    );
    let mut live = dzpos_core::db::open_unmigrated(&path).unwrap();
    assert!(
        dzpos_core::db::pending_migrations(&mut live)
            .unwrap()
            .is_empty(),
        "the live file should have been migrated forward"
    );

    // And it carries the shop's data, which is the only reason to keep it.
    assert_eq!(products_in(&copy.path), 1);
    assert_eq!(products_in(&path), 1);
}

#[test]
fn the_copy_is_good_enough_to_restore_from() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    a_file_from_the_previous_version(&path);
    drop(dzpos_api::AppState::open(&path, SHOP).unwrap());

    let copies = upgrade_copies(&path);
    // `verify` is what the restore screen runs before it will touch the
    // shop file: the copy opens, passes an integrity check, and carries no
    // migration this build does not know. It also reports what is in there.
    let summary = dzpos_core::services::backup::verify(&copies[0].path).unwrap();
    assert_eq!(summary.products, 1);
}

#[test]
fn opening_a_file_that_is_already_current_leaves_no_copy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    a_file_from_the_previous_version(&path);

    drop(dzpos_api::AppState::open(&path, SHOP).unwrap());
    assert_eq!(upgrade_copies(&path).len(), 1);

    // Every start after the upgrade finds nothing pending and writes
    // nothing, or a shop that restarts the till all day would fill its own
    // disk with copies of one file.
    drop(dzpos_api::AppState::open(&path, SHOP).unwrap());
    drop(dzpos_api::AppState::open(&path, SHOP).unwrap());
    assert_eq!(upgrade_copies(&path).len(), 1);
}

#[test]
fn a_brand_new_file_is_not_copied_before_its_first_migration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");

    drop(dzpos_api::AppState::open(&path, SHOP).unwrap());

    assert!(
        upgrade_copies(&path).is_empty(),
        "a copy of an empty file would sit beside the shop file for ever"
    );
}

#[test]
fn the_pre_upgrade_copy_is_not_one_of_the_thirty_daily_ones() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    a_file_from_the_previous_version(&path);
    drop(dzpos_api::AppState::open(&path, SHOP).unwrap());

    // The daily copies live in a folder that prunes to thirty. This one sits
    // beside the shop file, where the sweep cannot reach it.
    let daily = dzpos_core::services::backup::list(&dzpos_api::default_backup_dir(&path)).unwrap();
    assert!(daily.is_empty(), "startup takes no daily copy: {daily:?}");
    assert_eq!(upgrade_copies(&path).len(), 1);
}
