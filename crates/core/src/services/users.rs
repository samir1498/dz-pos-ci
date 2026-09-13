//! Who works in the shop, and how they prove it (features.md §5).
//!
//! Three roles, owner, manager and cashier, and two credentials: a PIN on the
//! till, because a cashier types it forty times a day between customers, and a
//! password everywhere else. Both are hashed with argon2id and a per-user salt
//! from the OS random source; nothing here ever stores or compares a
//! credential in the clear, and no function returns a hash.
//!
//! A user is never deleted. Documents, ledger movements and audit rows all
//! name a user by id and those ids are the record of who did what; the fiche
//! is switched off instead, which refuses every sign-in and changes nothing
//! else. `deactivate` and `reactivate` are the whole of it.
//!
//! The rules a screen would be tempted to restate live here: a PIN's shape,
//! the password's minimum, the last owner who cannot be switched off, and the
//! wait after a run of wrong PINs. A till stands on a counter in a shop with
//! the door open, so the wait is not optional (architecture.md rule 2: a rule
//! is written once, in a service).

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use chrono::{Duration, NaiveDateTime};
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::user::{Credentials, UserRowWrite};
use crate::repos::users as repo;
use crate::services::{audit, bounded_field, sessions};

pub use crate::models::user::{NewUser, Role, User};

/// Shortest PIN the till accepts. Four digits is what a bank card asks for and
/// what a cashier will actually use; anything shorter and the lockout below is
/// the only thing between a stranger and the drawer.
pub const MIN_PIN_DIGITS: usize = 4;
/// Longest. Six is the other length a person is used to typing; past that the
/// PIN pad stops being faster than a password, which is the only reason a PIN
/// exists here.
pub const MAX_PIN_DIGITS: usize = 6;
/// Shortest password. Eight characters, the floor NIST SP 800-63B sets for a
/// memorised secret, and it is a floor and not a pattern: no rule here asks
/// for a capital or a digit, because those push people to `Password1!` and
/// the standard stopped recommending them.
pub const MIN_PASSWORD_CHARS: usize = 8;

/// Wrong credentials a user gets before the file starts making them wait. Five
/// is enough room for a fat finger on a keypad and nowhere near enough to walk
/// a four-digit PIN: at ten thousand combinations and the wait below, a
/// stranger is looking at years.
pub const FAILURES_BEFORE_LOCKOUT: i32 = 5;
/// The first wait, in seconds, once the count is reached.
const FIRST_LOCKOUT_SECONDS: i64 = 30;
/// The longest it ever gets. It doubles per further failure and stops here:
/// fifteen minutes is already past the point where a stranger gives up, and
/// past it the rule would be punishing the cashier whose PIN is on a post-it
/// at home rather than the person it was written for. An owner clears it by
/// resetting the PIN, which is a thing somebody in the shop can actually do.
const MAX_LOCKOUT_SECONDS: i64 = 900;

/// The shop's users, active first then alphabetical.
pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<User>, CoreError> {
    repo::list(conn, shop_id)
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<User, CoreError> {
    repo::get(conn, shop_id, id)
}

/// Opens the fiche. No credential is set here: a PIN and a password each have
/// their own call with their own rules, so the shape of a PIN is checked in
/// one place and a user exists before anybody can sign in as them.
pub fn create(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    fields: NewUser,
) -> Result<User, CoreError> {
    let name = validated_name(&fields.name)?;
    conn.transaction(|conn| {
        refuse_taken_name(conn, shop_id, &name, None)?;
        let created = repo::insert(
            conn,
            &UserRowWrite {
                shop_id,
                name,
                role: fields.role,
                active: true,
                updated_at: stamp(),
            },
        )?;
        record(
            conn,
            shop_id,
            actor_id,
            audit::ACTION_CREATE_USER,
            created.id,
            None,
            Some(&created),
        )?;
        Ok(created)
    })
}

pub fn rename(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    id: i32,
    name: &str,
) -> Result<User, CoreError> {
    let name = validated_name(name)?;
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        refuse_taken_name(conn, shop_id, &name, Some(id))?;
        let after = write_fiche(conn, shop_id, id, name, before.role, before.active)?;
        record(
            conn,
            shop_id,
            actor_id,
            audit::ACTION_RENAME_USER,
            id,
            Some(&before),
            Some(&after),
        )?;
        Ok(after)
    })
}

