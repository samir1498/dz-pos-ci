//! Business rules. Every caller (HTTP, Tauri, tests) enters here.

use crate::error::CoreError;

/// Longest value any one printed field keeps. An RC is 20-odd characters and
/// an address a few lines; anything past this is a paste gone wrong, and the
/// ticket and facture templates would wrap it into nonsense.
///
/// The seller block and the buyer block print side by side on the same paper,
/// so they hold each other to the same bound from one place.
pub(crate) const MAX_FIELD_CHARS: usize = 200;

/// Trimmed, and blank becomes `None`: the column is cleared, never left
/// holding a space.
pub(crate) fn optional_field(
    field: &str,
    value: Option<&str>,
) -> Result<Option<String>, CoreError> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    bounded_field(field, value)?;
    Ok(Some(value.to_string()))
}

/// The role the permission table is asked about, read from the user the work
/// is being written under.
///
/// Read rather than taken as an argument so the role that was checked and the
/// user the document, the row and the audit entry name are the same person: a
/// role handed in beside a `user_id` is a second statement of who is acting,
/// and two statements can disagree. `services::sales` wrote this first for
/// the discount and typed-price checks; `services::shifts` is the second
/// caller, for the drawer that is not the closer's own, so it lives here
/// rather than being copied.
///
/// Here and not in either of them because one service calling the other for
/// it would be an import neither needs otherwise, and `services::users` is
/// already this module's child, so no `services::` edge is drawn by asking it.
pub(crate) fn role_of(
    conn: &mut diesel::sqlite::SqliteConnection,
    shop_id: i32,
    user_id: i32,
) -> Result<crate::models::sql_types::Role, CoreError> {
    Ok(users::get(conn, shop_id, user_id)?.role)
}

/// Every user in the shop by id and name, for a screen that prints a name
/// beside a row it read somewhere else. `services::audit` is the caller: the
/// log stores a `user_id` per entry and the page shows who that was, in the
/// rows and in the filter's dropdown.
///
/// Here for `role_of`'s reason and one more. `services::users` writes an audit
/// row on nearly everything it does, so `audit` asking `users` for a name
/// directly is the pair importing each other, which
/// `no_service_imports_a_sibling_that_imports_it_back` refuses and which made
/// both files unreadable apart. The name is not the audit log's to keep — a
/// renamed user shows their new name on every past row, which is the point of
/// storing the id — so the lookup belongs to neither side and sits above both.
pub(crate) fn user_names(
    conn: &mut diesel::sqlite::SqliteConnection,
    shop_id: i32,
) -> Result<Vec<(i32, String)>, CoreError> {
    Ok(users::list(conn, shop_id)?
        .into_iter()
        .map(|user| (user.id, user.name))
        .collect())
}

/// Every live session a user holds, ended, except one the caller names. A
/// credential that changed and a fiche switched off both mean the tokens
/// handed out before it are no longer the person they were issued to, and
/// `services::users` is where both of those are decided.
///
/// Here rather than in `users` because `services::sessions` signs a user in
/// by asking `services::users` to believe a PIN or a password, so a call the
/// other way is the two importing each other. The direction that stays is the
/// one the sign-in rule needs; this, the only call `users` had into
/// `sessions`, is lifted above both instead. `keep_session_id` is the
/// caller's own session when a person resets their own credential, and `None`
/// ends every one of them.
pub(crate) fn end_sessions_of(
    conn: &mut diesel::sqlite::SqliteConnection,
    shop_id: i32,
    user_id: i32,
    keep_session_id: Option<i32>,
    now: chrono::NaiveDateTime,
) -> Result<(), CoreError> {
    match keep_session_id {
        Some(keep) => sessions::end_all_for_user_except(conn, shop_id, user_id, keep, now)?,
        None => sessions::end_all_for_user(conn, shop_id, user_id, now)?,
    };
    Ok(())
}

pub(crate) fn bounded_field(field: &str, value: &str) -> Result<(), CoreError> {
    if value.chars().count() > MAX_FIELD_CHARS {
        return Err(CoreError::validation(
            field,
            "longer than a ticket or a facture can print",
        ));
    }
    Ok(())
}

pub mod audit;
pub mod avoir;
pub mod avoir_remaining;
pub mod avoir_slice;
pub mod backup;
pub mod cancellation;
pub mod cash;
pub mod cash_refunds;
pub mod categories;
pub mod clock;
pub mod customers;
pub mod dashboard;
pub mod debt;
pub mod documents;
pub mod expenses;
pub mod export;
pub mod import;
pub mod pairing;
pub mod permissions;
pub mod preferences;
pub mod pricing;
pub mod products;
pub mod proforma;
pub mod purchases;
pub mod sales;
pub mod seed;
pub mod sessions;
pub mod settings;
pub mod shifts;
pub mod shops;
pub mod stock;
pub mod supplier_debt;
pub mod suppliers;
pub mod support_bundle;
pub mod users;
