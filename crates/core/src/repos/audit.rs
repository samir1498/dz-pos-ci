//! The only place the audit log touches diesel. Scoped by `shop_id` like
//! every other query (rule 3).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::audit::{AuditEntry, AuditRow, AuditRowWrite};
use crate::schema::audit_log;

pub fn insert(conn: &mut SqliteConnection, write: &AuditRowWrite) -> Result<(), CoreError> {
    diesel::insert_into(audit_log::table)
        .values(write)
        .execute(conn)?;
    Ok(())
}

/// One action's rows, newest first. The order is the opposite of `list`'s on
/// purpose: a caller asking for one action wants the last time it happened,
/// and it reads backwards until the run it is looking at ends.
pub fn by_action(
    conn: &mut SqliteConnection,
    shop_id: i32,
    action: &str,
) -> Result<Vec<AuditEntry>, CoreError> {
    let rows: Vec<AuditRow> = audit_log::table
        .filter(audit_log::shop_id.eq(shop_id))
        .filter(audit_log::action.eq(action))
        .order(audit_log::id.desc())
        .select(AuditRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(AuditEntry::from).collect())
}

/// Oldest first: the log is read as a story, not as a feed.
pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<AuditEntry>, CoreError> {
    let rows: Vec<AuditRow> = audit_log::table
        .filter(audit_log::shop_id.eq(shop_id))
        .order(audit_log::id.asc())
        .select(AuditRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(AuditEntry::from).collect())
}

/// The whole shop's log, newest first: the shape the owner's screen reads
/// (M4 T7), the opposite of `list`'s story order for the same reason
/// `by_action` is. `services::audit` does the filtering and the paging in
/// memory, the way `services::purchases::list` already does over its own
/// `repo::list` for a smaller table than this one will ever be.
///
/// This has no `LIMIT` and no filter but `shop_id`: every call reads the
/// whole table, and the screen calls it again on every filter and every
/// page turn. Twenty-eight call sites across the core write to `audit_log`
/// and nothing prunes it, so this is fine at a shop's first few years of
/// data and stops being fine well before a shop is old enough to need
/// telling so. Whoever notices the screen getting slow should reach for a
/// `LIMIT`/`OFFSET` query pushed into the filter clauses, not a bigger page.
pub fn list_desc(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<AuditEntry>, CoreError> {
    let rows: Vec<AuditRow> = audit_log::table
        .filter(audit_log::shop_id.eq(shop_id))
        .order(audit_log::id.desc())
        .select(AuditRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(AuditEntry::from).collect())
}