/// Moves a user to another role. The last active owner cannot be moved off
/// `owner` for the reason they cannot be switched off: the shop would have
/// nobody who may hand the role back.
///
/// Unlike `deactivate`, this carries no unconditional `actor_id == id`
/// refusal: an owner may move their own row today, because with two or
/// more active owners `refuse_last_owner` has nothing to say about it and
/// nothing on this branch calls `set_role` on a route a manager or a
/// cashier could reach anyway. That is a fork from `deactivate`, not a rule
/// this function decided; it exists only because nothing exposes `set_role`
/// over HTTP yet (M4 T8). Whoever wires a role-change route should decide
/// on purpose whether an owner may demote themselves out of `owner` in a
/// two-owner shop, rather than inherit this gap unread: today that owner
/// could do it, and would not be locked out by it the way a self-deactivate
/// would, but the shop would still be down to one less owner than it meant
/// to give up.
///
/// Deliberately does not end any session, unlike `set_pin`, `set_password`
/// and `deactivate`. Those three each invalidate something
/// `by_token_hash`'s live join does not itself re-check on every request: a
/// credential a session was opened on, or the `active` flag the join reads
/// straight off the row. A role is not in that list: the join reads
/// `users.role` fresh on every single request a session makes, so a role
/// lowered here is already in force on the very next thing that session
/// tries to do, and there is nothing left for ending the session to buy.
/// Ending it anyway would cost something real for no such gain: a role
/// change is routine shop administration and not a response to a suspected
/// leak, and signing a manager out mid-sale because their `CommitMoney`
/// permission was turned off would be a worse shop for a control this table
/// already gives for free.
pub fn set_role(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    id: i32,
    role: Role,
) -> Result<User, CoreError> {
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        if role != Role::Owner {
            refuse_last_owner(conn, shop_id, &before, "role")?;
        }
        let after = write_fiche(conn, shop_id, id, before.name.clone(), role, before.active)?;
        record(
            conn,
            shop_id,
            actor_id,
            audit::ACTION_SET_ROLE,
            id,
            Some(&before),
            Some(&after),
        )?;
        Ok(after)
    })
}

/// Sets or resets the PIN, and clears any lockout with it: an owner resetting
/// a PIN is how a locked-out cashier gets back to the till.
///
/// Also ends every session the fiche is holding, because a PIN reset is
/// exactly the moment a session must stop trusting the credential it was
/// opened on: an owner who suspects a cashier's PIN was watched resets it so
/// the till that cashier is standing at stops working, not just so the next
/// sign-in wants the new one. `active` does not change here, so the live join
/// `by_token_hash` runs on every request has nothing to catch on its own
/// (unlike `deactivate`, below); ending the rows is the only thing that does.
///
/// `acting_session_id` is the session this call itself came in on, if any.
/// `None` when nothing is asking through a session (a seed script, a
/// migration). When it is `Some` and the owner is resetting their own PIN,
/// that one session is kept alive and every other one of theirs is ended: the
/// alternative, ending all of them unconditionally, would sign the owner out
/// of the very screen they used to fix the credential, mid-task, for no
/// security gain the "the rest of the shop's PINs might be watched too"
/// reasoning above does not already cover for that session too.
pub fn set_pin(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    id: i32,
    pin: &str,
    acting_session_id: Option<i32>,
) -> Result<User, CoreError> {
    validate_pin(pin)?;
    let hash = hash(pin)?;
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        let after = repo::set_pin_hash(conn, shop_id, id, &hash, stamp())?;
        end_sessions_after_credential_change(conn, shop_id, actor_id, id, acting_session_id)?;
        record(
            conn,
            shop_id,
            actor_id,
            audit::ACTION_SET_PIN,
            id,
            Some(&before),
            Some(&after),
        )?;
        Ok(after)
    })
}

