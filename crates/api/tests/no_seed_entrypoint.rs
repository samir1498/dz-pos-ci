// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The API routes nothing to the seeder, whatever a caller spells.
//!
//! The seeder is a development tool, and that it cannot reach a shop's books
//! is enforced in three places. This file holds one half of the first: the
//! API answers no seed route. The other half, that no build of the API or of
//! the desktop can produce the seeder at all, is a fact about the workspace's
//! own layout and is held in `crates/seed/src/main.rs`, which is the crate
//! that ships nowhere and is therefore the honest place for it.
//!
//! The other two: the guards in `crates/seed/src/main.rs` (`DZPOS_DEV=1`, a
//! file directly inside `.dev/`, and a refusal to write over a shop that
//! carries its own identifiers), and the `just seed` and `just seed-clean`
//! recipes, which take no path at all.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

const TOKEN: &str = "test-launch-token";

#[tokio::test]
async fn every_spelling_of_a_seed_route_is_no_route_at_all() {
    let dir = tempfile::tempdir().unwrap();
    let state = dzpos_api::AppState::open(dir.path().join("t.db"), 1).unwrap();
    let app = dzpos_api::router(state, &dzpos_api::LaunchToken::from_secret(TOKEN).unwrap());

    for (method, uri) in [
        ("GET", "/seed"),
        ("POST", "/seed"),
        ("POST", "/dev/seed"),
        ("POST", "/shops/1/seed"),
    ] {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", format!("Bearer {TOKEN}"))
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::NOT_FOUND,
            "{method} {uri} is answered by something"
        );
    }
}
