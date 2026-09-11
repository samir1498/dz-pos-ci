// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Signing in over HTTP (M4 T2). In-process router, real temp SQLite file.
//!
//! What this file holds to is the part the rest of the suite cannot see,
//! because the rest of it signs in by planting a row: the three routes, both
//! ways the token travels, the three 401s told apart by their code, and the
//! guard on every other route.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";
/// The owner the first migration seeds, and the row the PIN below goes on.
const OWNER: i32 = 1;
const OWNER_PIN: &str = "2580";
const OWNER_PASSWORD: &str = "correct horse";

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).unwrap()
}

struct Harness {
    _dir: tempfile::TempDir,
    app: axum::Router,
    owner_name: String,
}

/// A shop whose owner has both credentials set, which is what lets one file
/// exercise the PIN pad and the password screen.
fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = dzpos_core::db::open(&path).unwrap();
    dzpos_core::services::users::set_pin(&mut conn, SHOP, OWNER, OWNER, OWNER_PIN).unwrap();
    dzpos_core::services::users::set_password(&mut conn, SHOP, OWNER, OWNER, OWNER_PASSWORD)
        .unwrap();
    let owner_name = dzpos_core::services::users::get(&mut conn, SHOP, OWNER)
        .unwrap()
        .name;
    drop(conn);
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    Harness {
        _dir: dir,
        app: dzpos_api::router(state, &token()),
        owner_name,
    }
}

/// One call, with whatever extra headers the case needs. The response's
/// `set-cookie` comes back too, because half of this file is about it.
async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    headers: &[(&str, &str)],
) -> (StatusCode, Value, Vec<String>) {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"));
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    let req = match body {
        Some(v) => req
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let cookies: Vec<String> = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok().map(str::to_owned))
        .collect();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value, cookies)
}

async fn login(app: &axum::Router, body: Value) -> (StatusCode, Value, Vec<String>) {
    call(app, "POST", "/auth/login", Some(body), &[]).await
}

fn header(token: &str) -> Vec<(&'static str, String)> {
    vec![("x-dzpos-session", token.to_owned())]
}

/// The till's way in: a user id off the list and a PIN. The answer names who
/// is now acting, what they may do, and how long the session survives.
#[tokio::test]
async fn a_pin_signs_the_till_in_and_the_answer_says_who_is_acting() {
    let h = harness();
    let (status, body, cookies) =
        login(&h.app, json!({ "user_id": OWNER, "pin": OWNER_PIN })).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(body["me"]["user_id"], OWNER);
    assert_eq!(body["me"]["name"], h.owner_name);
    assert_eq!(body["me"]["role"], "owner");
    assert_eq!(body["idle_minutes"], 15);

    // The permission list is the core's table, walked, not a list the API
    // keeps of its own: an owner holds every one of them.
    let held = body["me"]["permissions"].as_array().unwrap();
    assert_eq!(held.len(), 13, "{held:?}");
    for wanted in ["sell", "manage_users", "see_audit_log", "commit_money"] {
        assert!(held.iter().any(|p| p == wanted), "{wanted} is missing");
    }

    // The token is 64 hex characters and the cookie carries the same one.
    let token = body["token"].as_str().unwrap();
    assert_eq!(token.len(), 64);
    assert!(token.bytes().all(|c| c.is_ascii_hexdigit()));
    let cookie = cookies.first().expect("no cookie was set");
    assert!(
        cookie.contains(&format!("dzpos_session={token}")),
        "{cookie}"
    );
    assert!(cookie.contains("HttpOnly"), "{cookie}");
    assert!(cookie.contains("SameSite=Lax"), "{cookie}");
    assert!(!cookie.contains("Secure"), "{cookie}");
}