/// The one door into a shop nobody has ever signed into. `ManageUsers` is
/// the owner's alone, and the owner cannot hold a session to use it before
/// somebody has set their PIN, which is what this call is for: it needs no
/// actor and no session, and it acts on the shop's own owner rather than on
/// an id a caller names, because nobody signed in yet is who is supposed to
/// choose one.
///
/// Whether the first-PIN door is still open: nobody in the shop has a
/// credential yet. The desktop asks this of `/health` so it can show the
/// onboarding pad instead of a sign-in for a user who does not exist.
pub fn shop_needs_first_pin(conn: &mut SqliteConnection, shop_id: i32) -> Result<bool, CoreError> {
    Ok(!repo::any_credential_set(conn, shop_id)?)
}

/// Two refusals, and each is the whole of a rule that would otherwise be
/// argued over in an API handler:
///
/// - any credential anywhere in the shop already exists. The door shuts for
///   good the moment an owner signs in the ordinary way and sets or resets a
///   PIN through `set_pin`, which is `ManageUsers`, behind a session. A
///   second call here after that is not a retry, it is a stranger with the
///   launch token (the desktop process on this machine) trying the door
///   again after it closed;
/// - the shop does not have exactly one active owner to give the PIN to.
///   Nothing to act on, or more than one candidate, and this call refuses
///   rather than guess which row a caller meant.
pub fn claim_first_pin(
    conn: &mut SqliteConnection,
    shop_id: i32,
    pin: &str,
) -> Result<User, CoreError> {
    validate_pin(pin)?;
    let hash = hash(pin)?;
    conn.transaction(|conn| {
        if repo::any_credential_set(conn, shop_id)? {
            return Err(CoreError::validation(
                "pin",
                "this shop has already been signed into; an owner resets a PIN from the users screen",
            ));
        }
        let owner = repo::sole_active_owner(conn, shop_id)?.ok_or_else(|| {
            CoreError::validation(
                "pin",
                "this shop has no single active owner to give the first PIN to",
            )
        })?;
        let after = repo::set_pin_hash(conn, shop_id, owner.id, &hash, stamp())?;
        record(
            conn,
            shop_id,
            owner.id,
            audit::ACTION_CLAIM_FIRST_PIN,
            owner.id,
            Some(&owner),
            Some(&after),
        )?;
        Ok(after)
    })
}

/// The same reasoning as `set_pin`'s, on the other credential: a password
/// reset ends every session the fiche holds, except the caller's own when the
/// caller is resetting their own password. No route calls this yet
/// (`crates/api/src/routes/users.rs` only wires the PIN), but the rule is the
/// same fiche and the same finding, so it is fixed here too rather than left
/// for whoever wires the route to rediscover.
pub fn set_password(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    id: i32,
    password: &str,
    acting_session_id: Option<i32>,
) -> Result<User, CoreError> {
    validate_password(password)?;
    let hash = hash(password)?;
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        let after = repo::set_password_hash(conn, shop_id, id, &hash, stamp())?;
        end_sessions_after_credential_change(conn, shop_id, actor_id, id, acting_session_id)?;
        record(
            conn,
            shop_id,
            actor_id,
            audit::ACTION_SET_PASSWORD,
            id,
            Some(&before),
            Some(&after),
        )?;
        Ok(after)
    })
}

/// The one rule `set_pin` and `set_password` share: end every session the
/// target fiche holds, except the caller's own current one when the caller is
/// resetting their own credential. `acting_session_id` is `None` when nothing
/// is asking through a session at all, in which case there is no "own
/// session" to spare and every session ends.
fn end_sessions_after_credential_change(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    target_id: i32,
    acting_session_id: Option<i32>,
) -> Result<(), CoreError> {
    let now = stamp();
    match acting_session_id.filter(|_| actor_id == target_id) {
        Some(keep) => sessions::end_all_for_user_except(conn, shop_id, target_id, keep, now)?,
        None => sessions::end_all_for_user(conn, shop_id, target_id, now)?,
    };
    Ok(())
}

