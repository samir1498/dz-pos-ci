// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! One sale per retry key (M7 T4) at the routes: the wire answers 201 for a
//! fresh ring and 200 with the stored paper for a replay, and 409/422 where
//! the key is misused. Totals are asserted equal on the replay, to the
//! centime — a replay that recomputed instead of returning would be a
//! second chance to get the money wrong.

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
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    (dir, dzpos_api::router(state, &token()))
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

async fn product(app: &axum::Router) -> i64 {
    let (status, body) = call(
        app,
        "POST",
        "/products",
        Some(json!({
            "name": "Sucre",
            "unit": "piece",
            "cost_centimes": 5_500,
            "selling_centimes": 11_000,
            "qty_on_hand_milli": 10_000,
            "rate_bps": 1900,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_i64().unwrap()
}

fn basket(product: i64, qty_milli: i64, key: Option<&str>) -> Value {
    let mut sale = json!({
        "lines": [{ "product_id": product, "qty_milli": qty_milli }],
        "payment_mode": "cash",
        "tendered_centimes": 30_000,
    });
    if let Some(key) = key {
        sale["idempotency_key"] = json!(key);
    }
    sale
}

async fn customer(app: &axum::Router) -> i64 {
    let (status, body) = call(
        app,
        "POST",
        "/customers",
        Some(json!({
            "name": "Entreprise Benali",
            "party_kind": "company",
            "phone": "0770 11 22 33",
            "address": null,
            "rc": null,
            "nif": null,
            "nis": null,
            "ai": null,
            "credit_limit_centimes": 5_000_000,
            "warn_threshold_centimes": 4_000_000,
            "notes": null,
            "active": true,
            "opening_debt_centimes": null
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_i64().unwrap()
}

#[tokio::test]
async fn a_replay_answers_200_with_the_same_paper_and_totals() {
    let (_dir, app) = app();
    let p = product(&app).await;
    let (status, first) = call(&app, "POST", "/sales", Some(basket(p, 2_000, Some("k1")))).await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    let (status, second) = call(&app, "POST", "/sales", Some(basket(p, 2_000, Some("k1")))).await;
    assert_eq!(status, StatusCode::OK, "{second}");
    assert_eq!(first["id"], second["id"]);
    assert_eq!(first["number"], second["number"]);
    assert_eq!(first["totals"], second["totals"]);
    // One ring happened, whatever the network did twice.
    let (status, list) = call(&app, "GET", "/sales", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn a_key_on_a_different_basket_is_409() {
    let (_dir, app) = app();
    let p = product(&app).await;
    let (status, _) = call(&app, "POST", "/sales", Some(basket(p, 2_000, Some("k1")))).await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, body) = call(&app, "POST", "/sales", Some(basket(p, 3_000, Some("k1")))).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["error"]["code"], "conflict");
    let (status, list) = call(&app, "GET", "/sales", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn no_key_rings_every_time_with_201() {
    let (_dir, app) = app();
    let p = product(&app).await;
    let (first_status, first) = call(&app, "POST", "/sales", Some(basket(p, 2_000, None))).await;
    let (second_status, second) = call(&app, "POST", "/sales", Some(basket(p, 2_000, None))).await;
    assert_eq!(
        (first_status, second_status),
        (StatusCode::CREATED, StatusCode::CREATED)
    );
    assert_ne!(first["id"], second["id"]);
}

#[tokio::test]
async fn empty_and_long_keys_are_422() {
    let (_dir, app) = app();
    let p = product(&app).await;
    for key in ["".to_string(), "k".repeat(129)] {
        let (status, body) = call(&app, "POST", "/sales", Some(basket(p, 2_000, Some(&key)))).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["error"]["code"], "validation");
    }
}

#[tokio::test]
async fn a_key_on_a_quotation_is_422() {
    let (_dir, app) = app();
    let p = product(&app).await;
    let customer = customer(&app).await;
    let mut quote = basket(p, 2_000, Some("kq"));
    quote["kind"] = json!("proforma");
    quote["customer_id"] = json!(customer);
    let (status, body) = call(&app, "POST", "/sales", Some(quote)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");
}
