// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Sessions: who is acting on a request, and when they stop being (M4 T2).
//!
//! What this file holds to is the part a screen or a route must not restate:
//! the token is never stored in the clear, the idle time slides and comes
//! from the shop's own setting, and the four ways a session can be dead are
//! one answer on the way out.

use chrono::{Duration, NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sql_types::Text;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::services::preferences;
use dzpos_kernel::services::sessions;
use dzpos_kernel::services::users::{self, NewUser, Role};

const SHOP: i32 = 1;
/// The owner the first migration seeds, and a real row of `users`.
const OWNER: i32 = 1;

mod common;

use common::open_temp;

#[derive(QueryableByName)]
struct Stored {
    #[diesel(sql_type = Text)]
    token_hash: String,
}

fn at(h: u32, min: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, 11)
        .unwrap()
        .and_hms_opt(h, min, 0)
        .unwrap()
}

fn noon() -> NaiveDateTime {
    at(12, 0)
}

/// A cashier with a PIN, which is what most of this file signs in as.
fn a_cashier(conn: &mut SqliteConnection, name: &str, pin: &str) -> i32 {
    let user = users::create(
        conn,
        SHOP,
        OWNER,
        NewUser {
            name: name.to_owned(),
            role: Role::Cashier,
        },
    )
    .unwrap();
    users::set_pin(conn, SHOP, OWNER, user.id, pin, None).unwrap();
    user.id
}

fn every_stored_hash(conn: &mut SqliteConnection) -> Vec<String> {
    diesel::sql_query("SELECT token_hash FROM sessions")
        .load::<Stored>(conn)
        .unwrap()
        .into_iter()
        .map(|r| r.token_hash)
        .collect()
}

/// The sign-in a route makes, and the actor it hands back.
#[test]
fn a_right_pin_opens_a_session_that_names_the_user_and_their_role() {
    let (_dir, mut conn) = open_temp();
    let cashier = a_cashier(&mut conn, "Karim", "2580");

    let signed_in = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();
    assert_eq!(signed_in.actor.user_id, cashier);
    assert_eq!(signed_in.actor.shop_id, SHOP);
    assert_eq!(signed_in.actor.role, Role::Cashier);
    assert_eq!(signed_in.name, "Karim");

    let actor = sessions::resolve(&mut conn, SHOP, signed_in.token.expose(), noon())
        .unwrap()
        .expect("the token that was just minted did not resolve");
    assert_eq!(actor, signed_in.actor);
}

/// The password side, which finds the user by the name the screen asked for.
#[test]
fn a_right_password_opens_a_session_the_same_way() {
    let (_dir, mut conn) = open_temp();
    users::set_password(&mut conn, SHOP, OWNER, OWNER, "correct horse", None).unwrap();
    let owner = users::get(&mut conn, SHOP, OWNER).unwrap();

    let signed_in =
        sessions::sign_in_with_password(&mut conn, SHOP, &owner.name, "correct horse", noon())
            .unwrap();
    assert_eq!(signed_in.actor.user_id, OWNER);
    assert_eq!(signed_in.actor.role, Role::Owner);
    assert!(
        sessions::resolve(&mut conn, SHOP, signed_in.token.expose(), noon())
            .unwrap()
            .is_some()
    );
}

/// The rule the brief calls out by name: a session token is a credential, so
/// what the file holds is a hash and never the token. A stolen shop file, or
/// a backup of one, must not hand anybody a live session.
#[test]
fn the_token_is_never_in_the_file_and_what_is_stored_is_not_it() {
    let (_dir, mut conn) = open_temp();
    let cashier = a_cashier(&mut conn, "Karim", "2580");
    let signed_in = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();

    let stored = every_stored_hash(&mut conn);
    assert_eq!(stored.len(), 1);
    assert_ne!(stored[0], signed_in.token.expose());
    assert!(!stored[0].contains(signed_in.token.expose()));
    assert_eq!(stored[0].len(), 64);
    assert!(stored[0].bytes().all(|c| c.is_ascii_hexdigit()));

    // And nothing anywhere in the row holds it either: the whole table as
    // text, in case a column is added later that a person would paste one
    // into.
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = Text)]
        whole: String,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id || '|' || shop_id || '|' || user_id || '|' || token_hash || '|' \
         || created_at || '|' || last_seen_at || '|' || COALESCE(ended_at, '') AS whole \
         FROM sessions",
    )
    .load(&mut conn)
    .unwrap();
    for row in rows {
        assert!(
            !row.whole.contains(signed_in.token.expose()),
            "the token is in the row"
        );
    }
}