/// The office screen's way in: a name and a password.
#[tokio::test]
async fn a_password_signs_the_office_screen_in() {
    let h = harness();
    let (status, body, _) = login(
        &h.app,
        json!({ "name": h.owner_name, "password": OWNER_PASSWORD }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["me"]["user_id"], OWNER);
}

/// One refusal for every wrong case, and it is `services::users`' single
/// answer. Nothing about a sign-in says whether a name exists.
#[tokio::test]
async fn every_wrong_sign_in_gets_the_one_refusal() {
    let h = harness();
    let cases = [
        json!({ "user_id": OWNER, "pin": "1357" }),
        json!({ "user_id": 9999, "pin": OWNER_PIN }),
        json!({ "name": h.owner_name, "password": "not it at all" }),
        json!({ "name": "Nobody Who Works Here", "password": "not it at all" }),
    ];
    for body in cases {
        let (status, answered, cookies) = login(&h.app, body.clone()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{body} -> {answered}");
        assert_eq!(answered["error"]["code"], "auth_refused", "{body}");
        assert!(cookies.is_empty(), "a refused sign-in set a cookie: {body}");
        assert_eq!(answered["token"], Value::Null);
        // Not a word about which of the four it was.
        let said = answered["error"]["message"].as_str().unwrap_or_default();
        for leak in ["name", "user", "exists", "PIN", "password", "deactivated"] {
            assert!(!said.contains(leak), "{said} says {leak}");
        }
    }
}

/// The three 401s this API has are told apart by their code, because a screen
/// does something different on each: a wrong launch token is the app started
/// wrong, a refused credential is the PIN just typed, and a missing session is
/// "sign in again".
#[tokio::test]
async fn the_three_401s_are_told_apart_by_their_code() {
    let h = harness();

    // No launch token at all.
    let no_launch = h
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/products")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(no_launch.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        no_launch.headers().get("www-authenticate").unwrap(),
        "Bearer"
    );
    let bytes = no_launch.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["code"], "unauthorized");

    // Launch token, no session.
    let (status, body, _) = call(&h.app, "GET", "/products", None, &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "session_required");

    // Launch token, a session token nobody issued.
    let (status, body, _) = call(
        &h.app,
        "GET",
        "/products",
        None,
        &[("x-dzpos-session", &"0".repeat(64))],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "session_required");

    // A wrong credential.
    let (status, body, _) = login(&h.app, json!({ "user_id": OWNER, "pin": "1357" })).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "auth_refused");
}

