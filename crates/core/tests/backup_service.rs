// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The backup service against real files: the copy is a database that opens
//! and holds the same rows, the folder never keeps more than thirty, and a
//! copy that would lose data is refused before anything is restored from it.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::money::Money;
use dzpos_core::services::{backup, products};

const SHOP: i32 = 1;
const SEEDED_CATEGORY: i32 = 1;

fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let conn = dzpos_core::db::open(&path).unwrap();
    (dir, conn)
}

fn draft(name: &str) -> NewProduct {
    NewProduct {
        name: name.to_string(),
        barcode: None,
        category_id: Some(SEEDED_CATEGORY),
        unit: Unit::Piece,
        cost: Money::centimes(820),
        selling: Money::centimes(920),
        wholesale: None,
        qty_on_hand_milli: 24_000,
        low_stock_at_milli: 10_000,
        rate_bps: None,
        active: true,
    }
}

fn at(day: u32, hour: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, day)
        .and_then(|d| d.and_hms_opt(hour, 30, 0))
        .unwrap()
}

#[test]
fn the_copy_opens_and_holds_the_same_rows() {
    let (dir, mut conn) = open_temp();
    products::create(&mut conn, SHOP, draft("Huile Elio 5L")).unwrap();
    products::create(&mut conn, SHOP, draft("Semoule 10kg")).unwrap();
    let backups = dir.path().join("backups");

    let made = backup::create(&mut conn, &backups, at(8, 9)).unwrap();

    assert_eq!(made.name, "dzpos-20260108-093000.sqlite");
    assert_eq!(made.taken_at, at(8, 9));
    assert!(made.bytes > 0, "the copy is empty");
    assert!(made.path.is_file(), "{:?} is not a file", made.path);

    // The copy is a database of its own: it opens and it has the rows.
    let mut copy = dzpos_core::db::open(&made.path).unwrap();
    let names: Vec<String> = products::list(&mut copy, SHOP)
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, vec!["Huile Elio 5L", "Semoule 10kg"]);

    let summary = backup::verify(&made.path).unwrap();
    assert_eq!(summary.products, 2);
    assert_eq!(summary.documents, None, "no documents table in M1 yet");
}

#[test]
fn thirty_one_backups_leave_thirty_and_the_oldest_is_the_one_gone() {
    let (dir, mut conn) = open_temp();
    let backups = dir.path().join("backups");

    let mut made = Vec::new();
    for day in 1..=31 {
        made.push(backup::create(&mut conn, &backups, at(day, 3)).unwrap());
    }

    let kept = backup::list(&backups).unwrap();
    assert_eq!(kept.len(), backup::KEEP);
    assert_eq!(
        kept.first().map(|b| b.taken_at),
        Some(at(31, 3)),
        "the list is newest first"
    );
    assert_eq!(kept.last().map(|b| b.taken_at), Some(at(2, 3)));
    assert!(
        !made[0].path.exists(),
        "the oldest copy is the one that went"
    );
    assert!(made[30].path.exists(), "the newest copy stayed");
}

#[test]
fn a_file_that_is_not_a_database_or_is_torn_fails_verify() {
    let (dir, mut conn) = open_temp();
    let backups = dir.path().join("backups");
    let made = backup::create(&mut conn, &backups, at(8, 9)).unwrap();

    // A page of the copy overwritten: the header still says SQLite, so
    // nothing short of the integrity check notices.
    let mut bytes = std::fs::read(&made.path).unwrap();
    let start = bytes.len() / 2;
    for byte in bytes.iter_mut().skip(start).take(512) {
        *byte = 0x5a;
    }
    std::fs::write(&made.path, &bytes).unwrap();
    let torn = backup::verify(&made.path).unwrap_err();
    assert_eq!(torn.code(), "validation", "{torn}");

    let junk = dir.path().join("dzpos-20260107-093000.sqlite");
    std::fs::write(&junk, b"not a database at all, just text").unwrap();
    assert_eq!(backup::verify(&junk).unwrap_err().code(), "validation");

    let missing = dir.path().join("dzpos-20260106-093000.sqlite");
    assert_eq!(backup::verify(&missing).unwrap_err().code(), "validation");
    assert!(!missing.exists(), "verify created the file it was handed");
}

