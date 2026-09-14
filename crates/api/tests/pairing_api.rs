#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).unwrap()
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    session: &str,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, session);
    let req = match body {
        Some(v) => req
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
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

async fn call_no_session(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"));
    let req = match body {
        Some(v) => req
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
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

fn app() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    common::sign_in_as(&path, SHOP, "cashier", common::CASHIER_SESSION);
    common::sign_in_as(&path, SHOP, "manager", common::MANAGER_SESSION);
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    (dir, dzpos_api::router(state, &token()))
}

#[tokio::test]
async fn an_owner_can_create_a_pairing_qr_and_a_phone_can_claim_it_once() {
    let (_dir, app) = app();
    let (status, body) = call(&app, "POST", "/pairing/qr", None, common::OWNER_SESSION).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let pairing_token = body["pairing_token"].as_str().unwrap();
    assert_eq!(pairing_token.len(), 64);
    assert_eq!(body["expires_in_seconds"], 60);

    // Phone claims with pairing token, no session, just launch token.
    let (status, body) = call_no_session(
        &app,
        "POST",
        "/pairing/claim",
        Some(json!({ "pairing_token": pairing_token, "device_name": "Phone" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["device_token"].as_str().unwrap().len(), 64);

    // Same pairing token cannot be claimed again.
    let (status, body) = call_no_session(
        &app,
        "POST",
        "/pairing/claim",
        Some(json!({ "pairing_token": pairing_token, "device_name": "Phone2" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
}

#[tokio::test]
async fn a_cashier_cannot_create_a_pairing_qr() {
    let (_dir, app) = app();
    let (status, body) = call(&app, "POST", "/pairing/qr", None, common::CASHIER_SESSION).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"]["code"], "forbidden");
}

#[tokio::test]
async fn a_manager_can_create_a_pairing_qr() {
    let (_dir, app) = app();
    let (status, body) = call(&app, "POST", "/pairing/qr", None, common::MANAGER_SESSION).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
async fn a_pairing_qr_needs_a_session() {
    let (_dir, app) = app();
    let (status, body) = call_no_session(&app, "POST", "/pairing/qr", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
}
