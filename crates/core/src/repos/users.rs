//! The only place users touch diesel. Every query is scoped by `shop_id`
//! (rule 3); a function that forgets it is the bug this layer exists to make
//! visible.
//!
//! Nothing here decides anything: a hash is written as it is handed over and
//! read back as it is stored, and whether it matches is `services::users`'
//! business.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::user::{Credentials, Role, User, UserRow, UserRowWrite, PIN_UNSET};
use crate::schema::users;

/// The shop's users, the active ones first, then alphabetical. The same order
/// the customer list uses and for the same reason: this list is read by
/// somebody picking who is at the till, and a deactivated row is kept for the
/// paper it signed rather than for that. The id breaks a tie, so the order is
/// stable between two calls.
pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<User>, CoreError> {
    let rows: Vec<UserRow> = users::table
        .filter(users::shop_id.eq(shop_id))
        .order((users::active.desc(), users::name.asc(), users::id.asc()))
        .select(UserRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(User::from).collect())
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<User, CoreError> {
    Ok(User::from(row(conn, shop_id, id)?))
}

/// The stored hashes and the failure counter. Separate from `get` so a
/// credential is read only where it is checked, and never by a caller that
/// only wanted a name.
pub fn credentials(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
) -> Result<Credentials, CoreError> {
    Ok(Credentials::from(&row(conn, shop_id, id)?))
}

/// The same, found by name rather than by id: the password screen asks for a
/// name, while the PIN pad picks a row off the list. `None` is "no such
/// name", and the service answers it exactly as it answers a wrong password.
pub fn credentials_by_name(
    conn: &mut SqliteConnection,
    shop_id: i32,
    name: &str,
) -> Result<Option<Credentials>, CoreError> {
    let found: Option<UserRow> = users::table
        .filter(users::shop_id.eq(shop_id))
        .filter(users::name.eq(name))
        .select(UserRow::as_select())
        .first(conn)
        .optional()?;
    Ok(found.as_ref().map(Credentials::from))
}

fn row(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<UserRow, CoreError> {
    users::table
        .filter(users::shop_id.eq(shop_id))
        .filter(users::id.eq(id))
        .select(UserRow::as_select())
        .first(conn)
        .optional()?
        .ok_or(CoreError::NotFound { entity: "user", id })
}

pub fn insert(conn: &mut SqliteConnection, write: &UserRowWrite) -> Result<User, CoreError> {
    let row: UserRow = diesel::insert_into(users::table)
        .values(write)
        .returning(UserRow::as_returning())
        .get_result(conn)?;
    Ok(User::from(row))
}

/// The name, the role and the active flag: the three an owner edits. The
/// hashes and the failure counter are written by their own functions, so a
/// fiche edit can never blank a credential by leaving a field out.
pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    write: &UserRowWrite,
) -> Result<User, CoreError> {
    let changed = diesel::update(
        users::table
            .filter(users::shop_id.eq(shop_id))
            .filter(users::id.eq(id)),
    )
    .set(write)
    .execute(conn)?;
    if changed == 0 {
        return Err(CoreError::NotFound { entity: "user", id });
    }
    get(conn, shop_id, id)
}

/// Writes the PIN hash and clears the lockout with it: an owner who resets a
/// PIN is the way a locked-out cashier gets back to work, and leaving the
/// counter standing would reset the PIN and refuse it in the same breath.
pub fn set_pin_hash(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    hash: &str,
    now: NaiveDateTime,
) -> Result<User, CoreError> {
    let changed = diesel::update(
        users::table
            .filter(users::shop_id.eq(shop_id))
            .filter(users::id.eq(id)),
    )
    .set((
        users::pin_hash.eq(hash),
        users::pin_failures.eq(0),
        users::locked_until.eq(None::<NaiveDateTime>),
        users::updated_at.eq(now),
    ))
    .execute(conn)?;
    if changed == 0 {
        return Err(CoreError::NotFound { entity: "user", id });
    }
    get(conn, shop_id, id)
}

pub fn set_password_hash(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    hash: &str,
    now: NaiveDateTime,
) -> Result<User, CoreError> {
    let changed = diesel::update(
        users::table
            .filter(users::shop_id.eq(shop_id))
            .filter(users::id.eq(id)),
    )
    .set((
        users::password_hash.eq(Some(hash)),
        users::pin_failures.eq(0),
        users::locked_until.eq(None::<NaiveDateTime>),
        users::updated_at.eq(now),
    ))
    .execute(conn)?;
    if changed == 0 {
        return Err(CoreError::NotFound { entity: "user", id });
    }
    get(conn, shop_id, id)
}

/// The counter after a refused credential, and when the user may try again.
/// `updated_at` is deliberately not touched: it says when somebody changed
/// the fiche, and a stranger at the till guessing PINs has changed nothing.
pub fn record_failure(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    failures: i32,
    locked_until: Option<NaiveDateTime>,
) -> Result<(), CoreError> {
    diesel::update(
        users::table
            .filter(users::shop_id.eq(shop_id))
            .filter(users::id.eq(id)),
    )
    .set((
        users::pin_failures.eq(failures),
        users::locked_until.eq(locked_until),
    ))
    .execute(conn)?;
    Ok(())
}

/// Back to nothing after a credential that matched.
pub fn clear_failures(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<(), CoreError> {
    diesel::update(
        users::table
            .filter(users::shop_id.eq(shop_id))
            .filter(users::id.eq(id))
            // A till signs in all day and almost every sign-in has a clean
            // counter already; without this the row is rewritten each time.
            .filter(
                users::pin_failures
                    .ne(0)
                    .or(users::locked_until.is_not_null()),
            ),
    )
    .set((
        users::pin_failures.eq(0),
        users::locked_until.eq(None::<NaiveDateTime>),
    ))
    .execute(conn)?;
    Ok(())
}

/// How many users of this shop hold `role` and are still active. The last
/// owner rule is counted here rather than listed and filtered above, so the
/// count is the file's answer and not a copy of it that a page of results
/// could truncate.
pub fn count_active_with_role(
    conn: &mut SqliteConnection,
    shop_id: i32,
    role: Role,
) -> Result<i64, CoreError> {
    Ok(users::table
        .filter(users::shop_id.eq(shop_id))
        .filter(users::active.eq(true))
        .filter(users::role.eq(role))
        .count()
        .get_result(conn)?)
}

/// Whether any user of this shop has ever had a PIN or a password set.
/// `services::users::claim_first_owner` is the one caller: it is the shop-wide
/// half of "nobody has ever signed into this shop", the fact that keeps the
/// bootstrap door from reopening once an owner has set a credential the
/// ordinary way.
pub fn any_credential_set(conn: &mut SqliteConnection, shop_id: i32) -> Result<bool, CoreError> {
    let found: Option<i32> = users::table
        .filter(users::shop_id.eq(shop_id))
        .filter(
            users::pin_hash
                .ne(PIN_UNSET)
                .or(users::password_hash.is_not_null()),
        )
        .select(users::id)
        .first(conn)
        .optional()?;
    Ok(found.is_some())
}

/// The shop's one active owner, when there is exactly one. Every other
/// caller in this crate is handed an id by a session or by a caller who
/// already has one; `claim_first_owner` is the one place nothing has signed in
/// yet to say which row it means, so it asks for the row instead of the id.
pub fn sole_active_owner(
    conn: &mut SqliteConnection,
    shop_id: i32,
) -> Result<Option<User>, CoreError> {
    let mut rows: Vec<UserRow> = users::table
        .filter(users::shop_id.eq(shop_id))
        .filter(users::active.eq(true))
        .filter(users::role.eq(Role::Owner))
        .select(UserRow::as_select())
        .load(conn)?;
    if rows.len() != 1 {
        return Ok(None);
    }
    // `pop` rather than indexing: the length check above is the only proof
    // there is exactly one row, and this is the one way to take it without
    // restating that as an index.
    Ok(rows.pop().map(User::from))
}

/// Whether another row of this shop already answers to that name. `exclude`
/// is the row being renamed, which is allowed to keep its own name.
pub fn name_taken(
    conn: &mut SqliteConnection,
    shop_id: i32,
    name: &str,
    exclude: Option<i32>,
) -> Result<bool, CoreError> {
    let mut query = users::table
        .filter(users::shop_id.eq(shop_id))
        .filter(users::name.eq(name))
        .into_boxed();
    if let Some(id) = exclude {
        query = query.filter(users::id.ne(id));
    }
    let found: Option<i32> = query.select(users::id).first(conn).optional()?;
    Ok(found.is_some())
}
