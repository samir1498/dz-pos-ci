//! Who is signed in at the till (features.md §5). The row every document,
//! ledger movement and audit entry has pointed at since the first sale.
//!
//! No hash leaves this module. `User` is what a caller sees and it carries
//! `pin_hash` nowhere: a service that needs to check a credential reads the
//! row through `repos::users::credentials`, and anything above the repo layer
//! gets the two booleans that say whether a credential is set at all. A hash
//! on a struct is a hash in a log line, in a DTO and eventually on a wire.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::schema::users;

pub use super::sql_types::Role;

/// A user as the rest of the app sees them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub role: Role,
    /// Whether a PIN has been set. The sentinel the first migration writes
    /// (`'!unset'`) reads as `false` here, which is the honest answer: the row
    /// exists and nobody can sign in on it yet.
    pub has_pin: bool,
    /// Whether a password has been set. A till user never has one.
    pub has_password: bool,
    /// Deactivated users are kept: their documents and ledger rows name them
    /// for good. `false` refuses every sign-in and nothing else.
    pub active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// What the first migration writes into `pin_hash`, and what a row still
/// reads when nobody has set a PIN on it. It is not a PHC string, so no
/// verifier parses it and no credential matches it: the row fails closed.
pub const PIN_UNSET: &str = "!unset";

/// The fields `create` takes. The credentials are set by their own calls, so
/// a new user exists before they can sign in and the rules that refuse a bad
/// PIN live in one place rather than two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUser {
    pub name: String,
    pub role: Role,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct UserRow {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub role: Role,
    pub pin_hash: String,
    pub created_at: NaiveDateTime,
    pub password_hash: Option<String>,
    pub active: bool,
    pub pin_failures: i32,
    pub locked_until: Option<NaiveDateTime>,
    pub updated_at: NaiveDateTime,
}

/// The row a sign-in reads: the stored hashes and the failure counter beside
/// them, because the check and the lockout are one decision taken on one row.
/// Crate-internal, like the row it comes from.
#[derive(Debug, Clone)]
pub(crate) struct Credentials {
    pub id: i32,
    pub pin_hash: String,
    pub password_hash: Option<String>,
    pub active: bool,
    pub pin_failures: i32,
    pub locked_until: Option<NaiveDateTime>,
}

impl Credentials {
    /// The stored PIN string, or `None` when no PIN has been set: the
    /// sentinel is not a credential and is never handed to a verifier as one.
    pub(crate) fn pin_hash_of(&self) -> Option<&str> {
        (self.pin_hash != PIN_UNSET).then_some(self.pin_hash.as_str())
    }

    pub(crate) fn password_hash_of(&self) -> Option<&str> {
        self.password_hash.as_deref()
    }
}

impl From<&UserRow> for Credentials {
    fn from(r: &UserRow) -> Self {
        Credentials {
            id: r.id,
            pin_hash: r.pin_hash.clone(),
            password_hash: r.password_hash.clone(),
            active: r.active,
            pin_failures: r.pin_failures,
            locked_until: r.locked_until,
        }
    }
}

/// A user's name and role, the two fields an owner edits. `updated_at` is
/// stamped by the service on every write the way a customer's is: SQLite's
/// DEFAULT only fires on the insert, and this column has no useful default
/// at all (the migration says why).
#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = users)]
pub(crate) struct UserRowWrite {
    pub shop_id: i32,
    pub name: String,
    pub role: Role,
    pub active: bool,
    pub updated_at: NaiveDateTime,
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        User {
            id: r.id,
            shop_id: r.shop_id,
            name: r.name,
            role: r.role,
            has_pin: r.pin_hash != PIN_UNSET,
            has_password: r.password_hash.is_some(),
            active: r.active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
