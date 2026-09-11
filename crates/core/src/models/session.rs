//! A sign-in that is still standing (M4 T2). The launch token says a caller
//! is on this machine; a row here says which person is at the keyboard.
//!
//! No token leaves this module in the clear except the one time it is minted,
//! and the type that carries it that once has no `Debug` output, the way
//! `LaunchToken` in the API crate has none: a credential printed in a log
//! line is a credential in a log file, and the brief for this task says a
//! session token never appears in a log or an error message.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::models::sql_types::Role;
use crate::schema::sessions;

/// Who is acting, as every route above this layer wants it: the id that goes
/// on a document and the role the permission table answers for. Nothing about
/// the credential is on it.
///
/// `session_id` is the row this actor is acting through, not the user's only
/// session: a person may hold several at once (a desktop till and a phone),
/// and this is the one the current request carried. `services::users::set_pin`
/// and `set_password` are the callers that need it, to leave the screen doing
/// a self-reset signed in while every other open till for that name is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor {
    pub user_id: i32,
    pub shop_id: i32,
    pub role: Role,
    pub session_id: i32,
}

/// A freshly minted session: the secret, handed to the caller exactly once,
/// and the actor it stands for.
///
/// `Debug` is derived and safe because the only field that could leak carries
/// its own redacting one; `debug_output_keeps_the_secret` below is what holds
/// that, through this struct and not only on the token alone.
#[derive(Debug)]
pub struct SignedIn {
    pub token: SessionToken,
    pub actor: Actor,
    /// The name the screen puts in the topbar (T4). Read here so a sign-in is
    /// one round trip.
    pub name: String,
}

/// The session secret. Printable as a header value and as a cookie, and
/// printable nowhere else: no `Debug`, no `Display`, no `Serialize`. The one
/// way out is `expose`, which is a word a reviewer can grep for.
#[derive(Clone)]
pub struct SessionToken(String);

impl SessionToken {
    pub(crate) fn new(hex: String) -> Self {
        SessionToken(hex)
    }

    /// The secret itself, for the two places that hand it to a caller: the
    /// login response body and the `Set-Cookie` header.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SessionToken(..)")
    }
}

/// The stored row as a session check reads it. Crate-internal, like
/// `UserRow`: `token_hash` is on it and a hash on a public struct is a hash in
/// a DTO one refactor later.
///
/// `shop_id` and `created_at` are columns of the table and not fields here:
/// every query is already filtered by the shop it was asked for, so reading
/// the column back would only offer a caller the chance to compare it instead,
/// and nothing decides anything on when a session began.
#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = sessions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct SessionRow {
    pub id: i32,
    pub user_id: i32,
    pub token_hash: String,
    pub last_seen_at: NaiveDateTime,
    pub ended_at: Option<NaiveDateTime>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = sessions)]
pub(crate) struct SessionRowWrite {
    pub shop_id: i32,
    pub user_id: i32,
    pub token_hash: String,
    pub created_at: NaiveDateTime,
    pub last_seen_at: NaiveDateTime,
}

#[cfg(test)]
mod tests {
    use super::{Actor, SessionToken, SignedIn};
    use crate::models::sql_types::Role;

    /// The whole point of the type. A token that reaches a log line is a
    /// token in every copy of that log, and this is the check that a
    /// `{:?}` somebody adds in a hurry cannot put one there, through the
    /// struct it actually travels in as well as on its own.
    #[test]
    fn debug_output_keeps_the_secret() {
        let secret = "0123456789abcdef";
        let t = SessionToken::new(secret.to_owned());
        assert_eq!(format!("{t:?}"), "SessionToken(..)");
        assert_eq!(format!("{:?}", Some(t.clone())), "Some(SessionToken(..))");

        let signed_in = SignedIn {
            token: t,
            actor: Actor {
                user_id: 1,
                shop_id: 1,
                role: Role::Owner,
                session_id: 1,
            },
            name: "Propriétaire".to_owned(),
        };
        let printed = format!("{signed_in:?}");
        assert!(!printed.contains(secret), "{printed}");
    }
}