/// The wrong-credential path is `services::users`' and this asserts it is
/// still that one: one refusal, and no session opened on the way past.
#[test]
fn a_wrong_pin_opens_nothing_and_answers_the_one_refusal() {
    let (_dir, mut conn) = open_temp();
    let cashier = a_cashier(&mut conn, "Karim", "2580");

    let err = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "1357", noon()).unwrap_err();
    assert!(matches!(err, CoreError::AuthRefused), "{err}");
    assert!(every_stored_hash(&mut conn).is_empty());

    // A user id nobody answers to gets the same sentence, so a PIN pad is not
    // a way to count the staff.
    let unknown = sessions::sign_in_with_pin(&mut conn, SHOP, 9999, "2580", noon()).unwrap_err();
    assert!(matches!(unknown, CoreError::AuthRefused), "{unknown}");
    assert!(every_stored_hash(&mut conn).is_empty());
}

/// The lockout is `services::users`' too, and it still bites through a
/// sign-in: five wrong PINs and the sixth try is refused with the wait.
#[test]
fn the_lockout_still_bites_through_a_sign_in() {
    let (_dir, mut conn) = open_temp();
    let cashier = a_cashier(&mut conn, "Karim", "2580");
    for _ in 0..users::FAILURES_BEFORE_LOCKOUT {
        let _ = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "1357", noon());
    }
    // Even the right PIN, because the wait is on the person and not on the
    // guess.
    let err = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap_err();
    assert!(
        matches!(err, CoreError::LockedOut { retry_after_seconds } if retry_after_seconds > 0),
        "{err}"
    );
    assert!(every_stored_hash(&mut conn).is_empty());
}

/// A token nobody ever issued. The same `None` a dead session gets.
#[test]
fn a_token_nobody_issued_resolves_to_nobody() {
    let (_dir, mut conn) = open_temp();
    a_cashier(&mut conn, "Karim", "2580");
    for guess in ["", "not a token", &"0".repeat(64), &"f".repeat(64)] {
        assert!(sessions::resolve(&mut conn, SHOP, guess, noon())
            .unwrap()
            .is_none());
    }
}

/// The idle time, read off the shop's own setting rather than restated: a
/// session a minute under it counts and a minute over it does not.
#[test]
fn a_session_dies_a_settings_held_idle_time_after_its_last_request() {
    let (_dir, mut conn) = open_temp();
    let cashier = a_cashier(&mut conn, "Karim", "2580");
    let idle = preferences::session_idle(&mut conn, SHOP).unwrap();
    let signed_in = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();
    let token = signed_in.token.expose();

    assert!(
        sessions::resolve(&mut conn, SHOP, token, noon() + idle - Duration::seconds(1))
            .unwrap()
            .is_some(),
        "a session a second under the idle time was thrown out"
    );
}

/// The edge itself, and a session of its own for each case: `resolve` slides
/// `last_seen_at` every time it counts, so two edge checks against one session
/// would measure the second from the first. The shop's setting means "idle
/// this long and you are out", so the instant it names is already out, and
/// without this the comparison could be either way round unnoticed.
#[test]
fn the_idle_time_is_out_at_the_instant_it_names() {
    for (offset, counts) in [
        (-Duration::seconds(1), true),
        (Duration::zero(), false),
        (Duration::seconds(1), false),
    ] {
        let (_dir, mut conn) = open_temp();
        let cashier = a_cashier(&mut conn, "Karim", "2580");
        let idle = preferences::session_idle(&mut conn, SHOP).unwrap();
        let signed_in =
            sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();
        let token = signed_in.token.expose();

        assert_eq!(
            sessions::resolve(&mut conn, SHOP, token, noon() + idle + offset)
                .unwrap()
                .is_some(),
            counts,
            "at the idle time {offset} the session should count={counts}"
        );
    }
}

#[test]
fn a_session_idle_past_the_setting_is_nobody() {
    let (_dir, mut conn) = open_temp();
    let cashier = a_cashier(&mut conn, "Karim", "2580");
    preferences::set_session_idle(&mut conn, SHOP, OWNER, 20, noon()).unwrap();
    let signed_in = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();
    let token = signed_in.token.expose();

    assert!(sessions::resolve(&mut conn, SHOP, token, at(12, 19))
        .unwrap()
        .is_some());
    assert!(
        sessions::resolve(&mut conn, SHOP, token, at(12, 41))
            .unwrap()
            .is_none(),
        "a session idle past the shop's twenty minutes still counted"
    );
}

