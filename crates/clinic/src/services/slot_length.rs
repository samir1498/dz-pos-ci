//! The appointment book's slot length, in whole minutes: one number per
//! cabinet, the grid every start sits on and the length a new appointment
//! takes (C5 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`).
//!
//! Kept in the kernel's dated settings, the way
//! `dzpos_retail::services::discount_threshold` keeps the shop's discount
//! rule, through the same four pass-throughs: a change is a new row dated
//! the moment it was made, and the rows before it stay. Nothing reads that
//! history back today (each appointment carries its own `slot_minutes`), so
//! `dzpos_kernel::services::preferences`' own test ("does anybody read its
//! history?") would sort it there instead; the dated series is the one the
//! plan named, and it costs one row per change.

use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::{audit, clock, settings};

use crate::audit_actions::ACTION_SLOT_MINUTES_SET;

/// The settings key.
pub const SLOT_MINUTES: &str = "slot_minutes";

/// What a cabinet that has never set one books in: a quarter of an hour, the
/// usual consultation slot of a general practice.
pub const DEFAULT_SLOT_MINUTES: u32 = 15;

/// The shortest and longest slot a cabinet may set, and the step between:
/// 5, 10, ... 120. A step of five keeps every grid on the minutes a desk
/// says out loud.
pub const MIN_SLOT_MINUTES: u32 = 5;
pub const MAX_SLOT_MINUTES: u32 = 120;
pub const SLOT_STEP_MINUTES: u32 = 5;

/// The slot length in force now. `DEFAULT_SLOT_MINUTES` when the cabinet has
/// never set one.
pub fn slot_minutes(conn: &mut SqliteConnection, shop_id: i32) -> Result<u32, CoreError> {
    match settings::value_as_of(conn, shop_id, SLOT_MINUTES, clock::now())? {
        Some(value) => parse(&value),
        None => Ok(DEFAULT_SLOT_MINUTES),
    }
}

/// Sets the slot length from now on. Appointments already in the book keep
/// the length they were booked with. Setting the length in force is not an
/// error and writes nothing: the screen sends what it shows.
pub fn set_slot_minutes(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    minutes: u32,
) -> Result<u32, CoreError> {
    allowed(minutes)?;
    let now = clock::now();
    conn.transaction(|conn| {
        let before = settings::value_as_of(conn, shop_id, SLOT_MINUTES, now)?;
        let current = match before.as_deref() {
            Some(value) => parse(value)?,
            None => DEFAULT_SLOT_MINUTES,
        };
        if current == minutes {
            return Ok(minutes);
        }
        settings::append(conn, shop_id, SLOT_MINUTES, &minutes.to_string(), now)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: ACTION_SLOT_MINUTES_SET,
                entity: SLOT_MINUTES,
                entity_id: Some(shop_id),
                before: Some(serde_json::json!({ "slot_minutes": current }).to_string()),
                after: Some(
                    serde_json::json!({
                        "slot_minutes": minutes,
                        "valid_from": now.to_string(),
                    })
                    .to_string(),
                ),
            },
        )?;
        Ok(minutes)
    })
}

fn allowed(minutes: u32) -> Result<(), CoreError> {
    if (MIN_SLOT_MINUTES..=MAX_SLOT_MINUTES).contains(&minutes)
        && minutes.is_multiple_of(SLOT_STEP_MINUTES)
    {
        Ok(())
    } else {
        Err(CoreError::validation(
            SLOT_MINUTES,
            "a slot is 5 to 120 minutes long, in steps of 5",
        ))
    }
}

/// A stored value, checked against the same rule a write is: a row that
/// reached the table another way is refused rather than booked on.
fn parse(value: &str) -> Result<u32, CoreError> {
    let minutes: u32 = value.parse().map_err(|_| {
        CoreError::validation(SLOT_MINUTES, &format!("{value} is not a slot length"))
    })?;
    allowed(minutes)?;
    Ok(minutes)
}
