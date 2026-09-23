// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::body::Body;

fn with(headers: &[(&str, &str)]) -> Request {
    let mut req = Request::builder().uri("/");
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    match req.body(Body::empty()) {
        Ok(req) => req,
        // Every header this file's tests build is a valid one, so this
        // arm is unreachable; it is written out because the workspace
        // denies `unwrap` even here.
        Err(e) => panic!("the test built a request that will not build: {e}"),
    }
}

#[test]
fn the_desktops_header_is_read() {
    assert_eq!(
        token_of(&with(&[(SESSION_HEADER, "abc123")])).as_deref(),
        Some("abc123")
    );
    assert_eq!(token_of(&with(&[(SESSION_HEADER, "   ")])), None);
    assert_eq!(token_of(&with(&[])), None);
}

/// The browser's cookie, picked out of whatever else the page is
/// carrying, in either order and with the spacing a browser actually
/// sends.
#[test]
fn the_browsers_cookie_is_read_out_of_the_jar() {
    for jar in [
        "dzpos_session=abc123",
        "other=1; dzpos_session=abc123",
        "dzpos_session=abc123; other=1",
        "a=1;dzpos_session=abc123;b=2",
    ] {
        assert_eq!(
            token_of(&with(&[("cookie", jar)])).as_deref(),
            Some("abc123"),
            "{jar}"
        );
    }
    // A jar with no cookie of ours, and one whose name only looks like
    // ours.
    for jar in ["other=1", "xdzpos_session=abc123", "dzpos_sessionx=abc"] {
        assert_eq!(token_of(&with(&[("cookie", jar)])), None, "{jar}");
    }
}

/// A stale cookie in a webview must not quietly decide who a request came
/// from when the desktop said who it was.
#[test]
fn the_header_wins_over_a_cookie() {
    let req = with(&[
        (SESSION_HEADER, "from-the-desktop"),
        ("cookie", "dzpos_session=stale"),
    ]);
    assert_eq!(token_of(&req).as_deref(), Some("from-the-desktop"));
}

/// The three flags the cookie is worth setting for, and the one it must
/// not carry on a plain-HTTP loopback origin.
#[test]
fn the_cookie_is_http_only_same_site_and_not_secure() {
    let set = set_cookie("abc123", 15).expect("a hex token makes a header value");
    let text = set.to_str().unwrap_or_default();
    assert!(text.contains("dzpos_session=abc123"), "{text}");
    assert!(text.contains("HttpOnly"), "{text}");
    assert!(text.contains("SameSite=Lax"), "{text}");
    assert!(text.contains("Path=/"), "{text}");
    assert!(text.contains("Max-Age=900"), "{text}");
    assert!(
        !text.contains("Secure"),
        "a Secure cookie on a plain-HTTP loopback origin is one the browser drops: {text}"
    );

    let cleared = clear_cookie().to_str().unwrap_or_default().to_owned();
    assert!(cleared.contains("Max-Age=0"), "{cleared}");
    assert!(cleared.contains("HttpOnly"), "{cleared}");
}
