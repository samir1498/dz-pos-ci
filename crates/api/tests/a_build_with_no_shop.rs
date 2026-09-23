// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! S7 of `a-kernel-crate-and-retail-as-the-first-module`: the proof that
//! `dzpos-api` built with `--no-default-features` (no `retail`) is a working
//! program. This file's body compiles in both feature sets, so the same
//! test walks the same door whether or not the shop is built in; only the
//! last assertion, that a shop route is not there to answer, is retail-off
//! only.
//!
//! Real temp SQLite file, real router, over HTTP, the way
//! `tests/first_setup_api.rs` and `tests/auth_api.rs` already do: opening
//! the file is what runs every migration in the one folder the kernel owns
//! (S7's accepted cost — a kernel build makes every trade's tables and uses
//! a tenth of them), first setup claims the seeded owner and hands back a
//! session, and that session opens a kernel route.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).expect("a fixed secret is always a valid token")
}

/// A fresh shop file, migrated and seeded and touched by nothing else: the
/// seeded owner, no PIN, no password, exactly the file a real installation
/// opens on for the first time. Opening it is what runs the migrations.
fn virgin_shop() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().expect("a temp dir is always made");
    let path = dir.path().join("t.db");
    drop(dzpos_core::db::open(&path).expect("a fresh file always opens and migrates"));
    let state = dzpos_api::AppState::open(&path, SHOP)
        .expect("the freshly migrated file opens for the api");
    let app = dzpos_api::router(state, &token());
    (dir, app)
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    session: Option<&str>,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"));
    if let Some(session) = session {
        req = req.header("x-dzpos-session", session);
    }
    let req = match body {
        Some(v) => req
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .expect("a JSON body always builds a request"),
        None => req
            .body(Body::empty())
            .expect("an empty body always builds a request"),
    };
    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("the router always answers a request it was given");
    let status = res.status();
    let bytes = res
        .into_body()
        .collect()
        .await
        .expect("axum always drains its own response body")
        .to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

/// The whole claim of S7: a fresh file opens and migrates, first setup
/// claims the seeded owner over HTTP, and the session it hands back opens a
/// kernel route — a build with no shop in it still signs a user in.
#[tokio::test]
async fn a_kernel_only_build_opens_migrates_and_signs_a_user_in() {
    let (_dir, app) = virgin_shop();

    let (status, health) = call(&app, "GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK, "{health}");
    assert_eq!(health["needs_first_setup"], true);

    let (status, claimed) = call(
        &app,
        "POST",
        "/auth/first-setup",
        Some(json!({ "name": "Anouar", "password": "huit caracteres" })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{claimed}");
    assert_eq!(claimed["me"]["role"], "owner");
    let session = claimed["token"]
        .as_str()
        .expect("first setup hands back a session token");

    // A kernel route, session-guarded, answers for the owner the claim just
    // signed in: not only "a credential was set somewhere", a till standing
    // open under it.
    let (status, users) = call(&app, "GET", "/users", None, Some(session)).await;
    assert_eq!(status, StatusCode::OK, "{users}");
    let names: Vec<&str> = users
        .as_array()
        .expect("the users route answers a list")
        .iter()
        .map(|u| u["name"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(names, ["Anouar"]);

    let (status, settings) = call(&app, "GET", "/settings", None, Some(session)).await;
    assert_eq!(status, StatusCode::OK, "{settings}");
}

/// Retail off only: a shop route is not there to answer at all, before the
/// session is even looked up (`router.rs` merges the shop routes behind the
/// feature and falls through to `not_found` otherwise), which is the other
/// half of the split's claim — the kernel is a program without a shop.
#[cfg(not(feature = "retail"))]
#[tokio::test]
async fn a_shop_route_does_not_exist_in_a_kernel_only_build() {
    let (_dir, app) = virgin_shop();
    let (status, body) = call(&app, "GET", "/products", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(body["error"]["code"], "not_found", "{body}");

    let (status, body) = call(&app, "GET", "/sales", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(body["error"]["code"], "not_found", "{body}");
}
