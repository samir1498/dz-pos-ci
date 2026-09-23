//! The next free slot (item 4 of the book tools, C5b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! from a day, the earliest start on the slot grid that a booking of the
//! given visit type (or of the slot length) would be taken at: inside the
//! working hours, outside every absence block, running into no live
//! appointment, and not in the past on the shop's clock.
//!
//! "See again in 15 days" is this search from today with an offset of 15.
//! The search runs over `SEARCH_DAYS` days from its first one and answers
//! none past that, rather than walking the calendar for a week that has no
//! room. A first day already past starts the search today: the days before
//! it could only hold the past.
//!
//! The rules are the booking's own (`appointments::check_slot`), read here
//! against the book loaded once for the whole window instead of one query
//! per start. A booking made from the answer still runs every check, so a
//! slot taken in between is refused there, not here.

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::clock;

use crate::services::working_hours::MINUTES_IN_DAY;
use crate::services::{absence_blocks, appointments, slot_length, visit_types, working_hours};

/// How many days the search reads, its first included.
pub const SEARCH_DAYS: i64 = 60;

/// The furthest ahead an offset may start the search: a year.
pub const MAX_OFFSET_DAYS: u32 = 366;

/// What the desk asks: from which day, how many days after it, for which
/// visit type or how many minutes.
#[derive(Debug, Clone, Default)]
pub struct FreeSlotQuery {
    /// The day to count from; today on the shop's clock when `None`.
    pub from: Option<NaiveDate>,
    /// Days after `from` the search starts on: 15 for "see again in 15 days".
    pub offset_days: u32,
    /// A visit type of this shop whose length the slot must hold; the slot
    /// length when `None`.
    pub visit_type_id: Option<String>,
    /// A length in minutes the slot must hold, instead of a visit type: a
    /// booking being moved asks for its own `slot_minutes`, which no type
    /// may still carry. 1 to 240, the range a stored booking runs; never
    /// together with `visit_type_id`.
    pub minutes: Option<i32>,
}

/// The answer: the days read, first and last, the length searched for, and
/// the earliest start, `None` when the window has no room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreeSlot {
    pub first_day: NaiveDate,
    pub last_day: NaiveDate,
    pub minutes: i32,
    pub starts_at: Option<NaiveDateTime>,
}

pub fn next_free(
    conn: &mut SqliteConnection,
    shop_id: i32,
    query: FreeSlotQuery,
) -> Result<FreeSlot, CoreError> {
    if query.offset_days > MAX_OFFSET_DAYS {
        return Err(CoreError::validation(
            "offset_days",
            "the search starts at most a year ahead",
        ));
    }
    let now = clock::now();
    let today = now.date();
    let asked = query
        .from
        .unwrap_or(today)
        .checked_add_signed(Duration::days(i64::from(query.offset_days)))
        .ok_or_else(out_of_calendar)?;
    let first_day = asked.max(today);
    let last_day = first_day
        .checked_add_signed(Duration::days(SEARCH_DAYS - 1))
        .ok_or_else(out_of_calendar)?;

    let grid = i32::try_from(slot_length::slot_minutes(conn, shop_id)?).map_err(|_| {
        CoreError::validation(slot_length::SLOT_MINUTES, "the slot length is out of range")
    })?;
    let minutes = match (query.visit_type_id.as_deref(), query.minutes) {
        (Some(_), Some(_)) => {
            return Err(CoreError::validation(
                "minutes",
                "ask by visit type or by minutes, not both",
            ))
        }
        (Some(id), None) => visit_types::get(conn, shop_id, id)?.minutes,
        (None, Some(asked)) if (1..=visit_types::MAX_VISIT_MINUTES).contains(&asked) => asked,
        (None, Some(_)) => {
            return Err(CoreError::validation(
                "minutes",
                "a booking runs 1 to 240 minutes",
            ))
        }
        (None, None) => grid,
    };
    let week = working_hours::week(conn, shop_id)?;
    let from = first_day.and_time(NaiveTime::MIN);
    let until = last_day
        .succ_opt()
        .ok_or_else(out_of_calendar)?
        .and_time(NaiveTime::MIN);
    // A start on the last day may run past its midnight.
    let reach = until
        .checked_add_signed(Duration::minutes(i64::from(minutes)))
        .ok_or_else(out_of_calendar)?;
    let blocks = absence_blocks::overlapping(conn, shop_id, from, reach)?;
    let held: Vec<(NaiveDateTime, NaiveDateTime)> =
        appointments::overlapping(conn, shop_id, from, reach)?
            .into_iter()
            .map(|b| {
                let start = b.appointment.starts_at;
                (start, b.appointment.ends_at().unwrap_or(NaiveDateTime::MAX))
            })
            .collect();

    let step = usize::try_from(grid).map_err(|_| out_of_calendar())?;
    let mut day = first_day;
    while day <= last_day {
        for minute in (0..MINUTES_IN_DAY).step_by(step) {
            let Some(start) = day
                .and_time(NaiveTime::MIN)
                .checked_add_signed(Duration::minutes(i64::from(minute)))
            else {
                continue;
            };
            if start < now {
                continue;
            }
            if week
                .as_ref()
                .is_some_and(|w| !working_hours::holds(w, start, minutes))
            {
                continue;
            }
            let Some(end) = start.checked_add_signed(Duration::minutes(i64::from(minutes))) else {
                continue;
            };
            let blocked = blocks
                .iter()
                .any(|b| b.starts_at < end && start < b.ends_at);
            let taken = held.iter().any(|(s, e)| *s < end && start < *e);
            if !blocked && !taken {
                return Ok(FreeSlot {
                    first_day,
                    last_day,
                    minutes,
                    starts_at: Some(start),
                });
            }
        }
        day = day.succ_opt().ok_or_else(out_of_calendar)?;
    }
    Ok(FreeSlot {
        first_day,
        last_day,
        minutes,
        starts_at: None,
    })
}

fn out_of_calendar() -> CoreError {
    CoreError::validation("from", "that date is outside the calendar")
}
