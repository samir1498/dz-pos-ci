use super::LaunchToken;

#[test]
fn a_generated_token_is_64_hex_characters_and_never_the_same_twice() {
    let a = LaunchToken::generate().map_err(|e| e.to_string());
    let b = LaunchToken::generate().map_err(|e| e.to_string());
    let (a, b) = match (a, b) {
        (Ok(a), Ok(b)) => (a, b),
        (a, b) => panic!("{a:?} {b:?}"),
    };
    assert_eq!(a.expose().len(), 64);
    assert!(a.expose().bytes().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(a.expose(), b.expose());
    assert!(a.matches(a.expose()) && !a.matches(b.expose()));
}

#[test]
fn a_chosen_secret_is_checked_before_the_server_binds() {
    assert!(LaunchToken::from_secret("").is_err());
    assert!(LaunchToken::from_secret("short").is_err());
    assert!(LaunchToken::from_secret(" sixteen-chars-x ").is_err());
    assert!(LaunchToken::from_secret("sixteen\tchars-x-").is_err());
    assert!(LaunchToken::from_secret("sixteen-chars-ok").is_ok());
}

#[test]
fn debug_output_keeps_the_secret() {
    let t = LaunchToken::from_secret("sixteen-chars-ok").map_err(|e| e.to_string());
    assert_eq!(format!("{t:?}"), "Ok(LaunchToken(..))");
}