/// Switches a user off. Nothing is deleted: every document, ledger row and
/// audit entry they wrote goes on naming this id, which is the point of
/// switching off rather than removing.
///
/// Two refusals, and both live here rather than on the settings screen, so a
/// second caller cannot get them wrong: the last active owner stays, or the
/// shop has nobody who may hand the role to anybody; and nobody switches
/// themselves off, which is the same shop locked out by one wrong click.
pub fn deactivate(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    id: i32,
) -> Result<User, CoreError> {
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        if actor_id == id {
            return Err(CoreError::validation(
                "active",
                "a user does not switch their own fiche off",
            ));
        }
        refuse_last_owner(conn, shop_id, &before, "active")?;
        let after = write_fiche(conn, shop_id, id, before.name.clone(), before.role, false)?;
        // Unconditional: the refusal three lines up means `actor_id != id`
        // whenever this line runs, so there is no "own session" to spare the
        // way `set_pin` spares one. Without this, a fiche switched off and
        // switched back on inside the idle window would hand its old token
        // back to whoever was still holding it, with no new sign-in: `active`
        // flips back to true before the row that should have ended the
        // session ever gets written.
        sessions::end_all_for_user(conn, shop_id, id, stamp())?;
        record(
            conn,
            shop_id,
            actor_id,
            audit::ACTION_DEACTIVATE_USER,
            id,
            Some(&before),
            Some(&after),
        )?;
        Ok(after)
    })
}

pub fn reactivate(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    id: i32,
) -> Result<User, CoreError> {
    conn.transaction(|conn| {
        let before = repo::get(conn, shop_id, id)?;
        let after = write_fiche(conn, shop_id, id, before.name.clone(), before.role, true)?;
        record(
            conn,
            shop_id,
            actor_id,
            audit::ACTION_REACTIVATE_USER,
            id,
            Some(&before),
            Some(&after),
        )?;
        Ok(after)
    })
}

/// The user, if that is their PIN and they are still active.
///
/// `now` is UTC and is passed in rather than read here, the way every dated
/// rule in this crate takes its moment: the lockout is a stretch of seconds
/// and `locked_until` is written on the same clock it is compared against. It
/// is deliberately not the shop calendar `services::clock` answers, because an
/// offset applied on one side and not the other is a lockout an hour long.
pub fn verify_pin(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    pin: &str,
    now: NaiveDateTime,
) -> Result<User, CoreError> {
    let found = repo::credentials(conn, shop_id, user_id)
        .map(Some)
        .or_else(|e| match e {
            // A PIN pad picks a row off the list, so an id nobody answers to
            // is not how a stranger enumerates a shop's staff. It is still
            // answered as a refusal and not as a 404: whoever is standing at
            // the till learns the same thing either way, and the screen has
            // one sentence to say instead of two.
            CoreError::NotFound { .. } => Ok(None),
            other => Err(other),
        })?;
    settle(conn, shop_id, found, pin, Credentials::pin_hash_of, now)
}

/// The same on the password side, found by name because that is what the
/// screen asks for. A name nobody answers to is refused exactly as a wrong
/// password is, and takes the same work to refuse.
pub fn verify_password(
    conn: &mut SqliteConnection,
    shop_id: i32,
    name: &str,
    password: &str,
    now: NaiveDateTime,
) -> Result<User, CoreError> {
    let found = repo::credentials_by_name(conn, shop_id, name.trim())?;
    settle(
        conn,
        shop_id,
        found,
        password,
        Credentials::password_hash_of,
        now,
    )
}

