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
        Err(e) => panic!("the test built a request that will not build: {e}"),
    }
}

#[test]
fn the_phones_header_is_read() {
    assert_eq!(
        device_token(with(&[(DEVICE_HEADER, "abc123")]).headers()).as_deref(),
        Some("abc123")
    );
    assert_eq!(
        device_token(with(&[(DEVICE_HEADER, "   ")]).headers()),
        None
    );
    assert_eq!(device_token(with(&[]).headers()), None);
    assert_eq!(
        device_token(with(&[(DEVICE_HEADER, "  abc123  ")]).headers()).as_deref(),
        Some("abc123")
    );
}