#[test]
fn a_copy_from_a_newer_app_version_is_refused() {
    let (dir, mut conn) = open_temp();
    let backups = dir.path().join("backups");
    let made = backup::create(&mut conn, &backups, at(8, 9)).unwrap();

    // A migration this build has never heard of: restoring it would run the
    // file backwards through a schema it no longer matches.
    let mut copy = dzpos_core::db::open(&made.path).unwrap();
    diesel::sql_query("INSERT INTO __diesel_schema_migrations (version) VALUES ('29990101000000')")
        .execute(&mut copy)
        .unwrap();
    drop(copy);

    let refused = backup::verify(&made.path).unwrap_err();
    assert_eq!(refused.code(), "validation", "{refused}");
    assert!(
        refused.to_string().contains("29990101000000"),
        "the version that is unknown is named: {refused}"
    );
}

#[test]
fn the_list_is_newest_first_and_ignores_what_the_name_pattern_does_not_match() {
    let (dir, mut conn) = open_temp();
    let backups = dir.path().join("backups");
    for day in [3, 1, 2] {
        backup::create(&mut conn, &backups, at(day, 7)).unwrap();
    }
    // Neighbours in the same folder that are not backups of this app.
    for stranger in [
        "notes.txt",
        "dzpos-2026-01-04.sqlite",
        "dzpos-20260104-073000.sqlite.tmp",
        "dzpos-20260132-073000.sqlite",
    ] {
        std::fs::write(backups.join(stranger), b"x").unwrap();
    }
    std::fs::create_dir(backups.join("dzpos-20260105-073000.sqlite")).unwrap();

    let listed = backup::list(&backups).unwrap();
    let names: Vec<&str> = listed.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "dzpos-20260103-073000.sqlite",
            "dzpos-20260102-073000.sqlite",
            "dzpos-20260101-073000.sqlite",
        ]
    );

    // A folder that was never written to is empty, not an error.
    assert!(backup::list(&dir.path().join("nowhere"))
        .unwrap()
        .is_empty());
}

#[test]
fn a_name_is_read_back_only_when_it_matches_the_pattern() {
    assert_eq!(
        backup::taken_at("dzpos-20260108-093000.sqlite"),
        Some(at(8, 9))
    );
    for bad in [
        "dzpos-20260108-093000.sqlite.bak",
        "dzpos-20260108-093000.db",
        "../dzpos-20260108-093000.sqlite",
        "dzpos-20261308-093000.sqlite",
        "dzpos-20260908-253000.sqlite",
        "DZPOS-20260908-093000.sqlite",
        "",
    ] {
        assert_eq!(backup::taken_at(bad), None, "{bad} was accepted");
    }
}

#[test]
fn a_backup_is_due_when_there_is_none_or_the_newest_is_a_day_old() {
    let now = at(10, 12);
    assert!(backup::is_due(None, now), "no backup at all");
    assert!(backup::is_due(Some(at(9, 11)), now), "25 hours old");
    // The boundary itself: a day exactly is a day, so the copy is due. This
    // is the assertion that goes red if the comparison loosens to "more
    // than a day", which would push every copy an hour later than the last.
    assert!(
        backup::is_due(Some(at(9, 12)), now),
        "24 hours old to the second"
    );
    assert!(!backup::is_due(Some(at(9, 13)), now), "23 hours old");
    assert!(!backup::is_due(Some(at(10, 12)), now), "taken this minute");
    // A clock that went backwards (the till's date was corrected) must not
    // make a backup due on every tick.
    assert!(!backup::is_due(Some(at(11, 12)), now), "dated ahead");
}

