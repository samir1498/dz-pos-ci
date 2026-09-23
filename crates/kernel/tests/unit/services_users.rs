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
