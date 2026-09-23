// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::allowed_navigation;
use tauri::Url;

fn url(s: &str) -> Url {
    Url::parse(s).expect("test URL must parse")
}

#[test]
fn the_apps_own_origins_are_allowed() {
    assert!(allowed_navigation(&url("tauri://localhost/")));
    assert!(allowed_navigation(&url("tauri://localhost/settings")));
    assert!(allowed_navigation(&url("http://tauri.localhost/")));
}

#[test]
fn a_remote_origin_is_refused() {
    assert!(!allowed_navigation(&url("https://evil.example/")));
    assert!(!allowed_navigation(&url("http://evil.example/")));
}

#[test]
fn the_apis_own_loopback_origin_is_refused() {
    // No screen navigates the main frame there today (every call is a
    // `fetch`); this is what keeps a future one from working.
    assert!(!allowed_navigation(&url("http://127.0.0.1:4317/")));
}

#[test]
fn a_look_alike_host_is_refused() {
    // `.localhost` as a suffix, not an exact host, would let
    // `tauri.localhost.evil.example` through.
    assert!(!allowed_navigation(&url(
        "http://tauri.localhost.evil.example/"
    )));
}
