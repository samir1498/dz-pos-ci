// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The till's shifts over HTTP: opening a drawer, counting it, reading
//! either one back, and the tag a sale rung with no drawer open carries
//! (plan `till-shifts-a-float-and-a-count` T4). The API crate is the
//! product's only entry point (architecture.md, consequence of rule 2), so
//! what these assert is the contract the desktop reads.
//!
//! The expected figures are written by hand from the rule
//! (`opening_cash + that user's cash sales + that user's cash debt
//! payments`) and never read back off the answer that is being checked. Two
//! of them are the point of the fixture: a card sale in the same window, and
//! a second cashier's cash sale in it, neither of which may reach the figure
//! the person at this drawer is counted against.

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

fn app() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    common::sign_in_as(&path, SHOP, "cashier", common::CASHIER_SESSION);
    common::sign_in_as(&path, SHOP, "manager", common::MANAGER_SESSION);
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    (dir, dzpos_api::router(state, &token()))
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    call_as(app, method, uri, body, common::OWNER_SESSION).await
}

async fn call_as(
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

/// A product through the API, so no test here reaches past the routes.
async fn product(app: &axum::Router, name: &str, selling: i64) -> i64 {
    let (status, body) = call(
        app,
        "POST",
        "/products",
        Some(json!({
            "name": name,
            "unit": "piece",
            "cost_centimes": selling / 2,
            "selling_centimes": selling,
            "qty_on_hand_milli": 100_000,
            // Zero-rated, so the figure a line is worth is the figure that
            // was typed and the fixture's arithmetic is the rule's and not
            // a TVA rounding's.
            "rate_bps": 0,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_i64().unwrap()
}

/// One sale of `qty` units at 10 000 centimes each, rung by whoever holds
/// `session`. A cash ring records what the customer handed over, and the
/// fixture hands over the exact amount so no change is due and the drawer
/// takes the price.
async fn sell(app: &axum::Router, p: i64, qty: i64, mode: &str, session: &str) -> Value {
    let mut body = json!({
        "lines": [{ "product_id": p, "qty_milli": qty * 1_000 }],
        "payment_mode": mode,
    });
    if mode == "cash" {
        body["tendered_centimes"] = json!(qty * 10_000);
    }
    let (status, body) = call_as(app, "POST", "/sales", Some(body), session).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body
}

/// Every row of the log with this action, newest first.
async fn rows_for(app: &axum::Router, action: &str) -> Vec<Value> {
    let (status, body) = call(app, "GET", &format!("/audit-log?action={action}"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["rows"].as_array().cloned().unwrap_or_default()
}

fn code(body: &Value) -> &str {
    body["error"]["code"].as_str().unwrap_or("")
}

// ---------------------------------------------------------------------------
// Opening and closing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_cashier_opens_a_drawer_reads_it_back_and_counts_it_at_close() {
    let (_dir, app) = app();
    let p = product(&app, "Sucre", 10_000).await;

    let (status, opened) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 500_000 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{opened}");
    assert_eq!(opened["opening_cash_centimes"], 500_000);
    assert_eq!(opened["closed_at"], Value::Null);
    assert_eq!(opened["difference_centimes"], Value::Null);
    let id = opened["id"].as_i64().unwrap();

    // Three cash sales of 10 000 by this cashier, one card sale of 10 000 by
    // the same person, and one cash sale of 10 000 by a second cashier. Only
    // the first three reach the drawer:
    //
    //   500 000 opening + 3 × 10 000 cash = 530 000
    //
    // written out by hand here, never summed by the code under test. Without
    // the card sale, the figure passes against a sum that forgot to filter
    // the payment mode; without the manager's, against a shop-wide sum.
    for _ in 0..3 {
        sell(&app, p, 1, "cash", common::CASHIER_SESSION).await;
    }
    sell(&app, p, 1, "card", common::CASHIER_SESSION).await;
    sell(&app, p, 1, "cash", common::MANAGER_SESSION).await;

    let (status, open) = call_as(
        &app,
        "GET",
        "/till/shifts/open",
        None,
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{open}");
    assert_eq!(open["shift"]["id"], id);
    assert_eq!(open["expected_centimes"], 530_000);
    assert_eq!(open["takings"]["sales_centimes"], 30_000);
    // Nothing has been counted, so there is nothing to be short of yet.
    assert_eq!(open["difference_centimes"], Value::Null);

    // The drawer holds 528 000: two thousand centimes short, and short is
    // negative.
    let (status, closed) = call_as(
        &app,
        "POST",
        &format!("/till/shifts/{id}/close"),
        Some(json!({ "counted_centimes": 528_000, "note": "un billet manquant" })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{closed}");
    assert_eq!(closed["expected_at_close_centimes"], 530_000);
    assert_eq!(closed["counted_centimes"], 528_000);
    assert_eq!(closed["difference_centimes"], -2_000);
    assert_eq!(closed["note"], "un billet manquant");

    // And the drawer is no longer open, which is `null` and not a 404: the
    // till screen asks this on every sign-in and acts on the answer.
    let (status, none) = call_as(
        &app,
        "GET",
        "/till/shifts/open",
        None,
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{none}");
    assert_eq!(none, Value::Null);
}

#[tokio::test]
async fn a_drawer_that_does_not_match_and_carries_no_reason_is_refused_on_the_note() {
    let (_dir, app) = app();
    let (status, opened) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 500_000 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{opened}");
    let id = opened["id"].as_i64().unwrap();

    // Nothing was sold, so the expected figure is the float. A count that is
    // not 500 000 differs, and a difference needs a reason (ruling 4).
    let (status, refused) = call_as(
        &app,
        "POST",
        &format!("/till/shifts/{id}/close"),
        Some(json!({ "counted_centimes": 499_000 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(code(&refused), "validation", "{refused}");
    assert_eq!(refused["error"]["field"], "note", "{refused}");

    // The same count with a reason goes through, and a clean count with no
    // reason goes through too: the rule is about the gap, not about the
    // field always being filled in.
    let (status, closed) = call_as(
        &app,
        "POST",
        &format!("/till/shifts/{id}/close"),
        Some(json!({ "counted_centimes": 500_000 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{closed}");
    assert_eq!(closed["difference_centimes"], 0);
    assert_eq!(closed["note"], Value::Null);
}

#[tokio::test]
async fn a_drawer_cannot_open_with_less_than_nothing_in_it() {
    let (_dir, app) = app();
    let (status, refused) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": -1 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["field"], "opening_cash_centimes");

    // And a float past what a JSON number carries without loss is refused at
    // this edge rather than rounded on its way through JavaScript.
    let (status, refused) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 9_007_199_254_740_992i64 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["field"], "opening_cash_centimes");
}

// ---------------------------------------------------------------------------
// Neither moment reaches the wire
// ---------------------------------------------------------------------------

/// The absence proved over HTTP and not by reading the struct. A body naming
/// a moment is refused outright, so nothing a client sends can move the
/// window the expected figure is summed over.
#[tokio::test]
async fn a_body_naming_a_moment_is_refused_and_the_stored_one_is_the_servers() {
    let (_dir, app) = app();

    let (status, refused) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({
            "opening_cash_centimes": 500_000,
            "opened_at": "2020-01-01 09:00:00",
        })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(code(&refused), "bad_request", "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("unknown field opened_at"),
        "the refusal did not name the field the client sent: {refused}"
    );

    // The body without it is taken, and the moment stored is the shop's
    // clock now, not 2020.
    let (status, opened) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 500_000 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{opened}");
    let id = opened["id"].as_i64().unwrap();
    let today = dzpos_core::services::clock::now()
        .format("%Y-%m-%d")
        .to_string();
    assert!(
        opened["opened_at"].as_str().unwrap().starts_with(&today),
        "the drawer was opened on {}, not on the shop's own day {today}",
        opened["opened_at"]
    );

    let (status, refused) = call_as(
        &app,
        "POST",
        &format!("/till/shifts/{id}/close"),
        Some(json!({ "counted_centimes": 500_000, "at": "2020-01-01 19:00:00" })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(code(&refused), "bad_request", "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("unknown field at"),
        "the refusal did not name the field the client sent: {refused}"
    );

    let (status, closed) = call_as(
        &app,
        "POST",
        &format!("/till/shifts/{id}/close"),
        Some(json!({ "counted_centimes": 500_000 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{closed}");
    assert!(
        closed["closed_at"].as_str().unwrap().starts_with(&today),
        "the drawer was counted on {}, not on the shop's own day {today}",
        closed["closed_at"]
    );
}

// ---------------------------------------------------------------------------
// Who may call what
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_cashier_opens_a_till_and_is_refused_somebody_elses_shift_by_name() {
    let (_dir, app) = app();
    let (status, opened) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 100 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "a cashier holds open_and_close_till (ruling 10) and was refused: {opened}"
    );
    let id = opened["id"].as_i64().unwrap();

    // Reading a shift by id is the shift report, which is `see_reports`: a
    // cashier does not run the floor off it. The refusal names the
    // permission so a screen never has to guess which one it wanted.
    let (status, refused) = call_as(
        &app,
        "GET",
        &format!("/till/shifts/{id}"),
        None,
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(code(&refused), "forbidden", "{refused}");
    assert_eq!(refused["error"]["permission"], "see_reports", "{refused}");

    // The manager, who holds it, reads the same shift.
    let (status, report) = call_as(
        &app,
        "GET",
        &format!("/till/shifts/{id}"),
        None,
        common::MANAGER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["shift"]["id"], id);
    assert_eq!(report["expected_centimes"], 100);

    // And the cashier still reads their own open drawer, which names nobody
    // else and carries no gate row at all. This is the pair the split exists
    // for: without it, `see_reports` on the id route would leave a cashier
    // unable to see the figure they are about to be counted against.
    let (status, own) = call_as(
        &app,
        "GET",
        "/till/shifts/open",
        None,
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{own}");
    assert_eq!(own["shift"]["id"], id);
}

/// `/till/shifts/open` is a literal segment sitting beside `/till/shifts/{id}`.
/// A router that preferred the parameter would hand `get_one` the string
/// "open" where an id belongs, and the answer would be a parse failure rather
/// than the caller's drawer.
#[tokio::test]
async fn the_open_read_is_not_swallowed_by_the_route_that_takes_an_id() {
    let (_dir, app) = app();
    let (status, body) = call_as(
        &app,
        "GET",
        "/till/shifts/open",
        None,
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, Value::Null);
}

// ---------------------------------------------------------------------------
// Ruling 2: a sale with no drawer open is tagged, never refused
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_sale_rung_with_no_shift_open_is_accepted_and_tagged() {
    let (_dir, app) = app();
    let p = product(&app, "Sucre", 10_000).await;

    // No drawer has ever been opened. Seven desktop e2e specs, `just seed`,
    // the demo recordings and the Maestro flows all ring exactly like this.
    let sale = sell(&app, p, 1, "cash", common::CASHIER_SESSION).await;
    let tagged = rows_for(&app, "till.sale_outside_shift").await;
    assert_eq!(tagged.len(), 1, "{tagged:?}");
    assert_eq!(tagged[0]["entity"], "document");
    assert_eq!(tagged[0]["entity_id"], sale["id"]);

    // And now with a drawer open, the same sale carries no row: the tag is
    // about falling outside every window of that person's own, not about
    // every sale.
    let (status, opened) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 0 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{opened}");
    sell(&app, p, 1, "cash", common::CASHIER_SESSION).await;
    let tagged = rows_for(&app, "till.sale_outside_shift").await;
    assert_eq!(
        tagged.len(),
        1,
        "a sale rung inside a shift was tagged too: {tagged:?}"
    );

    // The cashier's drawer says nothing about the manager's. A manager
    // ringing while holding no shift of their own is tagged even though
    // somebody else's drawer is open.
    sell(&app, p, 1, "cash", common::MANAGER_SESSION).await;
    let tagged = rows_for(&app, "till.sale_outside_shift").await;
    assert_eq!(
        tagged.len(),
        2,
        "another person's open drawer covered this sale: {tagged:?}"
    );
}

/// No request path refuses such a sale: not the ordinary ring, and not the
/// phone's queue replaying through `issue_idempotent`. A replay answers the
/// stored paper, so it tags once and not twice.
#[tokio::test]
async fn the_offline_replay_path_rings_and_tags_once_and_is_never_refused() {
    let (_dir, app) = app();
    let p = product(&app, "Sucre", 10_000).await;

    let body = json!({
        "lines": [{ "product_id": p, "qty_milli": 1_000 }],
        "payment_mode": "cash",
        "tendered_centimes": 10_000,
        "idempotency_key": "queued-on-the-phone-at-18h00",
    });
    let (status, first) = call_as(
        &app,
        "POST",
        "/sales",
        Some(body.clone()),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{first}");

    let (status, replay) =
        call_as(&app, "POST", "/sales", Some(body), common::CASHIER_SESSION).await;
    assert!(
        status.is_success(),
        "a replay of a sale rung with no drawer open was refused: {status} {replay}"
    );
    assert_eq!(replay["id"], first["id"], "{replay}");

    let tagged = rows_for(&app, "till.sale_outside_shift").await;
    assert_eq!(
        tagged.len(),
        1,
        "the replay wrote a second tag for one sale: {tagged:?}"
    );
    assert_eq!(tagged[0]["entity_id"], first["id"]);
}
