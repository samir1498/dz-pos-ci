// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The seeder is a development tool, and the first of the three places that
//! is enforced is this one: nothing the shop installs can contain it.
//!
//! The enforcement is a package boundary. `dzpos-seed` is its own crate, and
//! neither `dzpos-api` nor the desktop depends on it, so a release build of
//! either cannot produce the binary however the build is invoked. A boundary
//! nobody checks is a boundary somebody adds a dependency across on a Tuesday,
//! which is what this file is for: it fails the moment the API grows a seed
//! binary, a seed route, or a line in a manifest naming the seeder.
//!
//! The other two places, named here so a reader of this file knows where to
//! look: the guards in `crates/seed/src/main.rs` (`DZPOS_DEV=1`, a file
//! directly inside `.dev/`, and a refusal to write over a shop that carries
//! its own identifiers), and the `just seed` and `just seed-clean` recipes,
//! which take no path at all.

use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

const TOKEN: &str = "test-launch-token";

/// The repository root, from this crate's own directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

#[test]
fn the_api_crate_has_no_second_binary_and_names_the_seeder_nowhere() {
    // A `src/bin/` directory in this crate is a binary the API's own build
    // produces, which is how the seeder shipped for about an hour before this
    // test existed. It also breaks `cargo run -p dzpos-api`, which is what
    // `just api` and the e2e harness both run.
    assert!(
        !root().join("crates/api/src/bin").exists(),
        "crates/api/src/bin exists: a second binary here ships with the API and breaks `cargo run -p dzpos-api`"
    );

    for manifest in [
        "crates/api/Cargo.toml",
        "apps/desktop/src-tauri/Cargo.toml",
        "crates/core/Cargo.toml",
    ] {
        let text = std::fs::read_to_string(root().join(manifest)).unwrap();
        assert!(
            !text.contains("dzpos-seed"),
            "{manifest} depends on the seeder, which puts it in what the shop installs"
        );
    }

    // And the bundle ships no sidecar binary at all: `externalBin` is the
    // Tauri key that copies one in beside the app.
    let tauri =
        std::fs::read_to_string(root().join("apps/desktop/src-tauri/tauri.conf.json")).unwrap();
    assert!(
        !tauri.contains("externalBin"),
        "the desktop bundle carries a sidecar binary; check it is not the seeder"
    );
}

#[test]
fn the_api_source_reaches_the_seeder_nowhere_and_routes_nothing_to_it() {
    // Not a search for the word: `SEEDED_OWNER_USER_ID` is the owner the
    // first migration writes, which has nothing to do with this and would
    // make the check a thing people learn to work around. What is looked for
    // is a way in: the crate by name, and a route or a flag spelled `seed`.
    for file in ["crates/api/src/main.rs", "crates/api/src/lib.rs"] {
        let text = std::fs::read_to_string(root().join(file)).unwrap();
        for forbidden in [
            "dzpos_seed",
            "dzpos-seed",
            "services::seed",
            "\"seed\"",
            "/seed",
        ] {
            assert!(
                !text.contains(forbidden),
                "{file} contains {forbidden}: the API has no seed flag, no seed route and no path to the seeder"
            );
        }
    }
}

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
