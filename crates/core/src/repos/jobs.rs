// The callers are T2 (suppliers), T3 (purchases), T4 (expenses) and T5 (the
// re-derive job): the tables land here before the services that read them, and
// `repos` is crate-internal on purpose (architecture.md: nothing outside this
// crate touches diesel), so a plain build sees no use of these yet.
#![allow(dead_code)]

//! The only place the job markers touch diesel. One row per
//! `(shop_id, name)`, holding the day that job last ran.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::job::{Job, JobRow};
use crate::schema::jobs;

/// The marker, or `None` when the shop has never named this job. A job
/// nobody has run is not a `NotFound`: the caller's question is whether it
/// ran today, and "there is no row" is a perfectly good no.
pub fn get(
    conn: &mut SqliteConnection,
    shop_id: i32,
    name: &str,
) -> Result<Option<Job>, CoreError> {
    let row: Option<JobRow> = jobs::table
        .filter(jobs::shop_id.eq(shop_id))
        .filter(jobs::name.eq(name))
        .select(JobRow::as_select())
        .first(conn)
        .optional()?;
    Ok(row.map(Job::from))
}

/// Writes the day this job last ran, creating the row on first use the way
/// `counters::take_next` does: a shop the migration never seeded still has a
/// marker to move rather than a missing row to trip over.
pub fn mark_run(
    conn: &mut SqliteConnection,
    shop_id: i32,
    name: &str,
    day: &str,
) -> Result<Job, CoreError> {
    diesel::insert_or_ignore_into(jobs::table)
        .values((jobs::shop_id.eq(shop_id), jobs::name.eq(name)))
        .execute(conn)?;
    let row: JobRow = diesel::update(
        jobs::table
            .filter(jobs::shop_id.eq(shop_id))
            .filter(jobs::name.eq(name)),
    )
    .set((
        jobs::last_run_day.eq(Some(day)),
        jobs::updated_at.eq(crate::services::clock::now()),
    ))
    .returning(JobRow::as_returning())
    .get_result(conn)?;
    Ok(Job::from(row))
}

#[cfg(test)]
mod tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::repos::testdb::{open, SHOP};

    #[test]
    fn a_job_nobody_has_run_has_no_last_day_at_all() {
        // Null is never, and a date standing in for that would say the job
        // had already run today.
        let (_dir, mut conn) = open();
        assert_eq!(get(&mut conn, SHOP, "stock_rederive").unwrap(), None);
    }

    #[test]
    fn the_day_is_written_on_the_first_run_and_moved_on_the_next() {
        let (_dir, mut conn) = open();
        let first = mark_run(&mut conn, SHOP, "stock_rederive", "2026-09-10").unwrap();
        assert_eq!(first.last_run_day.as_deref(), Some("2026-09-10"));
        let again = mark_run(&mut conn, SHOP, "stock_rederive", "2026-09-11").unwrap();
        assert_eq!(again.id, first.id, "a second run made a second row");
        assert_eq!(again.last_run_day.as_deref(), Some("2026-09-11"));
        assert_eq!(
            get(&mut conn, SHOP, "stock_rederive")
                .unwrap()
                .and_then(|j| j.last_run_day)
                .as_deref(),
            Some("2026-09-11")
        );
    }

    #[test]
    fn each_shop_keeps_its_own_marker() {
        let (_dir, mut conn) = open();
        diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
            .execute(&mut conn)
            .unwrap();
        mark_run(&mut conn, SHOP, "stock_rederive", "2026-09-10").unwrap();
        assert_eq!(get(&mut conn, 2, "stock_rederive").unwrap(), None);
        mark_run(&mut conn, 2, "stock_rederive", "2026-09-09").unwrap();
        assert_eq!(
            get(&mut conn, SHOP, "stock_rederive")
                .unwrap()
                .and_then(|j| j.last_run_day)
                .as_deref(),
            Some("2026-09-10")
        );
    }
}
