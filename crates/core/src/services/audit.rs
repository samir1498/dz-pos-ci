//! The audit log of sensitive actions (features.md §5): a price change, a
//! settings change, a régime change. An ISO 27001 control that costs almost
//! nothing while the writes are being written, and a rewrite afterwards.
//!
//! It records, it never decides. A caller writes the row in the same
//! transaction as the change, so an entry without its change cannot exist.

use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::audit::{AuditEntry, AuditRowWrite};
use crate::repos::audit as repo;

/// A settings or shop block that was replaced.
pub const ACTION_UPDATE: &str = "update";
/// A régime change appended to the dated series.
pub const ACTION_SET_REGIME: &str = "set_regime";

/// What changed, as the log stores it. `before` and `after` are JSON
/// documents the caller writes; the log never guesses a shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub action: &'static str,
    pub entity: &'static str,
    pub entity_id: Option<i32>,
    pub before: Option<String>,
    pub after: Option<String>,
}

pub fn record(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    change: Change,
) -> Result<(), CoreError> {
    repo::insert(
        conn,
        &AuditRowWrite {
            shop_id,
            user_id,
            action: change.action.to_string(),
            entity: change.entity.to_string(),
            entity_id: change.entity_id,
            before: change.before,
            after: change.after,
        },
    )
}

pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<AuditEntry>, CoreError> {
    repo::list(conn, shop_id)
}
