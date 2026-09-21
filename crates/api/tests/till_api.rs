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
    let (dir, app, _) = app_with_two_cashiers();
    (dir, app)
}

/// Who the ids belong to. A shift is about one person, so a test that cannot
/// name the people cannot assert whose drawer it is looking at.
struct Staff {
    cashier: i32,
    second_cashier: i32,
    manager: i32,
}

/// The same router, plus a second cashier. `sign_in_as` takes the first
/// active user of a role, so it can never make two of one; ruling 10 is about
/// one cashier reaching for another cashier's drawer, which needs two.
fn app_with_two_cashiers() -> (tempfile::TempDir, axum::Router, Staff) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    common::sign_in_as(&path, SHOP, "cashier", common::CASHIER_SESSION);
    common::sign_in_as(&path, SHOP, "manager", common::MANAGER_SESSION);
    let second_cashier = common::sign_in_new(
        &path,
        SHOP,
        "cashier",
        "Karim",
        common::SECOND_CASHIER_SESSION,
    );
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    let staff = Staff {
        cashier: user_of(&path, "cashier"),
        second_cashier,
        manager: user_of(&path, "manager"),
    };
    (dir, dzpos_api::router(state, &token()), staff)
}

/// The id behind a role's session, read off the file the harness just made.
/// `sign_in_as` plants the row and does not say which id it used, and every
/// assertion about whose drawer a shift is needs that number.
fn user_of(db: &std::path::Path, role: &str) -> i32 {
    use diesel::prelude::*;
    #[derive(diesel::QueryableByName)]
    struct Id {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        id: i32,
    }
    let mut conn = dzpos_core::db::open(db).unwrap();
    let row: Id = diesel::sql_query(
        "SELECT id FROM users WHERE shop_id = ? AND role = ? AND active = 1 ORDER BY id LIMIT 1",
    )
    .bind::<diesel::sql_types::Integer, _>(SHOP)
    .bind::<diesel::sql_types::Text, _>(role)
    .get_result(&mut conn)
    .unwrap();
    row.id
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

/// A customer with room on the account, so a credit sale has somewhere to be
/// owed from. Through the API, like every other fixture here.
async fn customer(app: &axum::Router, name: &str) -> i64 {
    let (status, body) = call(
        app,
        "POST",
        "/customers",
        Some(json!({
            "name": name,
            "party_kind": "company",
            "phone": null,
            "address": null,
            "rc": null,
            "nif": null,
            "nis": null,
            "ai": null,
            "credit_limit_centimes": 10_000_000,
            "warn_threshold_centimes": null,
            "notes": null,
            "active": true,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_i64().unwrap()
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

/// `services::shifts::open`'s rule: a drawer opens with nothing in it or with
/// money in it, never with less.
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
    assert_eq!(code(&refused), "validation", "{refused}");
    assert_eq!(refused["error"]["field"], "opening_cash_centimes");
    assert!(
        refused["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("never with less"),
        "{refused}"
    );

    // Zero is not less than nothing: an empty drawer is a real float and the
    // refusal above must not swallow it.
    let (status, opened) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 0 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{opened}");
    assert_eq!(opened["opening_cash_centimes"], 0);
}

/// A different rule that happens to share a field name: `dto::common`'s
/// ceiling on what a JSON number carries without loss. It is about size and
/// not about sign — it takes `-1` happily — so it is its own test and not a
/// second half of the one above.
#[tokio::test]
async fn a_float_beyond_what_a_json_number_carries_is_refused_at_the_edge() {
    let (_dir, app) = app();
    for absurd in [9_007_199_254_740_992i64, -9_007_199_254_740_992i64] {
        let (status, refused) = call_as(
            &app,
            "POST",
            "/till/shifts",
            Some(json!({ "opening_cash_centimes": absurd })),
            common::CASHIER_SESSION,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
        assert_eq!(refused["error"]["field"], "opening_cash_centimes");
        assert!(
            refused["error"]["message"]
                .as_str()
                .unwrap_or("")
                .contains("2^53"),
            "{absurd} was refused by some other rule: {refused}"
        );
    }

    // And the same ceiling on the count at close, which is the other figure a
    // client sends.
    let (status, opened) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 0 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{opened}");
    let id = opened["id"].as_i64().unwrap();
    let (status, refused) = call_as(
        &app,
        "POST",
        &format!("/till/shifts/{id}/close"),
        Some(json!({ "counted_centimes": 9_007_199_254_740_992i64 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["field"], "counted_centimes");
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
async fn a_cashier_opens_their_own_till_and_is_refused_another_cashiers_shift() {
    // Two cashiers, and the second one's drawer is a drawer the first has no
    // business in: not the read, which is `see_reports`, and not the close,
    // which is `close_another_persons_till` (ruling 10). A fixture with one
    // cashier cannot show either, because the only shift on the file is the
    // caller's own.
    let (_dir, app, staff) = app_with_two_cashiers();

    let (status, mine) = call_as(
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
        "a cashier holds open_and_close_till (ruling 10) and was refused: {mine}"
    );
    assert_eq!(mine["opened_by"], staff.cashier, "{mine}");

    // Somebody else's, opened by the second cashier under their own session.
    let (status, theirs) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 700_000 })),
        common::SECOND_CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{theirs}");
    assert_eq!(theirs["opened_by"], staff.second_cashier, "{theirs}");
    let theirs_id = theirs["id"].as_i64().unwrap();
    assert_ne!(theirs_id, mine["id"].as_i64().unwrap());

    // Reading it is the shift report, which is `see_reports`: whose count was
    // short is a management figure. The refusal names the permission so a
    // screen never has to guess which one it wanted.
    let (status, refused) = call_as(
        &app,
        "GET",
        &format!("/till/shifts/{theirs_id}"),
        None,
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(code(&refused), "forbidden", "{refused}");
    assert_eq!(refused["error"]["permission"], "see_reports", "{refused}");

    // And counting it is `close_another_persons_till`, which a cashier does
    // not hold. The route's own gate is `open_and_close_till` and everybody
    // holds that, so this refusal can only come from inside
    // `services::shifts::close` — which is the point: one route serves both
    // cases and whose drawer it is is a fact about the row.
    let (status, refused) = call_as(
        &app,
        "POST",
        &format!("/till/shifts/{theirs_id}/close"),
        Some(json!({ "counted_centimes": 1, "note": "pas la mienne" })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(code(&refused), "forbidden", "{refused}");
    assert_eq!(
        refused["error"]["permission"], "close_another_persons_till",
        "{refused}"
    );

    // The refusal left the drawer open and stored no figure, so the person it
    // belongs to still counts it themselves.
    let (status, still) = call_as(
        &app,
        "GET",
        "/till/shifts/open",
        None,
        common::SECOND_CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{still}");
    assert_eq!(still["shift"]["id"], theirs_id);
    assert_eq!(still["shift"]["counted_centimes"], Value::Null, "{still}");

    // The manager, who holds both, reads it and counts it. The answer names
    // the opener and the closer apart: without this pair, a DTO that reported
    // a closed shift's closer as its opener would go unnoticed.
    let (status, report) = call_as(
        &app,
        "GET",
        &format!("/till/shifts/{theirs_id}"),
        None,
        common::MANAGER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(
        report["shift"]["opened_by"], staff.second_cashier,
        "{report}"
    );
    assert_eq!(report["expected_centimes"], 700_000);

    let (status, closed) = call_as(
        &app,
        "POST",
        &format!("/till/shifts/{theirs_id}/close"),
        Some(json!({
            "counted_centimes": 690_000,
            "note": "caissier parti, tiroir compté par la responsable",
        })),
        common::MANAGER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{closed}");
    assert_eq!(closed["opened_by"], staff.second_cashier, "{closed}");
    assert_eq!(closed["closed_by"], staff.manager, "{closed}");
    assert_ne!(closed["opened_by"], closed["closed_by"], "{closed}");
    // 700 000 float, nothing sold, 690 000 counted: short by 10 000, and
    // short reads negative.
    assert_eq!(closed["expected_at_close_centimes"], 700_000);
    assert_eq!(closed["difference_centimes"], -10_000);
}

/// A cashier reads their own open drawer with no permission at all, which is
/// the other half of the split above: `see_reports` on the id route would
/// otherwise leave them unable to see the figure they are about to be counted
/// against (T5's close modal).
#[tokio::test]
async fn a_cashier_reads_their_own_open_drawer_and_sees_only_their_own() {
    let (_dir, app, staff) = app_with_two_cashiers();
    for session in [common::CASHIER_SESSION, common::SECOND_CASHIER_SESSION] {
        let (status, opened) = call_as(
            &app,
            "POST",
            "/till/shifts",
            Some(json!({ "opening_cash_centimes": 100 })),
            session,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{opened}");
    }
    for (session, who) in [
        (common::CASHIER_SESSION, staff.cashier),
        (common::SECOND_CASHIER_SESSION, staff.second_cashier),
    ] {
        let (status, own) = call_as(&app, "GET", "/till/shifts/open", None, session).await;
        assert_eq!(status, StatusCode::OK, "{own}");
        assert_eq!(
            own["shift"]["opened_by"], who,
            "the route answered somebody else's drawer: {own}"
        );
    }
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
    let (_dir, app, staff) = app_with_two_cashiers();
    let p = product(&app, "Sucre", 10_000).await;

    // No drawer has ever been opened. Seven desktop e2e specs, `just seed`,
    // the demo recordings and the Maestro flows all ring exactly like this.
    let sale = sell(&app, p, 1, "cash", common::CASHIER_SESSION).await;
    let tagged = rows_for(&app, "till.sale_outside_shift").await;
    assert_eq!(tagged.len(), 1, "{tagged:?}");
    assert_eq!(tagged[0]["entity"], "document");
    assert_eq!(tagged[0]["entity_id"], sale["id"]);
    assert_eq!(tagged[0]["user_id"], staff.cashier, "{tagged:?}");

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
    // somebody else's drawer is open. Which row it is, is the point of the
    // block: a count of two is also what a walk over every open shift in the
    // shop would give, so the row is read by the person and the document it
    // names rather than by how many there are.
    let theirs = sell(&app, p, 1, "cash", common::MANAGER_SESSION).await;
    let tagged = rows_for(&app, "till.sale_outside_shift").await;
    assert_eq!(
        tagged.len(),
        2,
        "another person's open drawer covered this sale: {tagged:?}"
    );
    let managers = tagged
        .iter()
        .find(|row| row["entity_id"] == theirs["id"])
        .unwrap_or_else(|| panic!("no row names the manager's sale: {tagged:?}"));
    assert_eq!(managers["user_id"], staff.manager, "{managers}");
    assert_eq!(managers["entity"], "document", "{managers}");

    // A credit sale is tagged too, and it is its own case: the hook sits
    // above `sales::issue_inner`'s cash arm early return, and a hook below
    // that line tags a credit sale and nothing else. Without this, moving it
    // down leaves every assertion above green.
    let owing = customer(&app, "Entreprise Amrani").await;
    let (status, on_credit) = call_as(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "credit",
            "customer_id": owing,
        })),
        common::SECOND_CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{on_credit}");
    let tagged = rows_for(&app, "till.sale_outside_shift").await;
    assert_eq!(
        tagged.len(),
        3,
        "a credit sale rung with no drawer open was not tagged: {tagged:?}"
    );
    let on_the_account = tagged
        .iter()
        .find(|row| row["entity_id"] == on_credit["id"])
        .unwrap_or_else(|| panic!("no row names the credit sale: {tagged:?}"));
    assert_eq!(
        on_the_account["user_id"], staff.second_cashier,
        "{on_the_account}"
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

// ---------------------------------------------------------------------------
// T7: the shift list a manager runs the floor off
// ---------------------------------------------------------------------------

/// Overwrites a shift's `opened_at` directly on the file, the only way this
/// suite can put a row outside the default day window: the API never takes a
/// moment on either DTO (`dto/till.rs`'s own doc), so a test that wants one
/// on the wrong day has to plant it the way a phone's drifted clock never
/// could.
fn backdate_shift(db: &std::path::Path, id: i64, opened_at: &str) {
    use diesel::prelude::*;
    let mut conn = dzpos_core::db::open(db).expect("the test's shop file will not open");
    diesel::sql_query("UPDATE shifts SET opened_at = ? WHERE id = ?")
        .bind::<diesel::sql_types::Text, _>(opened_at)
        .bind::<diesel::sql_types::Integer, _>(id as i32)
        .execute(&mut conn)
        .expect("the test could not backdate the shift");
}

#[tokio::test]
async fn a_cashier_is_refused_the_list_naming_the_permission() {
    let (_dir, app) = app();
    let (status, body) = call_as(&app, "GET", "/till/shifts", None, common::CASHIER_SESSION).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(code(&body), "forbidden");
    assert_eq!(body["error"]["permission"], "see_reports", "{body}");
}

#[tokio::test]
async fn a_manager_reads_the_shop_shifts_newest_first_row_only() {
    let (_dir, app, staff) = app_with_two_cashiers();

    let (status, first) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 100_000 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    let (status, second) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 200_000 })),
        common::SECOND_CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{second}");

    let (status, body) = call_as(&app, "GET", "/till/shifts", None, common::MANAGER_SESSION).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let rows = body.as_array().expect("a list, not a report");
    let ids: Vec<i64> = rows.iter().map(|r| r["id"].as_i64().unwrap()).collect();
    // Newest first: the second cashier opened after the first, and neither
    // row carries a takings figure or an expected one — a row, not a report.
    assert_eq!(
        ids,
        vec![
            second["id"].as_i64().unwrap(),
            first["id"].as_i64().unwrap()
        ]
    );
    assert_eq!(rows[0]["opened_by"], staff.second_cashier, "{body}");
    assert_eq!(rows[1]["opened_by"], staff.cashier, "{body}");
    assert!(
        rows[0].get("takings").is_none(),
        "the list answered a report, not a row: {body}"
    );
}

#[tokio::test]
async fn a_short_drawer_reads_the_same_in_the_list_as_on_its_own_and_user_id_narrows_it() {
    let (_dir, app, staff) = app_with_two_cashiers();

    let (status, first) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 100_000 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    let first_id = first["id"].as_i64().unwrap();
    let (status, second) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 200_000 })),
        common::SECOND_CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{second}");

    // Nothing was sold, so the drawer should hold its float; 98 000 is two
    // thousand centimes short, and short is negative.
    let (status, closed) = call_as(
        &app,
        "POST",
        &format!("/till/shifts/{first_id}/close"),
        Some(json!({ "counted_centimes": 98_000, "note": "un billet manquant" })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{closed}");

    // The list and the single read are two routes to the same row, and the
    // three money fields must not differ between them by one centime.
    let (status, one) = call_as(
        &app,
        "GET",
        &format!("/till/shifts/{first_id}"),
        None,
        common::MANAGER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{one}");
    let (status, body) = call_as(
        &app,
        "GET",
        &format!("/till/shifts?user_id={}", staff.cashier),
        None,
        common::MANAGER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let rows = body.as_array().expect("a list, not a report");
    // `user_id` over the wire narrows to that cashier: the second cashier's
    // open drawer is not in it.
    assert_eq!(rows.len(), 1, "{body}");
    let row = &rows[0];
    assert_eq!(row["id"], first_id, "{body}");
    assert_eq!(row["expected_at_close_centimes"], 100_000);
    assert_eq!(row["counted_centimes"], 98_000);
    assert_eq!(row["difference_centimes"], -2_000);
    for field in [
        "expected_at_close_centimes",
        "counted_centimes",
        "difference_centimes",
        "note",
    ] {
        assert_eq!(
            row[field], one["shift"][field],
            "{field}: list {body} vs one {one}"
        );
    }
}

#[tokio::test]
async fn another_shops_shifts_never_appear_in_this_ones_list() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    common::sign_in_as(&path, SHOP, "manager", common::MANAGER_SESSION);
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    let app = dzpos_api::router(state, &token());

    let (status, ours) = call(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 50_000 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{ours}");

    // The same file, answered as shop 2 (the pattern
    // `crates/api/tests/purchases_api.rs::other_shop` uses): its own owner,
    // its own drawer, on the same SQLite file this test's own list is asked
    // against.
    const OTHER_SHOP: i32 = 2;
    let elsewhere = common::signed_in_router(&path, OTHER_SHOP, &token());
    let (status, theirs) = call(
        &elsewhere,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 90_000 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{theirs}");

    let (status, body) = call_as(&app, "GET", "/till/shifts", None, common::MANAGER_SESSION).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let ids: Vec<i64> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, vec![ours["id"].as_i64().unwrap()]);
    assert!(
        !ids.contains(&theirs["id"].as_i64().unwrap()),
        "another shop's shift leaked into this one's list: {body}"
    );
}

#[tokio::test]
async fn a_day_window_excludes_a_shift_opened_outside_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    common::sign_in_as(&path, SHOP, "manager", common::MANAGER_SESSION);
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    let app = dzpos_api::router(state, &token());

    let (status, opened) = call(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 70_000 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{opened}");
    let id = opened["id"].as_i64().unwrap();
    backdate_shift(&path, id, "2020-01-05 09:00:00");

    // No window named: the default is today, on the shop's calendar, and a
    // shift planted on a Sunday in January of 2020 is not on it.
    let (status, today) = call_as(&app, "GET", "/till/shifts", None, common::MANAGER_SESSION).await;
    assert_eq!(status, StatusCode::OK, "{today}");
    let ids: Vec<i64> = today
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_i64().unwrap())
        .collect();
    assert!(
        !ids.contains(&id),
        "the default window answered a shift from 2020: {today}"
    );

    // The same day, named directly, is what brings it back — proving the
    // shift above was excluded by the window and not lost some other way.
    let (status, that_day) = call_as(
        &app,
        "GET",
        "/till/shifts?from=2020-01-05&to=2020-01-05",
        None,
        common::MANAGER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{that_day}");
    let ids: Vec<i64> = that_day
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, vec![id], "{that_day}");
}

/// The count of those tagged sales reaches the close screen, which is the
/// wire gap T5 worked around rather than inventing a number for (the plan's
/// own paragraph on `ShiftReportDto`).
///
/// **Why the window is not the shift's own.** A sale is tagged exactly when
/// it fell inside none of that person's windows, so a count taken over
/// `opened_at..until` is zero for every shift that ever existed. The figure
/// worth showing is what this person rang before they opened up, which is
/// what the assertions below pin: two sales rung with no drawer, then the
/// drawer opened, then a third sale that is covered and must not be counted.
#[tokio::test]
async fn the_open_shift_report_counts_the_sales_rung_before_the_drawer_was_opened() {
    let (_dir, app, _staff) = app_with_two_cashiers();
    let p = product(&app, "Sucre", 10_000).await;

    // Two sales this morning with nobody holding a drawer.
    sell(&app, p, 1, "cash", common::CASHIER_SESSION).await;
    sell(&app, p, 1, "cash", common::CASHIER_SESSION).await;
    // And one by somebody else, which is not this cashier's to explain.
    sell(&app, p, 1, "cash", common::MANAGER_SESSION).await;

    let (status, opened) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 0 })),
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{opened}");

    // Rung inside the window, so it is covered and tagged nowhere.
    sell(&app, p, 1, "cash", common::CASHIER_SESSION).await;

    let (status, report) = call_as(
        &app,
        "GET",
        "/till/shifts/open",
        None,
        common::CASHIER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(
        report["rung_outside_shift"], 2,
        "the count is not this cashier's two sales before the drawer opened: {report}"
    );

    // The manager's own drawer, opened after their untagged-by-this-cashier
    // sale, counts that one and not the cashier's two. The figure is per
    // person for the same reason the expected figure is: each drawer is
    // physically somebody's own.
    let (status, theirs) = call_as(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 0 })),
        common::MANAGER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{theirs}");
    let (status, report) = call_as(
        &app,
        "GET",
        "/till/shifts/open",
        None,
        common::MANAGER_SESSION,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(
        report["rung_outside_shift"], 1,
        "one person's untagged morning reached another person's close screen: {report}"
    );
}
