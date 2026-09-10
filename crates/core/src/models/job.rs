//! The day a once-a-day job last ran, per shop. The stock re-derive runs at
//! app start when this says it has not run today, so the marker has to
//! survive a restart, which a value held in the process does not.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::schema::jobs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    /// A day on the shop's calendar, `YYYY-MM-DD`. `None` means never: a job
    /// named here for the first time has not run, and a date standing in for
    /// that would say it had.
    pub last_run_day: Option<String>,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = jobs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct JobRow {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub last_run_day: Option<String>,
    pub updated_at: NaiveDateTime,
}

impl From<JobRow> for Job {
    fn from(r: JobRow) -> Self {
        Job {
            id: r.id,
            shop_id: r.shop_id,
            name: r.name,
            last_run_day: r.last_run_day,
            updated_at: r.updated_at,
        }
    }
}
