// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The stock recount over HTTP. The API is the product's only entry point
//! (architecture.md, consequence of rule 2), so what these assert is the
//! contract the settings panel reads: what a run answers, what the last run
//! answers before and after one, and that a sale leaves nothing to correct.

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
    path: std::path::PathBuf,
}

fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    Harness {
        _dir: dir,
        app: dzpos_api::router(state, &token()),
        path,
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

async fn a_product(app: &axum::Router, name: &str, stock_milli: i64) -> i64 {
    let (status, body) = call(
        app,
        "POST",
        "/products",
        Some(json!({
            "name": name,
            "barcode": null,
            "category_id": 1,
            "unit": "piece",
            "cost_centimes": 8_000,
            "selling_centimes": 12_000,
            "wholesale_centimes": null,
            "qty_on_hand_milli": stock_milli,
            "low_stock_at_milli": 0,
            "rate_bps": 1900,
            "active": true,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_i64().unwrap()
}

/// A cached quantity no movement explains, written straight into the file the
/// server is answering from. There is no route that writes the cache (that is
/// the point of the ledger), so a forgery is the only way to give the recount
/// something to correct.
fn forge_cache(path: &std::path::Path, product_id: i64, qty_milli: i64) {
    use diesel::prelude::*;
    let mut conn = SqliteConnection::establish(&path.display().to_string()).unwrap();
    diesel::sql_query(format!(
        "UPDATE products SET qty_on_hand_milli = {qty_milli} WHERE id = {product_id}"
    ))
    .execute(&mut conn)
    .unwrap();
}

/// Today as the API writes it, read from the route that owns the shop's
/// calendar rather than from a second clock in this file.
async fn today(app: &axum::Router) -> String {
    let (status, body) = call(app, "GET", "/clock", None).await;
    assert_eq!(status, StatusCode::OK);
    body["today"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn a_shop_that_has_never_recounted_answers_no_day_and_no_drift() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/stock/recount", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["last_run_day"], Value::Null);
    assert_eq!(body["drifts"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn a_run_corrects_the_forged_cache_and_the_last_run_reads_it_back() {
    let h = harness();
    let id = a_product(&h.app, "Sucre 1kg", 24_000).await;
    a_product(&h.app, "Farine 1kg", 3_000).await;
    forge_cache(&h.path, id, 99_000);

    let (status, run) = call(&h.app, "POST", "/stock/recount", None).await;
    assert_eq!(status, StatusCode::OK, "{run}");
    assert_eq!(run["day"], today(&h.app).await);
    assert_eq!(run["products_checked"], 2);
    let drifts = run["drifts"].as_array().unwrap();
    assert_eq!(drifts.len(), 1);
    assert_eq!(drifts[0]["product_id"], id);
    assert_eq!(drifts[0]["name"], "Sucre 1kg");
    assert_eq!(drifts[0]["cached_milli"], 99_000);
    assert_eq!(drifts[0]["ledger_milli"], 24_000);
    assert_eq!(drifts[0]["difference_milli"], -75_000);

    // The cache was put right, so the products screen shows the ledger.
    let (status, product) = call(&h.app, "GET", &format!("/products/{id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{product}");
    assert_eq!(product["qty_on_hand_milli"], 24_000);

    let (status, last) = call(&h.app, "GET", "/stock/recount", None).await;
    assert_eq!(status, StatusCode::OK, "{last}");
    assert_eq!(last["last_run_day"], today(&h.app).await);
    assert_eq!(last["drifts"], run["drifts"]);
}

#[tokio::test]
async fn a_sale_leaves_the_recount_nothing_to_correct() {
    // The ordinary case, and the one the e2e drives: every movement the till
    // writes moves the cache with it, so a shop that has only traded drifts
    // by nothing at all.
    let h = harness();
    let id = a_product(&h.app, "Sucre 1kg", 24_000).await;
    let (status, sale) = call(
        &h.app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": id, "qty_milli": 2_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 30_000,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{sale}");

    let (status, run) = call(&h.app, "POST", "/stock/recount", None).await;
    assert_eq!(status, StatusCode::OK, "{run}");
    assert_eq!(run["products_checked"], 1);
    assert_eq!(run["drifts"].as_array().unwrap().len(), 0);

    let (status, last) = call(&h.app, "GET", "/stock/recount", None).await;
    assert_eq!(status, StatusCode::OK, "{last}");
    assert_eq!(last["last_run_day"], run["day"]);
    assert_eq!(last["drifts"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn the_route_takes_no_other_method() {
    let h = harness();
    let (status, body) = call(&h.app, "DELETE", "/stock/recount", None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{body}");
}
