#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
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

/// A call from the shop LAN: same router, but the request carries the peer
/// address a real serve puts there (`into_make_service_with_connect_info`),
/// so the device gate treats it as a phone and not the desktop.
async fn call_lan(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    session: Option<&str>,
    device: Option<&str>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    req = req.header("authorization", format!("Bearer {TOKEN}"));
    if let Some(session) = session {
        req = req.header(common::SESSION_HEADER, session);
    }
    if let Some(device) = device {
        req = req.header("x-dzpos-device", device);
    }
    let req = match body {
        Some(v) => req
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let mut req = req;
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([192, 168, 1, 7], 4317))));
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

/// Owner shows a QR, a phone claims it: the device token and row id the
/// LAN tests below exercise the gate with.
async fn pair_phone(app: &axum::Router) -> (String, i64) {
    let (status, body) = call(app, "POST", "/pairing/qr", None, common::OWNER_SESSION).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let pairing_token = body["pairing_token"].as_str().unwrap();
    let (status, body) = call_no_session(
        app,
        "POST",
        "/pairing/claim",
        Some(json!({ "pairing_token": pairing_token, "device_name": "Phone" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let device_token = body["device_token"].as_str().unwrap().to_string();
    let (status, body) = call(app, "GET", "/pairing/devices", None, common::OWNER_SESSION).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body.as_array().unwrap().len(), 1);
    let device_id = body[0]["id"].as_i64().unwrap();
    (device_token, device_id)
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

#[tokio::test]
async fn paired_devices_are_listed_and_revocable_by_an_owner() {
    let (_dir, app) = app();
    // Owner creates QR, phone claims.
    let (status, body) = call(&app, "POST", "/pairing/qr", None, common::OWNER_SESSION).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let pairing_token = body["pairing_token"].as_str().unwrap().to_string();
    let (status, body) = call_no_session(
        &app,
        "POST",
        "/pairing/claim",
        Some(json!({ "pairing_token": pairing_token, "device_name": "Phone" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    // List shows the phone, newest first, with no token hash.
    let (status, body) = call(&app, "GET", "/pairing/devices", None, common::OWNER_SESSION).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let list = body.as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["name"], "Phone");
    assert!(list[0]["revoked_at"].is_null());
    let device_id = list[0]["id"].as_i64().unwrap();

    // Revoke.
    let (status, body) = call(
        &app,
        "POST",
        &format!("/pairing/devices/{device_id}/revoke"),
        None,
        common::OWNER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(!body["revoked_at"].is_null());

    // Second revoke is 422 (already revoked).
    let (status, body) = call(
        &app,
        "POST",
        &format!("/pairing/devices/{device_id}/revoke"),
        None,
        common::OWNER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
}

#[tokio::test]
async fn a_phone_off_loopback_without_a_device_is_session_required() {
    let (_dir, app) = app();
    // No credential shown at all: the same 401 a missing session gets, not
    // a device-shaped refusal about a token nobody sent.
    let (status, body) = call_lan(&app, "GET", "/products", None, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "session_required");
}

#[tokio::test]
async fn a_signed_in_caller_without_a_device_is_refused_off_loopback() {
    // The diverging case the no-device test cannot see: the session would
    // pass, so only the device layer can be refusing here.
    let (_dir, app) = app();
    let (status, body) = call_lan(
        &app,
        "GET",
        "/products",
        None,
        Some(common::OWNER_SESSION),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "session_required");
}

#[tokio::test]
async fn an_unknown_device_off_loopback_is_refused() {
    let (_dir, app) = app();
    let (status, body) = call_lan(
        &app,
        "GET",
        "/products",
        None,
        Some(common::OWNER_SESSION),
        Some(&"0".repeat(64)),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "auth_refused");
}

#[tokio::test]
async fn a_live_device_off_loopback_reaches_the_route() {
    let (_dir, app) = app();
    let (device_token, _) = pair_phone(&app).await;
    let (status, body) = call_lan(
        &app,
        "GET",
        "/pairing/devices",
        None,
        Some(common::OWNER_SESSION),
        Some(&device_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn a_live_device_without_a_session_is_still_session_required() {
    // The device passed and the session refused: which phone is settled
    // before which person, and a paired phone is not a signed-in person.
    let (_dir, app) = app();
    let (device_token, _) = pair_phone(&app).await;
    let (status, body) = call_lan(&app, "GET", "/products", None, None, Some(&device_token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "session_required");
}

#[tokio::test]
async fn a_revoked_device_is_refused_before_any_session() {
    // No session on the call on purpose: auth_refused (not session_required)
    // proves the device layer runs before the session layer.
    let (_dir, app) = app();
    let (device_token, device_id) = pair_phone(&app).await;
    let (status, body) = call(
        &app,
        "POST",
        &format!("/pairing/devices/{device_id}/revoke"),
        None,
        common::OWNER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = call_lan(&app, "GET", "/products", None, None, Some(&device_token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "auth_refused");
}

#[tokio::test]
async fn loopback_ignores_a_bogus_device_header() {
    // The desktop shows no device token; a stray header on loopback must not
    // lock the till out. No ConnectInfo here, like every other test drive
    // of this router.
    let (_dir, app) = app();
    let req = Request::builder()
        .method("GET")
        .uri("/products")
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, common::OWNER_SESSION)
        .header("x-dzpos-device", "bogus")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn loopback_with_an_address_is_still_the_desktop() {
    // The contrast the harness-bypass test cannot see: both serve sites
    // install ConnectInfo, so production loopback always carries one, and
    // the gate must read the address, not the header's presence.
    let (_dir, app) = app();
    let mut req = Request::builder()
        .method("GET")
        .uri("/products")
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, common::OWNER_SESSION)
        .header("x-dzpos-device", "bogus")
        .body(Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 4317))));
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn a_claim_without_a_device_name_is_refused() {
    let (_dir, app) = app();
    let (status, body) = call(&app, "POST", "/pairing/qr", None, common::OWNER_SESSION).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let pairing_token = body["pairing_token"].as_str().unwrap();
    for name in ["", "   "] {
        let (status, body) = call_no_session(
            &app,
            "POST",
            "/pairing/claim",
            Some(json!({ "pairing_token": pairing_token, "device_name": name })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["error"]["code"], "validation");
    }
}

#[tokio::test]
async fn a_device_from_another_shop_is_refused() {
    let (dir, app) = app();
    let (device_token, _) = pair_phone(&app).await;
    let db = dir.path().join("t.db");
    let other = common::signed_in_router(&db, 2, &token());
    // Same token, other shop's router, other shop's owner session: the
    // lookup is scoped by shop, so there is no device here to find.
    let (status, body) = call_lan(
        &other,
        "GET",
        "/products",
        None,
        Some(common::OWNER_SESSION),
        Some(&device_token),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "auth_refused");
}

/// A PIN on the cashier, set directly on the test file: the phone signs a
/// person in with the same credentials as the till, so the test does what
/// a shop does (owner sets the PIN in settings) rather than what the API
/// cannot do for itself.
fn pin_cashier(db: &std::path::Path) -> i32 {
    use diesel::prelude::*;
    use diesel::sql_types::Integer;
    #[derive(diesel::QueryableByName)]
    struct Id {
        #[diesel(sql_type = Integer)]
        id: i32,
    }
    let mut conn = dzpos_core::db::open(db).unwrap();
    let owner: Id = diesel::sql_query(
        "SELECT id FROM users WHERE shop_id = ? AND role = 'owner' AND active = 1 \
         ORDER BY id LIMIT 1",
    )
    .bind::<Integer, _>(SHOP)
    .get_result(&mut conn)
    .unwrap();
    let cashier: Id = diesel::sql_query(
        "SELECT id FROM users WHERE shop_id = ? AND role = 'cashier' AND active = 1 \
         ORDER BY id LIMIT 1",
    )
    .bind::<Integer, _>(SHOP)
    .get_result(&mut conn)
    .unwrap();
    dzpos_core::services::users::set_pin(&mut conn, SHOP, owner.id, cashier.id, "2580", None)
        .unwrap();
    cashier.id
}

#[tokio::test]
async fn a_phone_signs_a_person_in_with_pin_and_device() {
    let (dir, app) = app();
    let (device_token, _) = pair_phone(&app).await;
    let cashier = pin_cashier(&dir.path().join("t.db"));
    // No session yet, but a paired device: the sign-in answers a session.
    let (status, body) = call_lan(
        &app,
        "POST",
        "/auth/login",
        Some(json!({ "user_id": cashier, "pin": "2580" })),
        None,
        Some(&device_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let session = body["token"].as_str().unwrap().to_string();
    assert_eq!(body["me"]["role"], "cashier");
    // And that session acts, with the device alongside: the phone sells
    // through the same gates as the desktop.
    let (status, body) = call_lan(
        &app,
        "GET",
        "/products",
        None,
        Some(&session),
        Some(&device_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
async fn a_phone_cannot_sign_in_without_its_device() {
    let (dir, app) = app();
    pair_phone(&app).await;
    let cashier = pin_cashier(&dir.path().join("t.db"));
    // Right PIN, no device: the launch token alone mints no sessions.
    let (status, body) = call_lan(
        &app,
        "POST",
        "/auth/login",
        Some(json!({ "user_id": cashier, "pin": "2580" })),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "session_required");
}

#[tokio::test]
async fn a_cashier_on_a_phone_is_refused_by_name_like_on_the_desktop() {
    // The permission gates do not know or care which glass the call came
    // through: a cashier's phone is refused the manager routes with the
    // permission named, like the cashier specs prove for the desktop.
    let (dir, app) = app();
    let (device_token, _) = pair_phone(&app).await;
    let cashier = pin_cashier(&dir.path().join("t.db"));
    let (status, body) = call_lan(
        &app,
        "POST",
        "/auth/login",
        Some(json!({ "user_id": cashier, "pin": "2580" })),
        None,
        Some(&device_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let session = body["token"].as_str().unwrap();
    let (status, body) = call_lan(
        &app,
        "POST",
        "/pairing/qr",
        None,
        Some(session),
        Some(&device_token),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"]["code"], "forbidden");
    assert_eq!(body["error"]["permission"], "edit_settings");
}

#[tokio::test]
async fn a_cashier_cannot_list_or_revoke_devices() {
    let (_dir, app) = app();
    let (status, body) = call(
        &app,
        "GET",
        "/pairing/devices",
        None,
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let (status, body) = call(
        &app,
        "POST",
        "/pairing/devices/1/revoke",
        None,
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
}
