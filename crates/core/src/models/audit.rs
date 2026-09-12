//! One sensitive action, with what the row held before and after
//! (features.md §5). `before` and `after` are JSON documents; the column
//! names are quoted in the migration because BEFORE is a keyword.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::schema::audit_log;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    pub id: i32,
    pub shop_id: i32,
    pub user_id: i32,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<i32>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = audit_log)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct AuditRow {
    pub id: i32,
    pub shop_id: i32,
    pub user_id: i32,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<i32>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = audit_log)]
pub(crate) struct AuditRowWrite {
    pub shop_id: i32,
    pub user_id: i32,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<i32>,
    pub before: Option<String>,
    pub after: Option<String>,
    /// On the shop's calendar, from `services::clock`, never left to the
    /// column's own default. SQLite's `CURRENT_TIMESTAMP` is UTC and every
    /// other date in this file is UTC+1, which put a row written at 00:30 in
    /// Algiers on the day before and out of the day an owner filtered for.
    /// Not an `Option` on purpose: the supplier ledger guards the same rule
    /// at its repo with `CoreError::Unstamped` because a caller there chooses
    /// the moment, while every audit row is written now, so the type can
    /// carry the rule instead of a runtime check.
    pub created_at: NaiveDateTime,
}

impl From<AuditRow> for AuditEntry {
    fn from(r: AuditRow) -> Self {
        AuditEntry {
            id: r.id,
            shop_id: r.shop_id,
            user_id: r.user_id,
            action: r.action,
            entity: r.entity,
            entity_id: r.entity_id,
            before: r.before,
            after: r.after,
            created_at: r.created_at,
        }
    }
}
