//! Absence blocks (item 2 of the book tools, C5b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`):
//! a period the doctor is away, from a start to an end with an optional
//! short label. The book refuses a booking or a move that shares a minute
//! with one; touching end to start is not sharing one.
//!
//! Making a block answers with every live appointment it lands on. The block
//! moves or cancels none of them: the desk does, one by one, through the
//! book's own `move_to` (checked against every rule, this block included)
//! or `cancel`. A block is removed outright, its audit row the trace.
//! Blocks are the desk's, so they take the patient file's write permission
//! at the gate, as the book does.

use chrono::{NaiveDateTime, Timelike};
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::{audit, clock};

use crate::audit_actions::{ACTION_ABSENCE_BLOCK_CREATE, ACTION_ABSENCE_BLOCK_REMOVE};
use crate::repos::absence_blocks as repo;
use crate::services::appointments;

pub use crate::models::absence_block::{AbsenceBlock, BlockMade};

/// The longest label the table holds.
pub const MAX_LABEL_CHARS: usize = 60;

/// What the desk asks for.
#[derive(Debug, Clone, Default)]
pub struct NewBlock {
    pub starts_at: NaiveDateTime,
    pub ends_at: NaiveDateTime,
    pub label: Option<String>,
}

/// Makes a block and answers with the live appointments inside it. A block
/// in the past is taken: it refuses nothing any more, and the desk may be
/// writing down a morning it was away.
pub fn create(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    new: NewBlock,
) -> Result<BlockMade, CoreError> {
    for (field, at) in [("starts_at", new.starts_at), ("ends_at", new.ends_at)] {
        if at.second() != 0 || at.nanosecond() != 0 {
            return Err(CoreError::validation(
                field,
                "a block starts and ends on a whole minute",
            ));
        }
    }
    if new.ends_at <= new.starts_at {
        return Err(CoreError::validation(
            "ends_at",
            "a block ends after it starts",
        ));
    }
    let label = label(new.label.as_deref())?;
    let now = clock::now();
    conn.transaction(|conn| {
        let block = repo::insert(
            conn,
            &AbsenceBlock {
                id: uuid::Uuid::now_v7().to_string(),
                shop_id,
                starts_at: new.starts_at,
                ends_at: new.ends_at,
                label,
                created_at: now,
            },
        )?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: ACTION_ABSENCE_BLOCK_CREATE,
                entity: "absence_block",
                entity_id: None,
                before: None,
                after: Some(as_json(&block)),
            },
        )?;
        let hits = appointments::overlapping(conn, shop_id, block.starts_at, block.ends_at)?;
        Ok(BlockMade { block, hits })
    })
}

/// Removes a block of this shop and answers with it as it was. Another
/// shop's is not found.
pub fn remove(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: &str,
) -> Result<AbsenceBlock, CoreError> {
    conn.transaction(|conn| {
        let block = repo::get(conn, shop_id, id)?;
        repo::delete(conn, shop_id, id)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: ACTION_ABSENCE_BLOCK_REMOVE,
                entity: "absence_block",
                entity_id: None,
                before: Some(as_json(&block)),
                after: None,
            },
        )?;
        Ok(block)
    })
}

/// The blocks not yet over on the shop's clock, in time order.
pub fn upcoming(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<AbsenceBlock>, CoreError> {
    repo::ending_after(conn, shop_id, clock::now())
}

/// The blocks sharing a minute with `[from, until)`, in time order.
pub fn overlapping(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Vec<AbsenceBlock>, CoreError> {
    repo::overlapping(conn, shop_id, from, until)
}

/// Trimmed, `None` when blank, refused past the table's 60 characters.
fn label(text: Option<&str>) -> Result<Option<String>, CoreError> {
    let Some(text) = text.map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    if text.chars().count() > MAX_LABEL_CHARS {
        return Err(CoreError::validation(
            "label",
            "a block's label is 60 characters at most",
        ));
    }
    Ok(Some(text.to_string()))
}

fn as_json(b: &AbsenceBlock) -> String {
    serde_json::json!({
        "id": b.id,
        "starts_at": b.starts_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        "ends_at": b.ends_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        "label": b.label,
    })
    .to_string()
}
