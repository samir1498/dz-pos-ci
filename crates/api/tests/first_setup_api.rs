// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `POST /auth/first-setup` over HTTP: the one door into a shop nobody has
//! ever signed into (M4 T8). No session, unlike every other file in this
//! folder but `auth_api.rs`'s own three routes: this is why it needs its
//! own harness rather than `tests/common`, which plants a session before a
//! test's first call.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use diesel::prelude::*;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

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

async fn claim(app: &axum::Router, name: &str, password: &str) -> (StatusCode, Value) {
    call(
        app,
        "POST",
        "/auth/first-setup",
        Some(json!({ "name": name, "password": password })),
    )
    .await
}

/// A second owner already in the shop before anyone has ever signed in, with
/// no credential of their own either: a co-founder's row a seeder or an
/// import wrote, not one this file's own flow made. `sole_active_owner`
/// finds two and refuses to guess between them.
fn add_a_second_owner(db: &std::path::Path, shop: i32) {
    let mut conn = dzpos_core::db::open(db).expect("the test's shop file will not open");
    diesel::sql_query("INSERT INTO users (shop_id, name, role) VALUES (?, 'Nabil', 'owner')")
        .bind::<diesel::sql_types::Integer, _>(shop)
        .execute(&mut conn)
        .expect("the second owner could not be made");
}

#[tokio::test]
async fn a_virgin_shop_says_so_on_health() {
    let (_dir, app) = virgin_shop();
    let (status, body) = call(&app, "GET", "/health", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["needs_first_setup"], true);
}

/// The whole point of the route: a shop nobody has ever signed into goes
/// from unusable to a live session in one call, and the answer names the
/// seeded owner the way `login`'s does.
#[tokio::test]
async fn a_virgin_shop_claims_its_first_owner_and_is_handed_a_session() {
    let (_dir, app) = virgin_shop();
    let (status, body) = claim(&app, "Anouar", "huit caracteres").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["me"]["user_id"], OWNER);
    assert_eq!(body["me"]["name"], "Anouar");
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

    let (status, health) = call(&app, "GET", "/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(health["needs_first_setup"], false);
}

/// A PIN the shape rule refuses (`services::users::validate_pin`) is refused
/// here too, before anything is claimed: `422 validation`, the same code a
/// screen already translates.
#[tokio::test]
async fn a_badly_shaped_password_is_refused_and_claims_nothing() {
    let (_dir, app) = virgin_shop();
    let (status, body) = claim(&app, "Anouar", "short").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");
    let (status, body) = claim(&app, "Anouar", "huit caracteres").await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// The door shuts for good the moment this route itself has been walked
/// through once: a second claim right after the first is refused. The two
/// tests below prove the door shuts for a credential written some other
/// way too, which is the claim this one's old name made without a body to
/// back it.
#[tokio::test]
async fn the_door_shuts_after_a_second_claim() {
    let (_dir, app) = virgin_shop();
    let (status, _) = claim(&app, "Anouar", "huit caracteres").await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = claim(&app, "Samir", "autre mot de passe").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");
    assert_eq!(body["error"]["field"], "password");
}

/// `any_credential_set` is proved directly against the service in
/// `tests/users_service.rs`; this is the same door proved through the wire
/// both ends travel: a credential written by the ordinary reset route,
/// `POST /users/{id}/pin`, from a session planted the way every other file
/// in this folder signs one in (`tests/common`), must be seen by the
/// unauthenticated claim route sitting beside it. Since that route needs no
/// session, a gap here is a way to take over a live till, not a cosmetic
/// one.
#[tokio::test]
async fn the_door_shuts_after_a_pin_set_the_ordinary_way() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let app = common::signed_in_router(&path, SHOP, &token());

    let req = Request::builder()
        .method("POST")
        .uri(format!("/users/{OWNER}/pin"))
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, common::OWNER_SESSION)
        .header("content-type", "application/json")
        .body(Body::from(json!({ "pin": "2580" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let (status, body) = claim(&app, "Anouar", "huit caracteres").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");
    assert_eq!(body["error"]["field"], "password");
}

/// `more_than_one_owner_with_no_credential_yet_is_refused_rather_than_guessed`
/// in `tests/users_service.rs` proves the service refuses to guess between
/// two owners neither of whom has signed in yet; this is the same shop
/// reached through the unauthenticated route itself, the one that would
/// actually be sitting open on a real installation.
#[tokio::test]
async fn the_door_refuses_a_shop_with_two_owners_and_neither_credentialed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    drop(dzpos_core::db::open(&path).unwrap());
    add_a_second_owner(&path, SHOP);
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    let app = dzpos_api::router(state, &token());

    let (status, body) = claim(&app, "Anouar", "huit caracteres").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");
    assert_eq!(body["error"]["field"], "name");
}

/// The route needs the launch token like every other one, and answers the
/// same 401 a stranger's page would get calling it without one: `unauthorized`
/// by its code, not only its status, because `auth_api.rs`'s
/// `the_three_401s_are_told_apart_by_their_code` is what tells this one apart
/// from a refused PIN and a missing session, and a status check alone would
/// not notice this route answering the wrong one of the three.
#[tokio::test]
async fn the_route_still_wants_the_launch_token() {
    let (_dir, app) = virgin_shop();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/first-setup")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Anouar", "password": "huit caracteres" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["code"], "unauthorized", "{body}");
}