/// Sliding, not fixed: the idle time runs from the last request the session
/// carried, so a cashier working a queue is never thrown out mid-sale. Three
/// requests nineteen minutes apart, under a twenty-minute setting, and the
/// session is nearly an hour old and still alive.
#[test]
fn the_idle_time_runs_from_the_last_request_and_not_from_the_sign_in() {
    let (_dir, mut conn) = open_temp();
    let cashier = a_cashier(&mut conn, "Karim", "2580");
    preferences::set_session_idle(&mut conn, SHOP, OWNER, 20, noon()).unwrap();
    let signed_in = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();
    let token = signed_in.token.expose();

    for minutes in [19, 38, 57] {
        assert!(
            sessions::resolve(&mut conn, SHOP, token, noon() + Duration::minutes(minutes))
                .unwrap()
                .is_some(),
            "thrown out after {minutes} minutes of working"
        );
    }
    // And it still dies twenty-one minutes after the last one.
    assert!(
        sessions::resolve(&mut conn, SHOP, token, noon() + Duration::minutes(78))
            .unwrap()
            .is_none()
    );
}

/// `describe` is what `/auth/me` reads, and it does not keep a session alive:
/// a screen polling who is signed in would otherwise be a till that never
/// locks.
#[test]
fn reading_who_is_signed_in_does_not_itself_hold_the_session_open() {
    let (_dir, mut conn) = open_temp();
    let cashier = a_cashier(&mut conn, "Karim", "2580");
    preferences::set_session_idle(&mut conn, SHOP, OWNER, 20, noon()).unwrap();
    let signed_in = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();
    let token = signed_in.token.expose();

    for minutes in [5, 10, 15] {
        let (actor, name) =
            sessions::describe(&mut conn, SHOP, token, noon() + Duration::minutes(minutes))
                .unwrap()
                .expect("the session should still be there");
        assert_eq!(actor.user_id, cashier);
        assert_eq!(name, "Karim");
    }
    assert!(
        sessions::describe(&mut conn, SHOP, token, at(12, 21))
            .unwrap()
            .is_none(),
        "reading who was signed in kept the session alive"
    );
}

#[test]
fn signing_out_ends_the_session_and_signing_out_twice_is_not_an_error() {
    let (_dir, mut conn) = open_temp();
    let cashier = a_cashier(&mut conn, "Karim", "2580");
    let signed_in = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();
    let token = signed_in.token.expose().to_owned();

    sessions::sign_out(&mut conn, SHOP, &token, at(12, 5)).unwrap();
    assert!(sessions::resolve(&mut conn, SHOP, &token, at(12, 6))
        .unwrap()
        .is_none());
    sessions::sign_out(&mut conn, SHOP, &token, at(12, 7)).unwrap();
    // And a token that was never a session is not an error either.
    sessions::sign_out(&mut conn, SHOP, "not a token", at(12, 8)).unwrap();
}

/// One sign-in does not close another. The same person at the till and at the
/// office screen is two sessions, and signing out of one leaves the other.
#[test]
fn two_sessions_of_one_user_are_independent() {
    let (_dir, mut conn) = open_temp();
    let cashier = a_cashier(&mut conn, "Karim", "2580");
    let till = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();
    let office = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();
    assert_ne!(till.token.expose(), office.token.expose());

    sessions::sign_out(&mut conn, SHOP, till.token.expose(), at(12, 1)).unwrap();
    assert!(
        sessions::resolve(&mut conn, SHOP, till.token.expose(), at(12, 2))
            .unwrap()
            .is_none()
    );
    assert!(
        sessions::resolve(&mut conn, SHOP, office.token.expose(), at(12, 2))
            .unwrap()
            .is_some()
    );
}

/// A fiche switched off refuses the session it is holding, on the next
/// request and not when the idle time happens to run out. The row is kept,
/// because documents name it.
#[test]
fn a_user_switched_off_loses_the_session_they_were_holding() {
    let (_dir, mut conn) = open_temp();
    let cashier = a_cashier(&mut conn, "Karim", "2580");
    let signed_in = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();
    let token = signed_in.token.expose().to_owned();
    assert!(sessions::resolve(&mut conn, SHOP, &token, at(12, 1))
        .unwrap()
        .is_some());

    users::deactivate(&mut conn, SHOP, OWNER, cashier).unwrap();
    assert!(
        sessions::resolve(&mut conn, SHOP, &token, at(12, 2))
            .unwrap()
            .is_none(),
        "a switched-off user went on working on the session they already had"
    );
}