#[test]
fn a_copy_that_cannot_be_finished_leaves_no_name_the_list_would_trust() {
    use std::os::unix::fs::PermissionsExt;

    let (dir, mut conn) = open_temp();
    let backups = dir.path().join("backups");
    // One good copy first, so the failures below have something to leave
    // alone.
    backup::create(&mut conn, &backups, at(1, 3)).unwrap();

    // The name the next copy wants is taken by something that is not a file.
    // A half-written copy under a name the list trusts is the thing to
    // avoid: `list` would report it as the newest, `is_due` would be false
    // for a day, and a restore would be offered a file that is not one.
    let taken = backups.join(backup::file_name(at(2, 3)));
    std::fs::create_dir(&taken).unwrap();
    assert!(backup::create(&mut conn, &backups, at(2, 3)).is_err());
    assert_eq!(
        backup::list(&backups).unwrap().len(),
        1,
        "a copy that never finished was listed"
    );
    std::fs::remove_dir(&taken).unwrap();

    // A folder that cannot be written to at all.
    std::fs::set_permissions(&backups, std::fs::Permissions::from_mode(0o555)).unwrap();
    let refused = backup::create(&mut conn, &backups, at(3, 3));
    std::fs::set_permissions(&backups, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(refused.is_err(), "an unwritable folder answered ok");

    let left: Vec<String> = std::fs::read_dir(&backups)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        left,
        vec![backup::file_name(at(1, 3))],
        "a failed copy left something behind"
    );
}

#[test]
fn a_half_written_copy_is_never_a_name_the_list_reads() {
    let (dir, mut conn) = open_temp();
    let backups = dir.path().join("backups");
    let made = backup::create(&mut conn, &backups, at(4, 3)).unwrap();
    // The name a copy is written under before it is finished. It carries the
    // backup name and one more extension, so the pattern never matches it.
    let staging = backups.join(format!("{}.tmp", backup::file_name(at(5, 3))));
    std::fs::write(&staging, b"half a database").unwrap();

    let listed = backup::list(&backups).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed.first().map(|b| b.name.clone()), Some(made.name));
    assert_eq!(
        backup::taken_at(&format!("{}.tmp", backup::file_name(at(5, 3)))),
        None
    );
}

#[test]
fn a_safety_copy_is_named_for_the_shop_file_and_listed_beside_it() {
    let (dir, mut conn) = open_temp();
    let live = dir.path().join("t.db");
    let backups = dir.path().join("backups");
    backup::create(&mut conn, &backups, at(1, 3)).unwrap();

    let first = backup::safety_name(&live, at(2, 3));
    assert_eq!(first, "t.db.before-restore-20260102-033000-000.sqlite");
    backup::copy_to(&mut conn, &dir.path().join(&first)).unwrap();
    let second = backup::safety_name(&live, at(3, 3));
    backup::copy_to(&mut conn, &dir.path().join(&second)).unwrap();

    let listed = backup::list_safety(&live).unwrap();
    let names: Vec<&str> = listed.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, vec![second.as_str(), first.as_str()], "newest first");
    assert!(listed.iter().all(|b| b.bytes > 0));
    assert_eq!(listed.first().map(|b| b.taken_at), Some(at(3, 3)));

    // The daily copies and the safety copies never see each other: a safety
    // copy is not in the folder the thirty are counted in, so pruning that
    // folder to nothing at all still cannot reach one.
    assert_eq!(backup::prune(&backups, 0).unwrap().len(), 1);
    assert!(backup::list(&backups).unwrap().is_empty());
    assert_eq!(backup::list_safety(&live).unwrap().len(), 2);
    assert!(dir.path().join(&second).is_file());
    assert!(
        dir.path().join(&first).is_file(),
        "the oldest safety copy went"
    );
}

#[test]
fn a_name_beside_the_shop_file_is_read_back_only_when_it_is_a_safety_copy() {
    let live = std::path::Path::new("/shop/t.db");
    assert_eq!(
        backup::safety_taken_at(live, "t.db.before-restore-20260108-093000-250.sqlite"),
        Some(at(8, 9))
    );
    for bad in [
        "t.db",
        "t.db-wal",
        "t.db.restoring.tmp",
        "other.db.before-restore-20260108-093000-250.sqlite",
        "t.db.before-restore-20260108-093000.sqlite",
        "t.db.before-restore-20260108-093000-25.sqlite",
        "t.db.before-restore-20261308-093000-250.sqlite",
        "dzpos-20260108-093000.sqlite",
    ] {
        assert_eq!(
            backup::safety_taken_at(live, bad),
            None,
            "{bad} was accepted"
        );
    }
}
