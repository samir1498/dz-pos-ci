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
