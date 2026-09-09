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
async fn a_sale_of_another_shop_is_not_found_even_by_its_own_id() {
    // Rule 3, on the read path: the shop is the server's, and a document one
    // shop issued is not a document another shop can open by guessing an id.
    // Two routers over one file, the way two shops share one installation.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mine = dzpos_api::router(dzpos_api::AppState::open(&path, SHOP).unwrap(), &token());
    let p = product(&mine, "Sucre", 1_000, 1900).await;
    let (status, sale) = call(
        &mine,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 10_000,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{sale}");
    let id = sale["id"].as_i64().unwrap();

    let other = dzpos_api::router(dzpos_api::AppState::open(&path, 2).unwrap(), &token());
    let (status, body) = call(&other, "GET", &format!("/sales/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(code(&body), "not_found");
    // Not "forbidden" either: the other shop is not told the document exists.
    assert!(!body.to_string().contains("doc_ticket"), "{body}");

    // And the list stays its own.
    let (status, list) = call(&other, "GET", "/sales", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().map(Vec::len), Some(0));

    // The shop that issued it still reads it.
    let (status, ours) = call(&mine, "GET", &format!("/sales/{id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{ours}");
    assert_eq!(ours["id"], id);
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
async fn a_credit_sale_with_no_customer_is_422_on_the_customer_field() {
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
async fn a_line_whose_price_times_its_quantity_overflows_is_422_not_500() {
    // Both fields sit at the largest integer a JSON number carries, so the
    // body is well formed and the DTO accepts it. Their product is past i64
    // centimes. That is the caller's arithmetic, not a stored-file fault, so
    // it answers 422 with the field named and never 500.
    let (_dir, app) = app();
    let p = product(&app, "Sucre", 1_000, 1900).await;
    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{
                "product_id": p,
                "qty_milli": 9_007_199_254_740_991_i64,
                "unit_price_centimes": 9_007_199_254_740_991_i64,
            }],
            "payment_mode": "cash",
            "tendered_centimes": 9_007_199_254_740_991_i64,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "validation");
    let (_, list) = call(&app, "GET", "/sales", None).await;
    assert_eq!(list.as_array().unwrap().len(), 0);
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

/// A fiche through the API, so the test never reaches past the routes.
async fn customer(
    app: &axum::Router,
    name: &str,
    credit_limit: Option<i64>,
    warn_threshold: Option<i64>,
) -> i64 {
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
            "credit_limit_centimes": credit_limit,
            "warn_threshold_centimes": warn_threshold,
            "notes": null,
            "active": true,
            "opening_debt_centimes": null,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_i64().unwrap()
}

#[tokio::test]
async fn a_credit_sale_answers_the_balance_triple_and_the_ledger_carries_the_document() {
    let (_dir, app) = app();
    let p = product(&app, "Ciment", 100_000, 0).await;
    let c = customer(&app, "Entreprise Amrani", Some(1_000_000), None).await;
    let (status, sale) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 2_000 }],
            "payment_mode": "credit",
            "customer_id": c,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{sale}");
    assert_eq!(sale["customer_id"].as_i64(), Some(c));
    assert_eq!(sale["payment_mode"], "credit");
    assert_eq!(sale["totals"]["stamp_centimes"], 0, "credit pays no stamp");
    assert_eq!(sale["balance"]["old_balance_centimes"], 0);
    assert_eq!(sale["balance"]["remaining_debt_centimes"], 200_000);
    assert_eq!(sale["balance"]["total_debt_centimes"], 200_000);
    assert_eq!(sale["warning"], Value::Null, "no threshold, no warning");

    let (status, ledger) = call(&app, "GET", &format!("/customers/{c}/ledger"), None).await;
    assert_eq!(status, StatusCode::OK, "{ledger}");
    assert_eq!(ledger["balance_centimes"], 200_000);
    let entry = &ledger["entries"][0];
    assert_eq!(entry["kind"], "sale");
    assert_eq!(entry["debit_centimes"], 200_000);
    assert_eq!(entry["document_id"].as_i64(), sale["id"].as_i64());
}

#[tokio::test]
async fn a_sale_past_the_credit_limit_is_422_carrying_both_amounts_and_the_override_takes_it() {
    let (_dir, app) = app();
    let p = product(&app, "Ciment", 100_000, 0).await;
    let c = customer(&app, "Entreprise Amrani", Some(50_000), None).await;
    let body = json!({
        "lines": [{ "product_id": p, "qty_milli": 1_000 }],
        "payment_mode": "credit",
        "customer_id": c,
    });
    let (status, refused) = call(&app, "POST", "/sales", Some(body.clone())).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(code(&refused), "credit_limit");
    // The two amounts the till has to show. They are on the envelope because
    // a screen may not re-derive what a customer owes (architecture.md rule 2).
    assert_eq!(refused["error"]["balance_after_centimes"], 100_000);
    assert_eq!(refused["error"]["credit_limit_centimes"], 50_000);
    // Nothing was written, so no number was taken.
    let (_, sales) = call(&app, "GET", "/sales", None).await;
    assert_eq!(sales.as_array().map(Vec::len), Some(0));

    let mut passed = body;
    passed["override"] = json!(true);
    let (status, sale) = call(&app, "POST", "/sales", Some(passed)).await;
    assert_eq!(status, StatusCode::CREATED, "{sale}");
    assert_eq!(sale["number"], 1, "the refusal burned no number");
    assert_eq!(sale["balance"]["total_debt_centimes"], 100_000);
}

#[tokio::test]
async fn a_sale_that_reaches_the_warn_threshold_answers_created_with_the_warning() {
    let (_dir, app) = app();
    let p = product(&app, "Sac", 10_000, 0).await;
    let c = customer(&app, "Entreprise Amrani", Some(100_000), Some(40_000)).await;
    let (status, sale) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 4_000 }],
            "payment_mode": "credit",
            "customer_id": c,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{sale}");
    assert_eq!(sale["warning"], "near_limit");
    // Read back, the document carries no warning: it was about the moment
    // the sale was rung up, not about the paper.
    let id = sale["id"].as_i64().unwrap();
    let (_, stored) = call(&app, "GET", &format!("/sales/{id}"), None).await;
    assert_eq!(stored["warning"], Value::Null);
}

#[tokio::test]
async fn a_card_sale_with_a_customer_names_the_buyer_and_writes_no_debt_row() {
    let (_dir, app) = app();
    let p = product(&app, "Sac", 10_000, 0).await;
    // No credit at all, which is not a rule about a sale that is paid for.
    let c = customer(&app, "Entreprise Amrani", Some(0), Some(0)).await;
    let (status, sale) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "card",
            "customer_id": c,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{sale}");
    assert_eq!(sale["customer_id"].as_i64(), Some(c));
    assert_eq!(sale["balance"]["remaining_debt_centimes"], 0);
    assert_eq!(sale["warning"], Value::Null);

    let (_, ledger) = call(&app, "GET", &format!("/customers/{c}/ledger"), None).await;
    assert_eq!(ledger["balance_centimes"], 0);
    assert_eq!(ledger["entries"].as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn an_error_that_is_not_a_credit_refusal_carries_neither_amount() {
    let (_dir, app) = app();
    let (status, body) = call(&app, "GET", "/sales/404", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let payload = body["error"].as_object().unwrap();
    assert_eq!(
        payload.keys().collect::<Vec<_>>(),
        vec!["code", "message"],
        "every other error kept the two-field envelope"
    );
}
