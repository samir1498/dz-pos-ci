//! The cabinet's working hours (item 1 of the book tools, C5b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! for each weekday, Sunday first, zero or more open ranges. A booking or a
//! move has to fit whole inside one range of its start's day, so a visit
//! that runs into a lunch break, past the close or past midnight is
//! refused.
//!
//! A cabinet that has never set its hours is refused nothing on their
//! account: the C5 book, which had no hours, keeps behaving as it did. A
//! week is written whole, one `working_hours.set` audit row for it, with the
//! same `EditSettings` as the slot length. A week with no open range at all
//! is refused rather than read as "never set": the desk closes a day it is
//! away with an absence block, not by closing the week.

use chrono::{Datelike, NaiveDateTime, Timelike};
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::{audit, clock};

use crate::audit_actions::ACTION_WORKING_HOURS_SET;
use crate::models::working_hours::WorkingHoursInsert;
use crate::repos::working_hours as repo;

pub use crate::models::working_hours::{OpenRange, WorkingWeek, DAYS_IN_WEEK};

/// The field every refusal of a week names.
pub const DAYS: &str = "days";

/// Minutes in a day: the latest a range may close.
pub const MINUTES_IN_DAY: i32 = 1440;

/// The cabinet's week, or `None` when it has never set one.
pub fn week(conn: &mut SqliteConnection, shop_id: i32) -> Result<Option<WorkingWeek>, CoreError> {
    let rows = repo::all(conn, shop_id)?;
    if rows.is_empty() {
        return Ok(None);
    }
    let mut week = WorkingWeek::default();
    for row in rows {
        let day = usize::try_from(row.weekday)
            .ok()
            .and_then(|d| week.days.get_mut(d))
            .ok_or_else(|| CoreError::validation(DAYS, "a stored weekday is out of range"))?;
        day.push(OpenRange {
            opens_minute: row.opens_minute,
            closes_minute: row.closes_minute,
        });
    }
    Ok(Some(week))
}

/// Replaces the cabinet's week with `days`, Sunday first. Writing the week
/// in force writes nothing, the way the slot length does.
pub fn set_week(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    days: Vec<Vec<OpenRange>>,
) -> Result<WorkingWeek, CoreError> {
    let wanted = checked(days)?;
    let now = clock::now();
    conn.transaction(|conn| {
        let before = week(conn, shop_id)?;
        if before.as_ref() == Some(&wanted) {
            return Ok(wanted);
        }
        let mut rows = Vec::new();
        for (weekday, ranges) in (0_i32..).zip(wanted.days.iter()) {
            for range in ranges {
                rows.push(WorkingHoursInsert {
                    id: uuid::Uuid::now_v7().to_string(),
                    shop_id,
                    weekday,
                    opens_minute: range.opens_minute,
                    closes_minute: range.closes_minute,
                    created_at: now,
                });
            }
        }
        repo::replace(conn, shop_id, &rows)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: ACTION_WORKING_HOURS_SET,
                entity: "working_hours",
                entity_id: Some(shop_id),
                before: before.as_ref().map(as_json),
                after: Some(as_json(&wanted)),
            },
        )?;
        Ok(wanted)
    })
}

/// Whether a visit of `minutes` starting at `starts_at` fits whole inside
/// one open range of its day. A visit that would run past midnight never
/// fits: no range closes after 1440.
pub fn holds(week: &WorkingWeek, starts_at: NaiveDateTime, minutes: i32) -> bool {
    let Some(day) = usize::try_from(starts_at.weekday().num_days_from_sunday())
        .ok()
        .and_then(|d| week.days.get(d))
    else {
        return false;
    };
    let Ok(start) = i32::try_from(starts_at.hour() * 60 + starts_at.minute()) else {
        return false;
    };
    let Some(end) = start.checked_add(minutes) else {
        return false;
    };
    day.iter()
        .any(|r| r.opens_minute <= start && end <= r.closes_minute)
}

/// Seven days, each range inside the day and opening before it closes, a
/// day's ranges in time order with a gap between each, and at least one
/// range in the week.
fn checked(days: Vec<Vec<OpenRange>>) -> Result<WorkingWeek, CoreError> {
    let days: [Vec<OpenRange>; DAYS_IN_WEEK] = days
        .try_into()
        .map_err(|_| CoreError::validation(DAYS, "a week is seven days, Sunday first"))?;
    for ranges in &days {
        for range in ranges {
            if range.opens_minute < 0
                || range.closes_minute > MINUTES_IN_DAY
                || range.opens_minute >= range.closes_minute
            {
                return Err(CoreError::validation(
                    DAYS,
                    "a range opens before it closes, between 00:00 and 24:00",
                ));
            }
        }
        for pair in ranges.windows(2) {
            if let [earlier, later] = pair {
                if earlier.closes_minute >= later.opens_minute {
                    return Err(CoreError::validation(
                        DAYS,
                        "a day's ranges are in time order, with a break between each; \
                         two that touch are one range",
                    ));
                }
            }
        }
    }
    if days.iter().all(Vec::is_empty) {
        return Err(CoreError::validation(
            DAYS,
            "a week with no open hour closes the book; close a day with an absence block instead",
        ));
    }
    Ok(WorkingWeek { days })
}

/// The week as the audit row stores it: seven lists of `[opens, closes]`
/// minute pairs, Sunday first.
fn as_json(week: &WorkingWeek) -> String {
    let days: Vec<Vec<[i32; 2]>> = week
        .days
        .iter()
        .map(|d| {
            d.iter()
                .map(|r| [r.opens_minute, r.closes_minute])
                .collect()
        })
        .collect();
    serde_json::json!({ "days": days }).to_string()
}