/// What T8 calls when it switches a fiche off, so the row says the session
/// ended rather than the join happening to refuse it.
#[test]
fn ending_every_session_of_a_user_closes_them_all_and_leaves_other_users_alone() {
    let (_dir, mut conn) = open_temp();
    let karim = a_cashier(&mut conn, "Karim", "2580");
    let amina = a_cashier(&mut conn, "Amina", "3690");
    let a = sessions::sign_in_with_pin(&mut conn, SHOP, karim, "2580", noon()).unwrap();
    let b = sessions::sign_in_with_pin(&mut conn, SHOP, karim, "2580", noon()).unwrap();
    let other = sessions::sign_in_with_pin(&mut conn, SHOP, amina, "3690", noon()).unwrap();

    assert_eq!(
        sessions::end_all_for_user(&mut conn, SHOP, karim, at(12, 5)).unwrap(),
        2
    );
    for dead in [&a, &b] {
        assert!(
            sessions::resolve(&mut conn, SHOP, dead.token.expose(), at(12, 6))
                .unwrap()
                .is_none()
        );
    }
    assert!(
        sessions::resolve(&mut conn, SHOP, other.token.expose(), at(12, 6))
            .unwrap()
            .is_some()
    );
    // Ending twice moves nothing.
    assert_eq!(
        sessions::end_all_for_user(&mut conn, SHOP, karim, at(12, 7)).unwrap(),
        0
    );
}

/// Resetting a PIN never ended the session it was meant to
/// kill. An owner who suspects a cashier's PIN was watched over their
/// shoulder resets it exactly so the till that cashier is standing at stops
/// working; before this, it went on answering until the idle timer ran out
/// on its own.
#[test]
fn resetting_a_pin_ends_every_session_the_fiche_was_holding() {
    let (_dir, mut conn) = open_temp();
    let karim = a_cashier(&mut conn, "Karim", "2580");
    let signed_in = sessions::sign_in_with_pin(&mut conn, SHOP, karim, "2580", noon()).unwrap();
    let token = signed_in.token.expose().to_owned();
    assert!(
        sessions::resolve(&mut conn, SHOP, &token, at(12, 1))
            .unwrap()
            .is_some(),
        "the session was not even open to begin with"
    );

    users::set_pin(&mut conn, SHOP, OWNER, karim, "3690", None).unwrap();

    assert!(
        sessions::resolve(&mut conn, SHOP, &token, at(12, 2))
            .unwrap()
            .is_none(),
        "a PIN reset left the session it was meant to kill standing"
    );
}

/// The same, on the other credential: a password reset is the same finding
/// (`services::users::set_password`'s own doc says why it is fixed
/// alongside the PIN even with no route calling it yet).
#[test]
fn resetting_a_password_ends_every_session_the_fiche_was_holding() {
    let (_dir, mut conn) = open_temp();
    users::set_password(&mut conn, SHOP, OWNER, OWNER, "correct horse", None).unwrap();
    let owner = users::get(&mut conn, SHOP, OWNER).unwrap();
    let signed_in =
        sessions::sign_in_with_password(&mut conn, SHOP, &owner.name, "correct horse", noon())
            .unwrap();
    let token = signed_in.token.expose().to_owned();

    users::set_password(&mut conn, SHOP, OWNER, OWNER, "battery staple", None).unwrap();

    assert!(
        sessions::resolve(&mut conn, SHOP, &token, at(12, 1))
            .unwrap()
            .is_none(),
        "a password reset left the session it was meant to kill standing"
    );
}

/// The one thing to get right about ending sessions on a credential reset:
/// the person doing the resetting may be resetting their own, and ending
/// every session they hold would sign them out of the screen they used to do
/// it. `set_pin`'s own doc is the decision; this is the test of it. Every
/// *other* session of that same fiche still ends, because the finding is
/// about a watched PIN, not about the screen mid-reset.
#[test]
fn resetting_your_own_pin_keeps_the_session_that_did_it_and_ends_the_others() {
    let (_dir, mut conn) = open_temp();
    users::set_pin(&mut conn, SHOP, OWNER, OWNER, "1590", None).unwrap();
    let acting = sessions::sign_in_with_pin(&mut conn, SHOP, OWNER, "1590", noon()).unwrap();
    let elsewhere = sessions::sign_in_with_pin(&mut conn, SHOP, OWNER, "1590", noon()).unwrap();

    users::set_pin(
        &mut conn,
        SHOP,
        OWNER,
        OWNER,
        "2680",
        Some(acting.actor.session_id),
    )
    .unwrap();

    assert!(
        sessions::resolve(&mut conn, SHOP, acting.token.expose(), at(12, 1))
            .unwrap()
            .is_some(),
        "the session that reset its own PIN was ended by the reset it made"
    );
    assert!(
        sessions::resolve(&mut conn, SHOP, elsewhere.token.expose(), at(12, 1))
            .unwrap()
            .is_none(),
        "a second session of the same owner survived a self reset"
    );
}

