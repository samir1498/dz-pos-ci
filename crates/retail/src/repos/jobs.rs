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
        jobs::updated_at.eq(dzpos_kernel::services::clock::now()),
    ))
    .returning(JobRow::as_returning())
    .get_result(conn)?;
    Ok(Job::from(row))
}

#[cfg(test)]
#[path = "../../tests/unit/repos_jobs.rs"]
mod tests;