/// The one sign-in rule, shared by both credentials so the lockout, the
/// deactivated fiche and the unknown user are answered the same way whichever
/// screen asked.
///
/// Not wrapped in a transaction, and that is the whole design of it: a
/// refusal returns `Err`, and an `Err` out of `conn.transaction` rolls the
/// transaction back. A failure counter incremented inside one would be undone
/// by the very refusal it was counting, which is the M2 credit-block finding
/// on another table. Each write here commits on its own, before the refusal
/// leaves.
// One counter and one lock per user, shared by the PIN and the password: five
// wrong passwords at the office screen leave the same person waiting at the
// till. That is deliberate (the person is the thing being protected, not the
// screen) and it is also a way for one member of staff to keep another out of
// the till for a quarter of an hour. Samir has the question; splitting it is a
// column and this function, and T2 is the last comfortable moment to do it.
fn settle(
    conn: &mut SqliteConnection,
    shop_id: i32,
    found: Option<Credentials>,
    secret: &str,
    which: fn(&Credentials) -> Option<&str>,
    now: NaiveDateTime,
) -> Result<User, CoreError> {
    let Some(creds) = found else {
        // No row to count against, so nothing is written. The work is done
        // anyway: skipping it would answer an unknown name in a millisecond
        // and a known one in fifty, which is the staff list read off the
        // clock.
        let _ = matches(secret, DUMMY_HASH);
        return Err(CoreError::AuthRefused);
    };
    if let Some(until) = creds.locked_until.filter(|until| *until > now) {
        // Refused without counting. Counting here would let anybody at the
        // counter hold the owner out of their own till for as long as they
        // cared to keep typing.
        return Err(CoreError::LockedOut {
            retry_after_seconds: (until - now).num_seconds().max(1),
        });
    }
    // The credential is checked whether or not the fiche is active and
    // whether or not one was ever set, and the answers are merged afterwards.
    // A deactivated user, or one with no PIN, that refused before doing the
    // work would say by how long it took that this is a name the shop used to
    // employ, or one nobody has finished setting up.
    let stored = which(&creds).unwrap_or(DUMMY_HASH);
    let good = matches(secret, stored) && creds.active && which(&creds).is_some();
    if !good {
        let failures = creds.pin_failures.saturating_add(1);
        let locked_until = lockout_until(failures, now);
        repo::record_failure(conn, shop_id, creds.id, failures, locked_until)?;
        // Only on the crossing, so the log holds the lockout and not every
        // mistyped digit. The actor is the user the attempts were made on:
        // nobody knows who was standing there, and claiming otherwise in an
        // audit log is worse than saying nothing.
        if let Some(until) = locked_until.filter(|_| failures == FAILURES_BEFORE_LOCKOUT) {
            audit::record(
                conn,
                shop_id,
                creds.id,
                audit::Change {
                    action: audit::ACTION_LOCK_OUT_USER,
                    entity: "user",
                    entity_id: Some(creds.id),
                    before: None,
                    after: Some(
                        serde_json::json!({
                            "failures": failures,
                            "locked_until": until.to_string(),
                        })
                        .to_string(),
                    ),
                },
            )?;
        }
        return Err(CoreError::AuthRefused);
    }
    repo::clear_failures(conn, shop_id, creds.id)?;
    repo::get(conn, shop_id, creds.id)
}

/// When the user may try again, or `None` while they are still under the
/// allowance. It doubles per failure past the fifth and stops at the cap, so
/// the fifth wrong PIN costs half a minute and the tenth a quarter of an hour.
fn lockout_until(failures: i32, now: NaiveDateTime) -> Option<NaiveDateTime> {
    if failures < FAILURES_BEFORE_LOCKOUT {
        return None;
    }
    // `min` before the shift as well as after: past about sixty failures the
    // shift itself would overflow, and a till left with a finger on a key
    // gets there.
    let steps = u32::try_from(failures - FAILURES_BEFORE_LOCKOUT)
        .unwrap_or(u32::MAX)
        .min(32);
    let seconds = FIRST_LOCKOUT_SECONDS
        .saturating_mul(1i64 << steps)
        .min(MAX_LOCKOUT_SECONDS);
    Some(now + Duration::seconds(seconds))
}

/// What an unknown name is checked against so that it costs what a known one
/// costs. A well-formed argon2id string at this app's own parameters whose
/// digest matches no secret: it parses, so the verifier does the full two
/// passes over 19 MiB before answering no, and it is not a credential this
/// app ever writes. `dummy_hash_is_well_formed` below is what holds that it
/// still parses, because a string that stopped parsing would answer an
/// unknown name in microseconds and hand the staff list to anybody with a
/// clock. What the suite holds is that the string parses and matches nothing;
/// that the two answers take the same time is reasoned from the verifier
/// doing the same work, not measured, and measuring it would want a benchmark
/// rather than a test.
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$\
    ZHpwb3Mtbm8tc3VjaC11c2Vy$Qq1sMUS3FTRWiXf5xZTQ7ARAYhUXQ1kXBDJ2vcPYZTk";

