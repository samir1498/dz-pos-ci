//! The only place sessions touch diesel. Every query is scoped by `shop_id`
//! (rule 3).
//!
//! Nothing here decides anything: a hash is written as it is handed over and
//! read back as it is stored, and whether a session is still alive is
//! `services::sessions`' business.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::session::{SessionRow, SessionRowWrite};
use crate::models::sql_types::Role;
use crate::schema::{sessions, users};

pub(crate) fn insert(
    conn: &mut SqliteConnection,
    write: &SessionRowWrite,
) -> Result<SessionRow, CoreError> {
    Ok(diesel::insert_into(sessions::table)
        .values(write)
        .returning(SessionRow::as_returning())
        .get_result(conn)?)
}

/// What a request carrying a token needs in one read: the session row and the
/// two fields of its user that decide whether it still counts.
///
/// Joined rather than read in two queries because a deactivated fiche has to
/// refuse the session it is holding, and two reads with a write between them
/// is the shape of a check that passes on a row somebody has just switched
/// off.
pub(crate) struct Live {
    pub row: SessionRow,
    pub role: Role,
    pub user_active: bool,
    pub name: String,
}

/// The session under this hash, or `None`. The lookup is by hash and never by
/// id: a caller shows a token and nothing else, and a token that matches no
/// row must look exactly like a token whose row has ended.
pub(crate) fn by_token_hash(
    conn: &mut SqliteConnection,
    shop_id: i32,
    token_hash: &str,
) -> Result<Option<Live>, CoreError> {
    let found: Option<(SessionRow, Role, bool, String)> = sessions::table
        .inner_join(users::table)
        .filter(sessions::shop_id.eq(shop_id))
        .filter(sessions::token_hash.eq(token_hash))
        .select((
            SessionRow::as_select(),
            users::role,
            users::active,
            users::name,
        ))
        .first(conn)
        .optional()?;
    Ok(found.map(|(row, role, user_active, name)| Live {
        row,
        role,
        user_active,
        name,
    }))
}

/// The sliding half of the idle rule: this request is the last thing the
/// session did. Written on every call the session carries, which is why it is
/// one UPDATE on a row already found by a unique index.
pub(crate) fn touch(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    now: NaiveDateTime,
) -> Result<(), CoreError> {
    diesel::update(
        sessions::table
            .filter(sessions::shop_id.eq(shop_id))
            .filter(sessions::id.eq(id)),
    )
    .set(sessions::last_seen_at.eq(now))
    .execute(conn)?;
    Ok(())
}

/// Ends one session. Idempotent by the `is_null` filter: signing out twice is
/// a thing a screen does when a click lands during a reload, and the second
/// one must not move the moment the first recorded.
pub(crate) fn end(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    now: NaiveDateTime,
) -> Result<(), CoreError> {
    diesel::update(
        sessions::table
            .filter(sessions::shop_id.eq(shop_id))
            .filter(sessions::id.eq(id))
            .filter(sessions::ended_at.is_null()),
    )
    .set(sessions::ended_at.eq(Some(now)))
    .execute(conn)?;
    Ok(())
}

/// Ends every live session a user is holding. `services::users::deactivate`
/// calls this when a fiche is switched off: the join in `by_token_hash` would
/// refuse the same session on its next request anyway, once `active` reads
/// false, but this is what makes the refusal immediate instead of whatever
/// request the session happens to make next.
///
/// A credential reset is the other caller that needs a session ended, and it
/// cannot use this one: `active` never changes when a PIN or a password is
/// reset, so `by_token_hash` would go on saying yes to the old session
/// forever. `set_pin` and `set_password` call `end_all_for_user_except`
/// below instead, for that reason and to leave the screen doing the reset
/// signed in when it is resetting its own.
pub fn end_all_for_user(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    now: NaiveDateTime,
) -> Result<usize, CoreError> {
    Ok(diesel::update(
        sessions::table
            .filter(sessions::shop_id.eq(shop_id))
            .filter(sessions::user_id.eq(user_id))
            .filter(sessions::ended_at.is_null()),
    )
    .set(sessions::ended_at.eq(Some(now)))
    .execute(conn)?)
}

/// The same, except the one session named. `services::users::set_pin` and
/// `set_password` call this when the person resetting a credential is
/// resetting their own: every other open till under that name is signed out,
/// and the screen finishing the reset is not, because ending it too would be
/// a self-inflicted lockout for doing the thing that just fixed the
/// credential.
pub fn end_all_for_user_except(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    keep_session_id: i32,
    now: NaiveDateTime,
) -> Result<usize, CoreError> {
    Ok(diesel::update(
        sessions::table
            .filter(sessions::shop_id.eq(shop_id))
            .filter(sessions::user_id.eq(user_id))
            .filter(sessions::id.ne(keep_session_id))
            .filter(sessions::ended_at.is_null()),
    )
    .set(sessions::ended_at.eq(Some(now)))
    .execute(conn)?)
}

/// Rows whose idle time ran out long ago and that nothing will read again.
/// `before` is the caller's cut, so the rule about what "long ago" means
/// stays in the service.
pub(crate) fn delete_ended_before(
    conn: &mut SqliteConnection,
    shop_id: i32,
    before: NaiveDateTime,
) -> Result<usize, CoreError> {
    Ok(diesel::delete(
        sessions::table
            .filter(sessions::shop_id.eq(shop_id))
            .filter(sessions::last_seen_at.lt(before)),
    )
    .execute(conn)?)
}
