// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// S7 of `a-kernel-crate-and-retail-as-the-first-module`: `/audit-log` is a
// kernel route, but every fixture row this file reads back is written
// through `/products` or `/customers`, both retail-only, so this whole file
// has nothing to compile with the feature off.
#![cfg(feature = "retail")]

//! The owner's audit log screen (M4 T7, features.md §5). What the route
//! answers is checked against a row a real service wrote — `services::
//! products::update`, driven the ordinary way, through `PUT /products/{id}`
//! — never a row this file inserts itself: a test that wrote its own row and
//! read it back would prove the repo query works and nothing about whether
//! the screen shows what the shop's other services actually record.
//!
//! Who may reach the route at all (`GET /audit-log` wants `SeeAuditLog`, the
//! owner's alone) is `tests/route_gates.rs`'s own walk over every gated
//! route; it is not repeated here.

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

struct Harness {
    _dir: tempfile::TempDir,
    app: axum::Router,
}

fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    Harness {
        _dir: dir,
        app: dzpos_api::router(state, &token()),
    }
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
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, common::OWNER_SESSION);
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

fn draft() -> Value {
    json!({
        "name": "Huile Elio 5L",
        "barcode": null,
        "category_id": 1,
        "unit": "piece",
        "cost_centimes": 820,
        "selling_centimes": 920,
        "wholesale_centimes": null,
        "qty_on_hand_milli": 24000,
        "low_stock_at_milli": 10000,
        "rate_bps": null,
        "active": true
    })
}

/// The row the screen has to show: made by driving `PUT /products/{id}`
/// through the real router, the same call the products screen makes, with a
/// new selling price. `services::products::update` is what decides a price
/// change is worth an audit row and writes it inside the same transaction as
/// the update itself; this file only proves the read side agrees with it.
async fn priced_product(app: &axum::Router) -> (i64, i64, i64) {
    let (_, made) = call(app, "POST", "/products", Some(draft())).await;
    let id = made["id"].as_i64().unwrap();
    let mut edited = draft();
    edited["selling_centimes"] = json!(18_000);
    let (status, body) = call(app, "PUT", &format!("/products/{id}"), Some(edited)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    (id, 920, 18_000)
}

/// A second, unrelated row: a fiche made through `POST /customers`, which
/// `services::customers::create` logs under `action: "create"`, `entity:
/// "customer"`. It exists so the filter test below has something the owner
/// wrote that its filters must exclude, rather than one row that any filter,
/// even a deleted one, would trivially return.
async fn other_customer(app: &axum::Router) {
    let body = json!({
        "name": "Not This One",
        "party_kind": "company",
        "phone": null,
        "address": null,
        "rc": null,
        "nif": null,
        "nis": null,
        "ai": null,
        "credit_limit_centimes": null,
        "warn_threshold_centimes": null,
        "notes": null,
        "active": true,
        "opening_debt_centimes": null
    });
    let (status, made) = call(app, "POST", "/customers", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
}

#[tokio::test]
async fn the_owner_sees_the_price_change_a_real_service_wrote() {
    let h = harness();
    let (id, before, after) = priced_product(&h.app).await;

    let (status, page) = call(&h.app, "GET", "/audit-log", None).await;
    assert_eq!(status, StatusCode::OK, "{page}");
    let rows = page["rows"].as_array().expect("rows is an array");
    // The seed's own writes (a shop, an owner) carry no audit row; this
    // price change is the whole of the log.
    assert_eq!(rows.len(), 1, "{page}");
    let row = &rows[0];
    assert_eq!(row["action"], "update");
    assert_eq!(row["entity"], "product");
    assert_eq!(row["entity_id"], id);
    // The name behind the id, not just the id: there is no `/users` route
    // yet for the screen to join it itself. The exact name, not merely a
    // non-empty one, so a join that pulled the wrong person's name off the
    // shop's users would still fail here. `common::sign_in` seeds the owner
    // under this name.
    assert_eq!(row["user_name"], "Propriétaire", "{row}");

    let before_json: Value = serde_json::from_str(row["before"].as_str().unwrap()).unwrap();
    let after_json: Value = serde_json::from_str(row["after"].as_str().unwrap()).unwrap();
    assert_eq!(before_json["selling_centimes"], before);
    assert_eq!(after_json["selling_centimes"], after);

    // The two dropdowns' own options travel with the page, read off the same
    // rows: an owner who has only ever changed one price sees one action and
    // the one user who is signed in, not a taxonomy invented in the screen.
    assert_eq!(page["actions"], json!(["update"]));
    let users = page["users"].as_array().expect("users is an array");
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["id"], row["user_id"]);
}

#[tokio::test]
async fn filters_narrow_to_the_matching_row_and_refuse_what_does_not_match() {
    let h = harness();
    let (id, ..) = priced_product(&h.app).await;
    // A second row the same owner wrote, on the same day, that the filters
    // below must exclude. Without it there is exactly one row in the whole
    // log, and the positive assertion after this block would pass even with
    // every `.filter()` in `services::audit::read` deleted.
    other_customer(&h.app).await;

    let (_, unfiltered) = call(&h.app, "GET", "/audit-log", None).await;
    assert_eq!(
        unfiltered["rows"].as_array().map(Vec::len),
        Some(2),
        "{unfiltered}"
    );
    let price_row = unfiltered["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["action"] == "update")
        .expect("the price change is in the log");
    let user_id = price_row["user_id"].as_i64().unwrap();
    let day = price_row["created_at"]
        .as_str()
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .to_owned();

    let (status, matching) = call(
        &h.app,
        "GET",
        &format!("/audit-log?user_id={user_id}&action=update&day={day}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{matching}");
    assert_eq!(
        matching["rows"].as_array().map(Vec::len),
        Some(1),
        "{matching}"
    );
    assert_eq!(matching["rows"][0]["entity_id"], id);

    let (status, other_user) = call(&h.app, "GET", "/audit-log?user_id=999999", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(other_user["rows"].as_array().map(Vec::len), Some(0));

    let (status, other_action) = call(&h.app, "GET", "/audit-log?action=debt.pay", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(other_action["rows"].as_array().map(Vec::len), Some(0));

    let (status, other_day) = call(&h.app, "GET", "/audit-log?day=2020-01-01", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(other_day["rows"].as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn nothing_on_this_route_writes() {
    let h = harness();
    priced_product(&h.app).await;
    for method in ["POST", "PUT", "DELETE"] {
        let (status, _) = call(&h.app, method, "/audit-log", None).await;
        assert_ne!(
            status,
            StatusCode::OK,
            "{method} /audit-log answered as if it were a route"
        );
    }
}
