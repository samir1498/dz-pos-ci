//! Who is acting on a request (M4 T2, `docs/architecture.md` § Transport and
//! auth). The launch token says the caller is a process on this machine; a
//! session says which person is at the keyboard, and the two are separate
//! gates on purpose: folding them together would mean one secret that both
//! survives a restart and names a user, which is neither.
//!
//! The whole sign-in rule is `services::users`: the single refusal, the wrong
//! try counter and the lockout are settled there and this module calls them.
//! What is decided here is the session itself: the token, how it is stored,
//! and when it stops counting.
//!
//! ## The token
//!
//! Thirty-two bytes from the OS random source, shown as 64 hex characters.
//! Stored as SHA-256 of that string, hex, and never in the clear, so a stolen
//! shop file or a backup of it hands nobody a live session. The found row's
//! hash is still compared byte for byte in constant time, which the unique
//! index makes redundant and which costs a microsecond: a lookup that ever
//! stops being exact (a LIKE, a case fold, a prefix index) would then be
//! caught by the compare rather than by nobody.
//!
//! SHA-256 and not the argon2id that hashes a PIN, which is the one place
//! this module departs from "a credential is stored the way a password is".
//! A PIN is at most a million combinations and needs a slow hash to survive a
//! stolen file; this token is 256 bits of OS randomness, which no amount of
//! guessing reaches, and a salted argon2 string cannot be looked up by, so
//! every request would have to verify every live session in turn at 19 MiB
//! and two passes each. That is a sign-in cost paid on every keystroke of
//! every screen.
//!
//! ## When a session stops counting
//!
//! Four ways, and a caller cannot tell them apart, which is deliberate: a
//! token nobody ever issued, a session signed out, a session idle past the
//! shop's setting, and a session whose user has been switched off all answer
//! `None`. The screen has one sentence to say (sign in again) and a stranger
//! learns nothing from which it was.
//!
//! The idle time slides: it is measured from the last request the session
//! carried, not from the sign-in, so a cashier working through a queue is
//! never thrown out mid-sale and a till nobody has touched since lunch is.

use chrono::{Duration, NaiveDateTime};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::session::{SessionRowWrite, SessionToken};
use crate::repos::sessions as repo;
use crate::services::{preferences, users};

pub use crate::models::session::{Actor, SignedIn};
pub use crate::repos::sessions::end_all_for_user;

/// Bytes of randomness in a session token; shown as 64 hex characters. The
/// same figure the launch token uses, for the same reason: 256 bits is past
/// anything a guess reaches and it is still a short header value.
const TOKEN_BYTES: usize = 32;

/// How long a row whose session is finished with stays in the file before a
/// sign-in sweeps it. A fortnight past the last thing it did: long enough
/// that nobody loses a row somebody was about to ask about, short enough that
/// a till signing in twice a day does not grow a table forever.
const KEEP_ENDED_FOR_DAYS: i64 = 14;

/// Signs a user in with their PIN. The user is picked off the list by id,
/// which is what the till's PIN pad does, so nothing here tells a stranger
/// whether an id belongs to anybody.
pub fn sign_in_with_pin(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    pin: &str,
    now: NaiveDateTime,
) -> Result<SignedIn, CoreError> {
    let user = users::verify_pin(conn, shop_id, user_id, pin, now)?;
    open(conn, shop_id, user, now)
}

/// The same on the password side, found by name because that is what the
/// screen asks for. `services::users` answers an unknown name exactly as it
/// answers a wrong password, and takes the same work to do it, so nothing
/// about signing in leaks whether a name exists.
pub fn sign_in_with_password(
    conn: &mut SqliteConnection,
    shop_id: i32,
    name: &str,
    password: &str,
    now: NaiveDateTime,
) -> Result<SignedIn, CoreError> {
    let user = users::verify_password(conn, shop_id, name, password, now)?;
    open(conn, shop_id, user, now)
}

/// The half both sign-ins share once the credential has been believed: mint
/// the token, store its hash, and hand the secret back the one time it is
/// ever in the clear.
///
/// Not wrapped in a transaction, for the reason `services::users::settle`
/// gives about the wrong-try counter: the sweep below and the insert are
/// independent writes and a refusal from neither should undo the other.
fn open(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user: users::User,
    now: NaiveDateTime,
) -> Result<SignedIn, CoreError> {
    // On the way in rather than on a timer: a shop file has no scheduler of
    // its own, sign-in is the one moment a session table is certainly being
    // written anyway, and a till signs in at least once a day.
    let _ = repo::delete_ended_before(conn, shop_id, now - Duration::days(KEEP_ENDED_FOR_DAYS));

    let token = mint()?;
    repo::insert(
        conn,
        &SessionRowWrite {
            shop_id,
            user_id: user.id,
            token_hash: token_digest(token.expose()),
            created_at: now,
            last_seen_at: now,
        },
    )?;
    Ok(SignedIn {
        token,
        actor: Actor {
            user_id: user.id,
            shop_id,
            role: user.role,
        },
        name: user.name,
    })
}