/// argon2id with the crate's own defaults: version 19, 19 MiB of memory, two
/// passes, one lane. They are the OWASP configuration for argon2id and they
/// are what `tests/users_service.rs` reads back off a stored string, so the
/// numbers are asserted rather than described.
///
/// A PIN has at most a million combinations, so no cost setting makes one
/// strong on its own; what protects it is the lockout above, and what this
/// protects is a stolen shop file, where an attacker has no counter to trip.
/// The parameters travel inside every stored hash, so raising them later
/// changes new hashes and leaves old ones readable.
fn hasher() -> Argon2<'static> {
    Argon2::default()
}

fn hash(secret: &str) -> Result<String, CoreError> {
    // Sixteen bytes off the OS random source, per user, carried in the stored
    // string. Two cashiers who pick 2580 do not get the same hash, and a
    // rainbow table for four-digit PINs is worth nothing here.
    let salt = SaltString::generate(&mut OsRng);
    hasher()
        .hash_password(secret.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(CoreError::Hash)
}

/// Whether the secret is the one behind the stored string. A string that does
/// not parse is not a match and is not an error: `'!unset'`, the sentinel the
/// first migration writes, is exactly that, so a user nobody has given a PIN
/// to fails closed instead of 500ing the sign-in open.
fn matches(secret: &str, stored: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored) else {
        return false;
    };
    hasher().verify_password(secret.as_bytes(), &parsed).is_ok()
}

/// Four to six digits, and not one of the two shapes everybody picks first.
/// Refused here rather than on the keypad: the API and the mobile app will
/// both set a PIN and neither may decide this for itself (architecture.md
/// rule 2).
pub fn validate_pin(pin: &str) -> Result<(), CoreError> {
    let digits = pin.chars().count();
    if !(MIN_PIN_DIGITS..=MAX_PIN_DIGITS).contains(&digits) {
        return Err(CoreError::validation("pin", "a PIN is four to six digits"));
    }
    if !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(CoreError::validation(
            "pin",
            "a PIN is digits only; the till has a number pad and nothing else",
        ));
    }
    let values: Vec<i32> = pin
        .chars()
        // Every character is an ASCII digit by the check above, so the
        // subtraction is in range; the fold is written so it cannot panic
        // rather than trusting that from two lines away.
        .map(|c| i32::from(c as u8) - i32::from(b'0'))
        .collect();
    let steps: Vec<i32> = values.windows(2).map(|pair| pair[1] - pair[0]).collect();
    if steps.iter().all(|step| *step == 0) {
        return Err(CoreError::validation(
            "pin",
            "a PIN of one digit repeated is the first one anybody tries",
        ));
    }
    if steps.iter().all(|step| *step == 1) || steps.iter().all(|step| *step == -1) {
        return Err(CoreError::validation(
            "pin",
            "a PIN that counts up or down is the second one anybody tries",
        ));
    }
    Ok(())
}

/// Eight characters at least, and nothing else asked of it. Not trimmed: a
/// space is a character somebody chose, and silently dropping one would refuse
/// the password they set the next time they type it.
pub fn validate_password(password: &str) -> Result<(), CoreError> {
    if password.chars().count() < MIN_PASSWORD_CHARS {
        return Err(CoreError::validation(
            "password",
            "a password is at least eight characters",
        ));
    }
    bounded_field("password", password)?;
    Ok(())
}

fn validated_name(name: &str) -> Result<String, CoreError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::validation(
            "name",
            "a user needs a name; it is what the sign-in screen lists",
        ));
    }
    bounded_field("name", name)?;
    Ok(name.to_string())
}