/// Both ways the token travels, against a route that is not the auth ones.
#[tokio::test]
async fn the_desktops_header_and_the_browsers_cookie_both_carry_the_session() {
    let h = harness();
    let (_, body, cookies) = login(&h.app, json!({ "user_id": OWNER, "pin": OWNER_PIN })).await;
    let session = body["token"].as_str().unwrap().to_owned();

    let (status, listed, _) = call(
        &h.app,
        "GET",
        "/products",
        None,
        &[("x-dzpos-session", &session)],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{listed}");

    // The browser sends back what the Set-Cookie told it to, name and value.
    let jar = cookies
        .first()
        .and_then(|c| c.split(';').next())
        .expect("no cookie to send back")
        .to_owned();
    let (status, listed, _) = call(&h.app, "GET", "/products", None, &[("cookie", &jar)]).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
}

/// What the desktop reads on focus (T4): who is signed in, without a body and
/// without sliding the idle time forward.
#[tokio::test]
async fn me_answers_who_is_signed_in_and_401s_when_nobody_is() {
    let h = harness();
    let (status, body, _) = call(&h.app, "GET", "/auth/me", None, &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "session_required");

    let (_, signed_in, _) = login(&h.app, json!({ "user_id": OWNER, "pin": OWNER_PIN })).await;
    let session = signed_in["token"].as_str().unwrap().to_owned();
    let (status, me, _) = call(
        &h.app,
        "GET",
        "/auth/me",
        None,
        &[("x-dzpos-session", &session)],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!(me, signed_in["me"]);
}

/// Signing out ends the session and clears the cookie; a second sign-out is
/// the same answer, because a caller has nothing to do differently on "you
/// were not signed in".
#[tokio::test]
async fn signing_out_ends_the_session_and_clears_the_cookie() {
    let h = harness();
    let (_, signed_in, _) = login(&h.app, json!({ "user_id": OWNER, "pin": OWNER_PIN })).await;
    let session = signed_in["token"].as_str().unwrap().to_owned();

    let (status, _, cookies) = call(
        &h.app,
        "POST",
        "/auth/logout",
        None,
        &[("x-dzpos-session", &session)],
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let cleared = cookies.first().expect("no cookie was cleared");
    assert!(cleared.contains("Max-Age=0"), "{cleared}");

    let (status, body, _) = call(
        &h.app,
        "GET",
        "/products",
        None,
        &[("x-dzpos-session", &session)],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "session_required");

    // Twice, and with a token that was never a session.
    for token in [session.as_str(), "not a token"] {
        let (status, _, _) = call(
            &h.app,
            "POST",
            "/auth/logout",
            None,
            &[("x-dzpos-session", token)],
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }
    // And with no session at all.
    let (status, _, _) = call(&h.app, "POST", "/auth/logout", None, &[]).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// The heart of the task: a write names the person who signed in, not the
/// seeded owner id the API used to carry. Asserted on the stored row and not
/// on the answer, because the answer would have said the same thing before.
#[tokio::test]
async fn a_write_names_the_user_the_session_says_is_acting() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = dzpos_core::db::open(&path).unwrap();
    let cashier = dzpos_core::services::users::create(
        &mut conn,
        SHOP,
        OWNER,
        dzpos_core::services::users::NewUser {
            name: "Karim".to_owned(),
            role: dzpos_core::services::users::Role::Cashier,
        },
    )
    .unwrap();
    dzpos_core::services::users::set_pin(&mut conn, SHOP, OWNER, cashier.id, "3690").unwrap();
    drop(conn);

    let app = dzpos_api::router(dzpos_api::AppState::open(&path, SHOP).unwrap(), &token());
    let (_, signed_in, _) = login(&app, json!({ "user_id": cashier.id, "pin": "3690" })).await;
    let session = signed_in["token"].as_str().unwrap().to_owned();
    assert_eq!(signed_in["me"]["role"], "cashier");

    let (status, made, _) = call(
        &app,
        "POST",
        "/products",
        Some(json!({
            "name": "Semoule 10kg",
            "barcode": null,
            "category_id": null,
            "unit": "piece",
            "cost_centimes": 70_000,
            "selling_centimes": 100_000,
            "wholesale_centimes": null,
            "qty_on_hand_milli": 5000,
            "low_stock_at_milli": 0,
            "rate_bps": 1900,
            "active": true
        })),
        &[("x-dzpos-session", &session)],
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");

    // The opening stock movement the product service wrote names Karim, and
    // nothing was written under the seeded owner. Asserted on the stored row
    // and not on the answer, because the answer would have said the same
    // thing before this task.
    use diesel::prelude::*;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        user_id: i32,
    }
    let mut conn = dzpos_core::db::open(&path).unwrap();
    let rows: Vec<Row> = diesel::sql_query("SELECT user_id FROM stock_movements")
        .load(&mut conn)
        .unwrap();
    assert!(!rows.is_empty(), "the product write left no movement");
    for row in rows {
        assert_eq!(
            row.user_id, cashier.id,
            "a write was recorded under somebody who was not signed in"
        );
    }
}

/// A session opened against one shop is nobody on a router answering for
/// another, so a second till's token is not a way into this shop's rows.
#[tokio::test]
async fn a_session_of_one_shop_does_not_open_another() {
    let h = harness();
    let (_, signed_in, _) = login(&h.app, json!({ "user_id": OWNER, "pin": OWNER_PIN })).await;
    let session = signed_in["token"].as_str().unwrap().to_owned();

    let _ = header(&session);
    let (status, body, _) = call(
        &h.app,
        "GET",
        "/products",
        None,
        &[("x-dzpos-session", &session)],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// The lockout reaches the wire with the wait beside the code, so the sign-in
/// screen counts it down rather than working it out (`services::users` owns
/// the rule; this is only that it travels).
#[tokio::test]
async fn a_run_of_wrong_pins_answers_429_with_the_wait() {
    let h = harness();
    for _ in 0..dzpos_core::services::users::FAILURES_BEFORE_LOCKOUT {
        let (status, _, _) = login(&h.app, json!({ "user_id": OWNER, "pin": "1357" })).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let (status, body, _) = login(&h.app, json!({ "user_id": OWNER, "pin": OWNER_PIN })).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
    assert_eq!(body["error"]["code"], "locked_out");
    assert!(
        body["error"]["retry_after_seconds"].as_i64().unwrap_or(0) > 0,
        "{body}"
    );
}

/// A body that is neither of the two shapes. Refused before any service runs,
/// so a caller that sent a name beside a PIN is told it wrote the request
/// wrong rather than being counted as a wrong credential.
#[tokio::test]
async fn a_body_that_is_neither_shape_is_a_bad_request() {
    let h = harness();
    for body in [
        json!({}),
        json!({ "user_id": OWNER }),
        json!({ "name": "Karim" }),
        json!({ "pin": "2580" }),
    ] {
        let (status, answered, _) = login(&h.app, body.clone()).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{body} -> {answered}"
        );
        assert_eq!(answered["error"]["code"], "bad_request", "{body}");
    }
}

/// `/auth/idle` is behind the session guard and answers the shop's figure, so
/// the desktop's lock screen counts against one number and not its own.
#[tokio::test]
async fn the_idle_time_is_readable_by_anybody_signed_in_and_nobody_else() {
    let h = harness();
    let (status, body, _) = call(&h.app, "GET", "/auth/idle", None, &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "session_required");

    let (_, signed_in, _) = login(&h.app, json!({ "user_id": OWNER, "pin": OWNER_PIN })).await;
    let session = signed_in["token"].as_str().unwrap().to_owned();
    let (status, body, _) = call(
        &h.app,
        "GET",
        "/auth/idle",
        None,
        &[("x-dzpos-session", &session)],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["idle_minutes"], 15);
}

/// `/health` is the one route outside both gates, because a launcher asks it
/// whether the server is up before anybody has signed in.
#[tokio::test]
async fn health_still_answers_with_no_session() {
    let h = harness();
    let res = h
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

/// A deactivated user's session stops working on the next request, rather
/// than lasting until its idle time runs out.
#[tokio::test]
async fn switching_a_user_off_ends_the_session_they_were_holding() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = dzpos_core::db::open(&path).unwrap();
    let cashier = dzpos_core::services::users::create(
        &mut conn,
        SHOP,
        OWNER,
        dzpos_core::services::users::NewUser {
            name: "Karim".to_owned(),
            role: dzpos_core::services::users::Role::Cashier,
        },
    )
    .unwrap();
    dzpos_core::services::users::set_pin(&mut conn, SHOP, OWNER, cashier.id, "3690").unwrap();
    drop(conn);

    let app = dzpos_api::router(dzpos_api::AppState::open(&path, SHOP).unwrap(), &token());
    let (_, signed_in, _) = login(&app, json!({ "user_id": cashier.id, "pin": "3690" })).await;
    let session = signed_in["token"].as_str().unwrap().to_owned();
    let (status, _, _) = call(
        &app,
        "GET",
        "/products",
        None,
        &[("x-dzpos-session", &session)],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let mut conn = dzpos_core::db::open(&path).unwrap();
    dzpos_core::services::users::deactivate(&mut conn, SHOP, OWNER, cashier.id).unwrap();
    drop(conn);

    let (status, body, _) = call(
        &app,
        "GET",
        "/products",
        None,
        &[("x-dzpos-session", &session)],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"]["code"], "session_required");
}
