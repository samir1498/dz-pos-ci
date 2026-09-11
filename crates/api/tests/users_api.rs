// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The users screen's routes over HTTP (M4 T8). What a cashier and a manager
//! are refused is `tests/route_gates.rs`'s walk, once `gates::ROUTE_GATES`
//! names `ManageUsers` on every route here; this file is the owner's path
//! through them, and the refusals `services::users` enforces on the row
//! (the last owner, nobody's own fiche) reaching the wire the way every
//! other core refusal does.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

use common::{signed_in_router, OWNER_SESSION};

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
        .header(dzpos_api::session::SESSION_HEADER, session);
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

async fn owner(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    call(app, method, uri, body, OWNER_SESSION).await
}

fn app() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let app = signed_in_router(&path, SHOP, &token());
    (dir, app)
}

/// A fresh cashier's fiche, name and role only: no PIN, because that is its
/// own call.
async fn a_cashier(app: &axum::Router, name: &str) -> i32 {
    let (status, made) = owner(
        app,
        "POST",
        "/users",
        Some(json!({ "name": name, "role": "cashier" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    made["id"].as_i64().unwrap() as i32
}

#[tokio::test]
async fn the_list_starts_with_the_seeded_owner_and_nothing_else() {
    let (_dir, app) = app();
    let (status, listed) = owner(&app, "GET", "/users", None).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    let rows = listed.as_array().unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["role"], "owner");
    assert_eq!(rows[0]["active"], true);
}

#[tokio::test]
async fn adding_a_user_takes_a_name_and_a_role_and_nothing_says_it_has_a_pin_yet() {
    let (_dir, app) = app();
    let (status, made) = owner(
        &app,
        "POST",
        "/users",
        Some(json!({ "name": "Karim", "role": "cashier" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["name"], "Karim");
    assert_eq!(made["role"], "cashier");
    assert_eq!(made["active"], true);
    assert_eq!(
        made["has_pin"], false,
        "a fresh fiche already reads as having a PIN"
    );
    assert_eq!(made["has_password"], false);

    let (_, listed) = owner(&app, "GET", "/users", None).await;
    assert_eq!(listed.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn two_users_of_one_shop_cannot_share_a_name() {
    let (_dir, app) = app();
    a_cashier(&app, "Karim").await;
    let (status, refused) = owner(
        &app,
        "POST",
        "/users",
        Some(json!({ "name": "Karim", "role": "cashier" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert_eq!(refused["error"]["code"], "conflict");
    assert_eq!(refused["error"]["field"], "name");
}

/// Resetting a PIN never shows the old one, because there is not one to
/// show: the answer is the fiche, the same shape `POST /users` and
/// `GET /users` give, and it carries no field a hash or a PIN could hide in.
#[tokio::test]
async fn resetting_a_pin_says_only_that_one_is_set_now() {
    let (_dir, app) = app();
    let id = a_cashier(&app, "Karim").await;
    let (status, after) = owner(
        &app,
        "POST",
        &format!("/users/{id}/pin"),
        Some(json!({ "pin": "3690" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after["has_pin"], true);
    let mut keys: Vec<&str> = after
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "active",
            "has_password",
            "has_pin",
            "id",
            "name",
            "role",
            "shop_id"
        ],
        "the answer carries a field that was not asked for"
    );
}

#[tokio::test]
async fn a_pin_the_shape_rule_refuses_is_refused_on_the_wire_too() {
    let (_dir, app) = app();
    let id = a_cashier(&app, "Karim").await;
    let (status, refused) = owner(
        &app,
        "POST",
        &format!("/users/{id}/pin"),
        Some(json!({ "pin": "1234" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["code"], "validation");
    assert_eq!(refused["error"]["field"], "pin");
}

#[tokio::test]
async fn deactivating_and_reactivating_a_user_round_trips() {
    let (_dir, app) = app();
    let id = a_cashier(&app, "Karim").await;
    let (status, off) = owner(&app, "POST", &format!("/users/{id}/deactivate"), None).await;
    assert_eq!(status, StatusCode::OK, "{off}");
    assert_eq!(off["active"], false);

    let (status, on) = owner(&app, "POST", &format!("/users/{id}/reactivate"), None).await;
    assert_eq!(status, StatusCode::OK, "{on}");
    assert_eq!(on["active"], true);
}

/// `services::users::deactivate`'s own refusal, reaching the wire as a 422
/// naming the field it is about, the way every validation refusal does.
#[tokio::test]
async fn an_owner_cannot_deactivate_their_own_row() {
    let (_dir, app) = app();
    let (status, listed) = owner(&app, "GET", "/users", None).await;
    let owner_id = listed.as_array().unwrap()[0]["id"].as_i64().unwrap();
    let (status_self, refused) =
        owner(&app, "POST", &format!("/users/{owner_id}/deactivate"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(status_self, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["code"], "validation");
    assert_eq!(refused["error"]["field"], "active");
}

/// A second owner may deactivate the first, because two owners means neither
/// is the shop's last one: the refusal is about the count, not about a role
/// deactivating a role. `tests/users_service.rs` is where "the last owner,
/// moved off by a different actor" is proved directly against the service;
/// reaching that same state over HTTP is not possible with real sessions,
/// because the one route this refusal guards is `ManageUsers`-gated, and an
/// owner asking to switch off the shop's only other owner is always, by the
/// time there is only one left, asking about their own row.
#[tokio::test]
async fn a_second_owner_may_be_switched_off_because_two_owners_is_not_the_last_one() {
    let (_dir, app) = app();
    let (status, second) = owner(
        &app,
        "POST",
        "/users",
        Some(json!({ "name": "Nabil", "role": "owner" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{second}");
    let second_id = second["id"].as_i64().unwrap();

    let (status, off) = owner(
        &app,
        "POST",
        &format!("/users/{second_id}/deactivate"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{off}");
    assert_eq!(off["active"], false);
}
