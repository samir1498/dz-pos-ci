// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The API crate is the product's only entry point, so these are the
//! product's tests (architecture.md, consequence of rule 2). In-process
//! router, real temp SQLite file.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const SHOP: i32 = 1;

struct Harness {
    _dir: tempfile::TempDir,
    app: axum::Router,
}

fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    Harness {
        _dir: dir,
        app: dzpos_api::router(state),
    }
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let req = Request::builder().method(method).uri(uri);
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

#[tokio::test]
async fn health_is_ok() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn products_start_empty() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/products", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn create_then_list_one() {
    let h = harness();
    let (status, made) = call(&h.app, "POST", "/products", Some(draft())).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["name"], "Huile Elio 5L");
    assert_eq!(made["selling_centimes"], 920);
    assert_eq!(made["rate_bps"], 1900);
    assert!(made["id"].is_number());

    let (status, list) = call(&h.app, "GET", "/products", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().map(Vec::len), Some(1));
    assert_eq!(list[0]["id"], made["id"]);
}

#[tokio::test]
async fn money_crosses_the_wire_as_an_integer_number_of_centimes() {
    // architecture.md, contract between Rust and TypeScript.
    let h = harness();
    let mut d = draft();
    d["selling_centimes"] = json!(1_234_567);
    let (_, made) = call(&h.app, "POST", "/products", Some(d)).await;
    assert_eq!(made["selling_centimes"], json!(1_234_567));
    assert!(
        made["selling_centimes"].is_i64(),
        "not an integer JSON number"
    );
}

#[tokio::test]
async fn a_duplicate_barcode_is_409() {
    let h = harness();
    let mut d = draft();
    d["barcode"] = json!("6130001000018");
    let (status, _) = call(&h.app, "POST", "/products", Some(d.clone())).await;
    assert_eq!(status, StatusCode::CREATED);

    d["name"] = json!("Autre");
    let (status, body) = call(&h.app, "POST", "/products", Some(d)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "duplicate_barcode");
    assert!(body["error"]["message"].is_string());
}

#[tokio::test]
async fn a_validation_failure_is_422() {
    let h = harness();
    let mut d = draft();
    d["name"] = json!("   ");
    let (status, body) = call(&h.app, "POST", "/products", Some(d)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "validation");

    let mut e = draft();
    e["selling_centimes"] = json!(-1);
    let (status, body) = call(&h.app, "POST", "/products", Some(e)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "validation");
}

#[tokio::test]
async fn a_rate_above_one_whole_is_422_not_a_panic() {
    let h = harness();
    let mut d = draft();
    d["rate_bps"] = json!(190_000);
    let (status, body) = call(&h.app, "POST", "/products", Some(d)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "validation");
}

#[tokio::test]
async fn a_malformed_body_is_422_with_the_same_error_shape() {
    let h = harness();
    let (status, body) = call(&h.app, "POST", "/products", Some(json!({ "name": 3 }))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "bad_request");
    assert!(body["error"]["message"].is_string());
}

#[tokio::test]
async fn an_unknown_unit_is_rejected_before_it_reaches_the_database() {
    let h = harness();
    let mut d = draft();
    d["unit"] = json!("barrel");
    let (status, body) = call(&h.app, "POST", "/products", Some(d)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "bad_request");
}

#[tokio::test]
async fn an_unknown_route_is_404_with_the_same_error_shape() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
}

#[tokio::test]
async fn every_error_body_has_a_code_and_a_message() {
    // The UI translates the code; the message is for a log, never a screen.
    let h = harness();
    let mut d = draft();
    d["category_id"] = json!(4242);
    let (status, body) = call(&h.app, "POST", "/products", Some(d)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
    assert!(body["error"]["message"].is_string());
    assert_eq!(body.as_object().map(|o| o.len()), Some(1));
}

#[tokio::test]
async fn the_server_only_ever_answers_for_its_own_shop() {
    // Rule 3: the shop is the server's, never the caller's to choose.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mine = dzpos_api::router(dzpos_api::AppState::open(&path, SHOP).unwrap());
    let (status, _) = call(&mine, "POST", "/products", Some(draft())).await;
    assert_eq!(status, StatusCode::CREATED);

    let other = dzpos_api::router(dzpos_api::AppState::open(&path, 2).unwrap());
    let (_, list) = call(&other, "GET", "/products", None).await;
    assert_eq!(list.as_array().map(Vec::len), Some(0));

    // A shop_id in the query string changes nothing.
    let (_, list) = call(&other, "GET", "/products?shop_id=1", None).await;
    assert_eq!(list.as_array().map(Vec::len), Some(0));
}
