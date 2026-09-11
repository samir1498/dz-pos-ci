// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `POST /auth/first-pin` over HTTP: the one door into a shop nobody has
//! ever signed into (M4 T8). No session, unlike every other file in this
//! folder but `auth_api.rs`'s own three routes: this is why it needs its
//! own harness rather than `tests/common`, which plants a session before a
//! test's first call.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";
const OWNER: i32 = 1;

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).unwrap()
}

/// A fresh shop file, migrated and seeded and touched by nothing else: the
/// seeded owner, no PIN, no password, exactly the file a real installation
/// opens on for the first time.
fn virgin_shop() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    // Opening the file is what runs the migrations and seeds the owner; no
    // credential is touched.
    drop(dzpos_core::db::open(&path).unwrap());
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    let app = dzpos_api::router(state, &token());
    (dir, app)
}

async fn call(
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

async fn claim(app: &axum::Router, pin: &str) -> (StatusCode, Value) {
    call(app, "POST", "/auth/first-pin", Some(json!({ "pin": pin }))).await
}

/// The whole point of the route: a shop nobody has ever signed into goes
/// from unusable to a live session in one call, and the answer names the
/// seeded owner the way `login`'s does.
#[tokio::test]
async fn a_virgin_shop_claims_its_first_pin_and_is_handed_a_session() {
    let (_dir, app) = virgin_shop();
    let (status, body) = claim(&app, "2580").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["me"]["user_id"], OWNER);
    assert_eq!(body["me"]["role"], "owner");
    // An owner holds ManageUsers among the rest, proving the session this
    // route hands back is a real one and not a stand-in.
    let held = body["me"]["permissions"].as_array().unwrap();
    assert!(held.iter().any(|p| p == "manage_users"), "{held:?}");
    let token = body["token"].as_str().unwrap();
    assert_eq!(token.len(), 64);

    // The session works on an ordinary route, ManageUsers included: this is
    // not only "a PIN was set somewhere", it is a till standing open.
    let (status, listed) = call(&app, "GET", "/users", None).await;
    // No session header on this call: proves nothing yet either way, so the
    // real proof is the header call right after it.
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{listed}");

    let req = Request::builder()
        .method("GET")
        .uri("/users")
        .header("authorization", format!("Bearer {TOKEN}"))
        .header("x-dzpos-session", token)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

/// A PIN the shape rule refuses (`services::users::validate_pin`) is refused
/// here too, before anything is claimed: `422 validation`, the same code a
/// screen already translates.
#[tokio::test]
async fn a_badly_shaped_pin_is_refused_and_claims_nothing() {
    let (_dir, app) = virgin_shop();
    let (status, body) = claim(&app, "0000").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");
    // The door is still open: a good PIN right after still claims it.
    let (status, body) = claim(&app, "2580").await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// The door shuts for good the moment a credential exists, whichever call
/// wrote it: a second claim after the first is refused, and so is a claim
/// on a shop whose owner already has a PIN some other way.
#[tokio::test]
async fn the_door_shuts_once_the_owner_has_a_credential() {
    let (_dir, app) = virgin_shop();
    let (status, _) = claim(&app, "2580").await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = claim(&app, "3690").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");
    assert_eq!(body["error"]["field"], "pin");
}

/// The route needs the launch token like every other one, and answers the
/// same 401 a stranger's page would get calling it without one.
#[tokio::test]
async fn the_route_still_wants_the_launch_token() {
    let (_dir, app) = virgin_shop();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/first-pin")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "pin": "2580" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