/// The gap the finding named beside the credential one: `deactivate` never
/// called `end_all_for_user` either, and it happened to be safe only because
/// the live join on `users.active` refused on its own — until the fiche is
/// switched back on inside the idle window. This proves the row and not the
/// join: `active` reads true again by the time this asserts, so a resolve
/// that still answered `None` can only be the session having actually ended.
#[test]
fn deactivating_and_reactivating_inside_the_idle_window_does_not_resurrect_the_old_token() {
    let (_dir, mut conn) = open_temp();
    let karim = a_cashier(&mut conn, "Karim", "2580");
    let signed_in = sessions::sign_in_with_pin(&mut conn, SHOP, karim, "2580", noon()).unwrap();
    let token = signed_in.token.expose().to_owned();

    users::deactivate(&mut conn, SHOP, OWNER, karim).unwrap();
    let reactivated = users::reactivate(&mut conn, SHOP, OWNER, karim).unwrap();
    assert!(
        reactivated.active,
        "reactivate did not turn the fiche back on"
    );

    assert!(
        sessions::resolve(&mut conn, SHOP, &token, at(12, 1))
            .unwrap()
            .is_none(),
        "a fiche switched off and back on inside the idle window handed the \
         old token back with no new sign-in"
    );
}

/// Rule 3: a session of one shop is nobody in another. The token is the same
/// string and the answer is still `None`.
#[test]
fn a_session_of_one_shop_is_nobody_in_another() {
    let (_dir, mut conn) = open_temp();
    let cashier = a_cashier(&mut conn, "Karim", "2580");
    let signed_in = sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();
    assert!(
        sessions::resolve(&mut conn, SHOP + 1, signed_in.token.expose(), noon())
            .unwrap()
            .is_none()
    );
}

/// Rows nothing will read again go on the way into a sign-in, so the table
/// does not grow forever on a till that signs in twice a day. A row still
/// inside the keep window stays.
#[test]
fn a_sign_in_sweeps_rows_that_went_cold_a_fortnight_ago() {
    let (_dir, mut conn) = open_temp();
    let cashier = a_cashier(&mut conn, "Karim", "2580");
    sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();
    // The stored hashes name the rows; the tokens themselves are no use here
    // because what is being watched is which row survives.
    let cold = every_stored_hash(&mut conn);
    assert_eq!(cold.len(), 1);

    // Ten days on: nothing is a fortnight cold yet, so both rows stand.
    sessions::sign_in_with_pin(
        &mut conn,
        SHOP,
        cashier,
        "2580",
        noon() + Duration::days(10),
    )
    .unwrap();
    let two = every_stored_hash(&mut conn);
    assert_eq!(two.len(), 2, "a row nothing had finished with was swept");
    let warm = two
        .iter()
        .find(|h| **h != cold[0])
        .cloned()
        .expect("the second sign-in wrote no row");

    // Twenty days on: the first row has been untouched for twenty days and
    // goes; the second for ten and stays.
    sessions::sign_in_with_pin(
        &mut conn,
        SHOP,
        cashier,
        "2580",
        noon() + Duration::days(20),
    )
    .unwrap();
    let left = every_stored_hash(&mut conn);
    assert_eq!(
        left.len(),
        2,
        "the cold row was not swept, or the warm one was"
    );
    assert!(
        !left.contains(&cold[0]),
        "the fortnight-cold row is still there"
    );
    assert!(
        left.contains(&warm),
        "the ten-day-old row was swept with it"
    );
}

/// The fortnight is a fortnight and not "somewhere between ten and twenty
/// days": a day under it stays, a day over it goes. The test above proves the
/// sweep happens; this one pins the number it happens at.
#[test]
fn the_sweep_waits_exactly_the_fortnight_the_constant_names() {
    let keep = Duration::days(sessions::KEEP_ENDED_FOR_DAYS);

    for (age, survives) in [
        (keep - Duration::days(1), true),
        (keep + Duration::days(1), false),
    ] {
        let (_dir, mut conn) = open_temp();
        let cashier = a_cashier(&mut conn, "Karim", "2580");
        sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon()).unwrap();
        let first = every_stored_hash(&mut conn);
        assert_eq!(first.len(), 1);

        sessions::sign_in_with_pin(&mut conn, SHOP, cashier, "2580", noon() + age).unwrap();
        let left = every_stored_hash(&mut conn);
        assert_eq!(
            left.contains(&first[0]),
            survives,
            "a row {age} old: expected it to survive={survives}"
        );
    }
}
