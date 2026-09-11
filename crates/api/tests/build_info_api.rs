// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `GET /build-info` (M5 T1) and the log file `AppState::open` heads with
//! the same three values. In-process router, real temp SQLite file.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

mod common;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).unwrap()
}

async fn call(app: &axum::Router, method: &str, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, common::OWNER_SESSION)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

/// The route answers the same three fields `dzpos_core::build_info` carries,
/// which is the whole point: one source, read back rather than reassembled.
#[tokio::test]
async fn build_info_answers_the_same_values_the_binary_was_built_with() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    let app = dzpos_api::router(state, &token());

    let (status, body) = call(&app, "GET", "/build-info").await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let info = dzpos_core::build_info::BUILD_INFO;
    assert_eq!(body["version"], info.version);
    assert_eq!(body["git_hash"], info.git_hash);
    assert_eq!(body["build_date"], info.build_date);
    assert_eq!(body["debug"], info.is_debug);
}

/// A session is required the same as every other route past the launch
/// token, `/health` and the three auth routes aside.
#[tokio::test]
async fn build_info_refuses_a_caller_with_no_session() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    let app = dzpos_api::router(state, &token());

    let req = Request::builder()
        .method("GET")
        .uri("/build-info")
        .header("authorization", format!("Bearer {TOKEN}"))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// `AppState::open` is what both the standalone binary and the desktop
/// process call to start a session; this proves the log file it writes to
/// carries the header line, without needing either binary running.
#[tokio::test]
async fn opening_the_shop_file_heads_the_log_with_the_build_info_line() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    drop(state);

    let log = dir.path().join("dzpos.log");
    let written = std::fs::read_to_string(&log).expect("AppState::open should have headed a log");
    let info = dzpos_core::build_info::BUILD_INFO;
    assert!(written.contains(info.version), "{written}");
    assert!(written.contains(info.git_hash), "{written}");
    assert!(written.contains(info.build_date), "{written}");
    assert!(written.contains("session started"), "{written}");
}
