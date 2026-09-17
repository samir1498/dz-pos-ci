//! The audit log as it is read back.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// One row of the audit log, for the owner's screen (M4 T7, features.md
/// §5). `before` and `after` are the JSON documents the writing service
/// gave `services::audit::record`, sent through unparsed: the screen reads
/// them as the shape the service that wrote the row chose, and this layer
/// never guesses one.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "AuditEntryDto.ts")]
pub struct AuditEntryDto {
    pub id: i32,
    pub user_id: i32,
    /// Carried on the row because there is no `/users` list route yet for
    /// the screen to join it itself (`services::audit::EntryWithUser`).
    pub user_name: String,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<i32>,
    pub before: Option<String>,
    pub after: Option<String>,
    /// `YYYY-MM-DD HH:MM:SS` on the shop's calendar, the same clock every
    /// other date this app shows is read on.
    pub created_at: String,
}

impl From<dzpos_core::services::audit::EntryWithUser> for AuditEntryDto {
    fn from(e: dzpos_core::services::audit::EntryWithUser) -> Self {
        AuditEntryDto {
            id: e.entry.id,
            user_id: e.entry.user_id,
            user_name: e.user_name,
            action: e.entry.action,
            entity: e.entry.entity,
            entity_id: e.entry.entity_id,
            before: e.entry.before,
            after: e.entry.after,
            created_at: e.entry.created_at.format(DATE_TIME_FORMAT).to_string(),
        }
    }
}

/// A user the audit log's `user_id` filter offers, whether or not they have
/// written a row.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "AuditUserDto.ts")]
pub struct AuditUserDto {
    pub id: i32,
    pub name: String,
}

/// One page of the log, filtered, and what the screen's two dropdowns may
/// narrow it by. The dropdowns' own options travel with every page rather
/// than being their own route, because they are cheap beside the rows
/// (`services::audit::read` reads the shop's users and its distinct actions
/// once, not once per row) and a screen that filtered down to nothing would
/// otherwise have no way to offer the other choices back.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "AuditLogDto.ts")]
pub struct AuditLogDto {
    pub rows: Vec<AuditEntryDto>,
    pub page: i64,
    /// Whether `page + 1` would answer more rows under the same filter.
    pub has_more: bool,
    pub users: Vec<AuditUserDto>,
    pub actions: Vec<String>,
}

impl AuditLogDto {
    pub fn from_core(
        page: dzpos_core::services::audit::Page,
        facets: dzpos_core::services::audit::Facets,
    ) -> Self {
        AuditLogDto {
            rows: page.rows.into_iter().map(AuditEntryDto::from).collect(),
            page: page.page,
            has_more: page.has_more,
            users: facets
                .users
                .into_iter()
                .map(|(id, name)| AuditUserDto { id, name })
                .collect(),
            actions: facets.actions,
        }
    }
}

#[cfg(test)]
mod audit_dto_tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use dzpos_core::models::audit::AuditEntry;
    use dzpos_core::services::audit::EntryWithUser;

    /// The moment travels through this conversion untouched. It used to be
    /// shifted here, because the column held UTC and the screen wanted the
    /// shop's calendar; the row is stamped from the shop clock now
    /// (`2026-09-12-000013_audit_log_shop_clock`) and a shift put back by
    /// hand would print every row an hour late. The seconds are also the
    /// last of it: `clock::now()` carries nanoseconds and the column keeps
    /// them, so the format string is what stands between the owner's screen
    /// and a date ending in nine digits.
    #[test]
    fn the_moment_is_printed_as_it_stands_to_the_second() {
        let entry = AuditEntry {
            id: 1,
            shop_id: 1,
            user_id: 1,
            action: "product.update".to_string(),
            entity: "product".to_string(),
            entity_id: Some(7),
            before: None,
            after: None,
            created_at: chrono::NaiveDate::from_ymd_opt(2026, 9, 12)
                .unwrap()
                .and_hms_nano_opt(0, 30, 0, 123_456_789)
                .unwrap(),
        };
        let dto = AuditEntryDto::from(EntryWithUser {
            entry,
            user_name: "Propriétaire".to_string(),
        });
        assert_eq!(dto.created_at, "2026-09-12 00:30:00");
    }
}
