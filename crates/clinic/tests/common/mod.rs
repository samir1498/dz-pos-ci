// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// Every test binary compiles this whole file and calls part of it.
#![allow(dead_code)]

//! What a clinic test needs before it can say anything: a migrated file of
//! its own, and a patient as empty as a file is allowed to be. Nothing here
//! decides anything; a test that needs a phone or a note sets it itself.

use diesel::{RunQueryDsl, SqliteConnection};
use dzpos_clinic::services::patients::NewPatient;

/// The shop and its owner the first migration seeds.
pub const SHOP: i32 = 1;
pub const OWNER: i32 = 1;

/// A database of this test's own, migrated and seeded, in a directory that
/// is deleted when the returned handle drops. Opened through the kernel, the
/// same door the app uses, so every migration in the one shared folder runs.
pub fn open_temp() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = dzpos_kernel::db::open(dir.path().join("t.db")).unwrap();
    (dir, conn)
}

/// A second shop in the same file, for the cases about rule 3.
pub fn second_shop(conn: &mut SqliteConnection) -> i32 {
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième cabinet')")
        .execute(conn)
        .unwrap();
    diesel::sql_query("INSERT INTO users (id, shop_id, name, role) VALUES (2, 2, 'Dr B', 'owner')")
        .execute(conn)
        .unwrap();
    2
}

/// A file with the two names and nothing else.
pub fn named(first: &str, last: &str) -> NewPatient {
    NewPatient {
        first_name: first.to_string(),
        last_name: last.to_string(),
        ..NewPatient::default()
    }
}

/// Reverts every migration above `version`, top first, and returns their
/// ids in the order they went. Each clinic migration test takes the ones
/// stacked on its own off this way, so a new migration on top changes no
/// test below it.
pub fn revert_above(conn: &mut SqliteConnection, version: &str) -> Vec<String> {
    use diesel_migrations::MigrationHarness;
    let mut reverted = Vec::new();
    loop {
        let applied = conn
            .applied_migrations()
            .unwrap()
            .into_iter()
            .map(|v| v.to_string())
            .max()
            .unwrap();
        if applied.as_str() <= version {
            assert_eq!(applied, version, "{version} is not applied");
            return reverted;
        }
        let gone = conn
            .revert_last_migration(dzpos_kernel::db::MIGRATIONS)
            .unwrap();
        reverted.push(gone.to_string());
    }
}

/// The helpers every book test shares: dates counted from the wall clock,
/// since the book refuses the past on it, and the refusal shapes.
pub mod book {
    use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
    use diesel::SqliteConnection;
    use dzpos_clinic::services::appointments::{self, BookedPatient, NewAppointment};
    use dzpos_clinic::services::patients::{self, Patient};
    use dzpos_clinic::services::working_hours::OpenRange;
    use dzpos_kernel::error::CoreError;
    use dzpos_kernel::services::clock;
    use dzpos_kernel::services::permissions::Role;

    use super::{named, OWNER, SHOP};

    pub fn open(conn: &mut SqliteConnection, first: &str, last: &str) -> Patient {
        patients::create(conn, SHOP, OWNER, named(first, last), Role::Owner).unwrap()
    }

    /// `days` after today on the shop's clock.
    pub fn day_ahead(days: i64) -> NaiveDate {
        clock::now().date() + Duration::days(days)
    }

    /// The first `weekday` from tomorrow on, so never today.
    pub fn next(weekday: Weekday) -> NaiveDate {
        let mut day = day_ahead(1);
        while day.weekday() != weekday {
            day = day.succ_opt().unwrap();
        }
        day
    }

    pub fn at(day: NaiveDate, hh: u32, mm: u32) -> NaiveDateTime {
        day.and_time(NaiveTime::from_hms_opt(hh, mm, 0).unwrap())
    }

    /// `HH:MM` to `HH:MM` as a range; `24:00` is 1440.
    pub fn range(from: (i32, i32), to: (i32, i32)) -> OpenRange {
        OpenRange {
            opens_minute: from.0 * 60 + from.1,
            closes_minute: to.0 * 60 + to.1,
        }
    }

    pub fn book(
        conn: &mut SqliteConnection,
        patient: &Patient,
        starts_at: NaiveDateTime,
    ) -> Result<BookedPatient, CoreError> {
        appointments::book(
            conn,
            SHOP,
            OWNER,
            NewAppointment {
                patient_id: patient.id.clone(),
                starts_at,
                ..NewAppointment::default()
            },
        )
    }

    /// The field and message of a conflict; anything else fails the test.
    pub fn conflict<T: std::fmt::Debug>(result: Result<T, CoreError>) -> (String, String) {
        match result {
            Err(CoreError::Conflict { field, message }) => (field, message),
            other => panic!("expected a conflict, got {other:?}"),
        }
    }

    /// The field and message of a validation error.
    pub fn invalid<T: std::fmt::Debug>(result: Result<T, CoreError>) -> (String, String) {
        match result {
            Err(CoreError::Validation { field, message }) => (field, message),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }
}
