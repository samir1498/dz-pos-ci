// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The till's routes against an in-process router and a real temp SQLite
//! file. The API crate is the product's only entry point, so these are the
//! product's tests (architecture.md, consequence of rule 2).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).unwrap()
}

fn app() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
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

/// A product through the API, so the test never reaches past the routes.
async fn product(app: &axum::Router, name: &str, selling: i64, rate_bps: u32) -> i64 {
    let (status, body) = call(
        app,
        "POST",
        "/products",
        Some(json!({
            "name": name,
            "unit": "piece",
            "cost_centimes": selling / 2,
            "selling_centimes": selling,
            "qty_on_hand_milli": 10_000,
            "rate_bps": rate_bps,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_i64().unwrap()
}

fn code(body: &Value) -> &str {
    body["error"]["code"].as_str().unwrap_or("")
}

#[tokio::test]
async fn a_new_database_has_sold_nothing() {
    let (_dir, app) = app();
    let (status, body) = call(&app, "GET", "/sales", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn a_cash_sale_answers_201_with_the_document_its_lines_and_its_tva_rows() {
    let (_dir, app) = app();
    let p = product(&app, "Sucre", 11_000, 1900).await;
    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 2_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 30_000,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["kind"], "ticket");
    assert_eq!(body["series"], "doc_ticket");
    assert_eq!(body["number"], 1);
    assert_eq!(body["status"], "issued");
    assert_eq!(body["payment_mode"], "cash");
    assert_eq!(body["regime"], "reel");
    assert_eq!(body["totals"]["total_ht_centimes"], 22_000);
    assert_eq!(body["totals"]["tva_centimes"], 4_180);
    assert_eq!(body["totals"]["net_to_pay_centimes"], 26_180);
    assert_eq!(body["change_centimes"], 3_820);
    assert_eq!(body["lines"].as_array().unwrap().len(), 1);
    assert_eq!(body["lines"][0]["name"], "Sucre");
    assert_eq!(body["lines"][0]["qty_milli"], 2_000);
    assert_eq!(body["tva"].as_array().unwrap().len(), 1);
    assert_eq!(body["tva"][0]["rate_bps"], 1900);
    assert_eq!(body["seller"]["name"], "Mon magasin");
    // The server dates the document; the caller never sends it.
    assert!(body["issued_at"].as_str().unwrap().len() >= 19);

    // The stock left, and the product answers the new count.
    let (_, after) = call(&app, "GET", &format!("/products/{p}"), None).await;
    assert_eq!(after["qty_on_hand_milli"], 8_000);
}

#[tokio::test]
async fn a_sale_reads_back_and_lists_newest_first() {
    let (_dir, app) = app();
    let p = product(&app, "Sucre", 1_000, 1900).await;
    let basket = json!({
        "lines": [{ "product_id": p, "qty_milli": 1_000 }],
        "payment_mode": "cash",
        "tendered_centimes": 10_000,
    });
    let (_, first) = call(&app, "POST", "/sales", Some(basket.clone())).await;
    let (_, second) = call(&app, "POST", "/sales", Some(basket)).await;

    let (status, one) = call(
        &app,
        "GET",
        &format!("/sales/{}", first["id"].as_i64().unwrap()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(one["number"], 1);

    let (status, list) = call(&app, "GET", "/sales", None).await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<i64> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_i64().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec![
            second["id"].as_i64().unwrap(),
            first["id"].as_i64().unwrap()
        ]
    );
}

#[tokio::test]
async fn an_unknown_sale_is_404_in_the_envelope() {
    let (_dir, app) = app();
    let (status, body) = call(&app, "GET", "/sales/404", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(code(&body), "not_found");
}

#[tokio::test]
async fn an_unknown_product_on_a_line_is_404() {
    let (_dir, app) = app();
    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": 404, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 10_000,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(code(&body), "not_found");
}

#[tokio::test]
async fn an_inactive_product_is_422_with_the_validation_code() {
    let (_dir, app) = app();
    let p = product(&app, "Sucre", 1_000, 1900).await;
    let (status, body) = call(
        &app,
        "PUT",
        &format!("/products/{p}"),
        Some(json!({
            "name": "Sucre",
            "unit": "piece",
            "cost_centimes": 500,
            "selling_centimes": 1_000,
            "rate_bps": 1900,
            "active": false,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 10_000,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "validation");
}

#[tokio::test]
async fn a_credit_sale_is_422_until_customers_exist() {
    let (_dir, app) = app();
    let p = product(&app, "Sucre", 1_000, 1900).await;
    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "credit",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "validation");
}

#[tokio::test]
async fn cash_short_of_the_amount_to_pay_is_422_and_writes_nothing() {
    let (_dir, app) = app();
    let p = product(&app, "Sucre", 11_000, 1900).await;
    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 100,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "validation");
    let (_, list) = call(&app, "GET", "/sales", None).await;
    assert_eq!(list.as_array().unwrap().len(), 0);
    let (_, after) = call(&app, "GET", &format!("/products/{p}"), None).await;
    assert_eq!(after["qty_on_hand_milli"], 10_000);
}

#[tokio::test]
async fn a_body_the_type_does_not_know_is_422_bad_request() {
    let (_dir, app) = app();
    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [],
            "payment_mode": "cash",
            "surprise": 1,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "bad_request");

    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({ "lines": [], "payment_mode": "bitcoin" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "bad_request");
}

#[tokio::test]
async fn a_sale_id_that_is_not_a_number_answers_in_the_envelope() {
    let (_dir, app) = app();
    let (status, body) = call(&app, "GET", "/sales/abc", None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(code(&body), "bad_request");
}

#[tokio::test]
async fn the_till_routes_refuse_a_call_without_the_launch_token() {
    let (_dir, app) = app();
    for (method, uri) in [("GET", "/sales"), ("POST", "/sales"), ("GET", "/sales/1")] {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{method} {uri}");
    }
}

#[tokio::test]
async fn a_method_the_route_does_not_take_answers_405_in_the_envelope() {
    let (_dir, app) = app();
    let (status, body) = call(&app, "DELETE", "/sales", None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(code(&body), "method_not_allowed");
}