fn refuse_taken_name(
    conn: &mut SqliteConnection,
    shop_id: i32,
    name: &str,
    exclude: Option<i32>,
) -> Result<(), CoreError> {
    if repo::name_taken(conn, shop_id, name, exclude)? {
        return Err(CoreError::conflict(
            "name",
            "somebody in this shop already signs in under that name",
        ));
    }
    Ok(())
}

/// The rule that keeps a shop from locking itself out of its own settings: if
/// this user is the only active owner, they stay an active owner. `field` is
/// the one the caller was changing, so the message lands under it.
fn refuse_last_owner(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user: &User,
    field: &str,
) -> Result<(), CoreError> {
    if user.role != Role::Owner || !user.active {
        return Ok(());
    }
    if repo::count_active_with_role(conn, shop_id, Role::Owner)? > 1 {
        return Ok(());
    }
    Err(CoreError::validation(
        field,
        "this is the shop's last owner; give the role to somebody else first",
    ))
}

fn write_fiche(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    name: String,
    role: Role,
    active: bool,
) -> Result<User, CoreError> {
    repo::update(
        conn,
        shop_id,
        id,
        &UserRowWrite {
            shop_id,
            name,
            role,
            active,
            updated_at: stamp(),
        },
    )
}

/// UTC, not the shop's calendar, for the reason `services::customers` gives:
/// `created_at` is SQLite's CURRENT_TIMESTAMP and a fiche whose `updated_at`
/// reads an hour after its own `created_at` is a wrong figure in a row nobody
/// would think to doubt.
fn stamp() -> NaiveDateTime {
    chrono::Utc::now().naive_utc()
}

/// The fiche as the audit log stores it. Whether a credential is set, never
/// the credential: a hash written here is a hash in every backup of the file
/// and in every export of the log.
fn as_json(user: &User) -> String {
    serde_json::json!({
        "name": user.name,
        "role": user.role.as_str(),
        "active": user.active,
        "has_pin": user.has_pin,
        "has_password": user.has_password,
    })
    .to_string()
}

fn record(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    action: &'static str,
    target_id: i32,
    before: Option<&User>,
    after: Option<&User>,
) -> Result<(), CoreError> {
    audit::record(
        conn,
        shop_id,
        actor_id,
        audit::Change {
            action,
            entity: "user",
            entity_id: Some(target_id),
            before: before.map(as_json),
            after: after.map(as_json),
        },
    )
}

#[cfg(test)]
mod tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// The whole timing argument for an unknown name rests on this string
    /// parsing. Asserted here rather than trusted, because it is typed out by
    /// hand and nothing else in the crate would notice it rotting.
    #[test]
    fn dummy_hash_is_well_formed_and_matches_nothing() {
        assert!(
            PasswordHash::new(DUMMY_HASH).is_ok(),
            "the stand-in hash no longer parses, so an unknown name is answered in microseconds"
        );
        for guess in ["", "1234", "dzpos-no-such-user", "!unset"] {
            assert!(!matches(guess, DUMMY_HASH), "{guess} matched the stand-in");
        }
    }

    /// The sentinel the first migration writes fails closed: it is not a PHC
    /// string, so it is a refusal and never a `Hash` error that would 500 a
    /// sign-in open.
    #[test]
    fn the_unset_sentinel_is_a_refusal_and_not_an_error() {
        assert!(!matches("!unset", crate::models::user::PIN_UNSET));
        assert!(!matches("1357", crate::models::user::PIN_UNSET));
    }

    /// The doubling the shop chose, read straight off the function rather
    /// than through a database.
    #[test]
    fn the_wait_doubles_from_thirty_seconds_and_stops_at_a_quarter_of_an_hour() {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 9, 11)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(lockout_until(4, now), None);
        let seconds = |failures| {
            lockout_until(failures, now)
                .map(|until| (until - now).num_seconds())
                .unwrap_or_default()
        };
        assert_eq!(
            (5..=12).map(seconds).collect::<Vec<_>>(),
            vec![30, 60, 120, 240, 480, 900, 900, 900]
        );
        // A till with something resting on a key does not overflow the shift.
        assert_eq!(seconds(i32::MAX), MAX_LOCKOUT_SECONDS);
    }
}