/// Who is acting, if this token is a live session of this shop, and `None` if
/// it is anything else. The `Ok(None)` is the whole answer: the caller turns
/// it into one 401 and the four ways a session can be dead are not told
/// apart on the wire.
///
/// The side effect is the sliding idle time: a session that counted has its
/// `last_seen_at` moved to now, so the next request measures from this one.
pub fn resolve(
    conn: &mut SqliteConnection,
    shop_id: i32,
    token: &str,
    now: NaiveDateTime,
) -> Result<Option<Actor>, CoreError> {
    let Some(found) = live(conn, shop_id, token, now)? else {
        return Ok(None);
    };
    repo::touch(conn, shop_id, found.row.id, now)?;
    Ok(Some(Actor {
        user_id: found.row.user_id,
        shop_id,
        role: found.role,
    }))
}

/// The same read without the touch, for `/auth/me`, which reports what is
/// there and should not itself keep a session alive.
pub fn describe(
    conn: &mut SqliteConnection,
    shop_id: i32,
    token: &str,
    now: NaiveDateTime,
) -> Result<Option<(Actor, String)>, CoreError> {
    Ok(live(conn, shop_id, token, now)?.map(|found| {
        (
            Actor {
                user_id: found.row.user_id,
                shop_id,
                role: found.role,
            },
            found.name,
        )
    }))
}

fn live(
    conn: &mut SqliteConnection,
    shop_id: i32,
    token: &str,
    now: NaiveDateTime,
) -> Result<Option<repo::Live>, CoreError> {
    let Some(found) = repo::by_token_hash(conn, shop_id, &token_digest(token))? else {
        return Ok(None);
    };
    if !constant_time_eq(&found.row.token_hash, &token_digest(token)) {
        return Ok(None);
    }
    if found.row.ended_at.is_some() || !found.user_active {
        return Ok(None);
    }
    if now - found.row.last_seen_at >= preferences::session_idle(conn, shop_id)? {
        return Ok(None);
    }
    Ok(Some(found))
}

/// Ends the session this token names. Answering `Ok` whether or not there was
/// one is the point: signing out is not a place to tell a caller that their
/// token was already dead, and a screen that clicked twice gets one answer.
pub fn sign_out(
    conn: &mut SqliteConnection,
    shop_id: i32,
    token: &str,
    now: NaiveDateTime,
) -> Result<(), CoreError> {
    if let Some(found) = repo::by_token_hash(conn, shop_id, &token_digest(token))? {
        repo::end(conn, shop_id, found.row.id, now)?;
    }
    Ok(())
}

/// A fresh token from the operating system's randomness, through the same
/// source `services::users` takes a salt from, so this crate has one place it
/// asks the OS for bytes and no second random crate in the tree.
fn mint() -> Result<SessionToken, CoreError> {
    use argon2::password_hash::rand_core::{OsRng, RngCore};
    let mut bytes = [0u8; TOKEN_BYTES];
    OsRng.fill_bytes(&mut bytes);
    let mut hex = String::with_capacity(TOKEN_BYTES * 2);
    for b in bytes {
        use std::fmt::Write;
        // The buffer has the room reserved above and `write!` to a String is
        // infallible, so there is nothing here to answer for; it is written
        // this way rather than with an `unwrap` the workspace denies.
        let _ = write!(hex, "{b:02x}");
    }
    Ok(SessionToken::new(hex))
}

/// SHA-256 of the token, lowercase hex. The stored form, and the only form
/// this crate ever compares.
///
/// Public because it is not a secret: it is a published hash of a value the
/// caller already holds, and a test that wants to plant or find a session row
/// should ask for the stored form rather than guess at it. Nothing here turns
/// a digest back into a token.
pub fn token_digest(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hex = String::with_capacity(64);
    for b in Sha256::digest(token.as_bytes()) {
        use std::fmt::Write;
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

/// Whether two stored hashes are the same string, every byte compared
/// whatever the first mismatch, the way `LaunchToken::matches` does it. The
/// lookup is already exact, so this is the belt to that index's braces.
fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// The shape the whole design rests on, asserted rather than described:
    /// a token is 64 hex characters, never the same twice, and what is stored
    /// is not it.
    #[test]
    fn a_token_is_sixty_four_hex_characters_and_never_the_same_twice() {
        let a = mint().unwrap();
        let b = mint().unwrap();
        assert_eq!(a.expose().len(), 64);
        assert!(a.expose().bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a.expose(), b.expose());
        assert_ne!(token_digest(a.expose()), a.expose());
        assert_eq!(token_digest(a.expose()).len(), 64);
    }

    /// The one published vector for SHA-256, so a refactor that reached for
    /// another digest, or hexed it the other way round, fails here.
    #[test]
    fn the_digest_is_sha_256_and_lowercase_hex() {
        assert_eq!(
            token_digest("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(token_digest(""), token_digest(""));
        assert_ne!(token_digest("a"), token_digest("A"));
    }

    #[test]
    fn the_compare_is_by_value_and_refuses_a_different_length() {
        assert!(constant_time_eq("abcd", "abcd"));
        assert!(!constant_time_eq("abcd", "abce"));
        assert!(!constant_time_eq("abcd", "abcd "));
        assert!(!constant_time_eq("", "a"));
        assert!(constant_time_eq("", ""));
    }
}
