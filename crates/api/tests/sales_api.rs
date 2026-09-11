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

mod common;

use common::{printed, series_of};

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
    assert_eq!(body["series"], series_of("doc_ticket"));
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
    let mine = common::signed_in_router(&path, SHOP, &token());
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

    let other = common::signed_in_router(&path, 2, &token());
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

// ---- the facture at the till (features.md §3)

/// The shop's own block as a facture needs it, through the route a shop
/// owner would use.
async fn seller_ready(app: &axum::Router) {
    let (status, body) = call(
        app,
        "PUT",
        "/settings/store",
        Some(json!({
            "name": "Supérette El Bahdja",
            "rc": "16/00-1234567 B 21",
            "nif": "000216001234567",
            "nis": "098216001234567",
            "ai": "16123456789",
            "address": "Rue Didouche Mourad, Alger",
            "phone": "021 00 00 00",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// A fiche with only the identifiers the caller hands over, so a test can
/// take one away and see which refusal it earns.
async fn party(
    app: &axum::Router,
    name: &str,
    party_kind: &str,
    rc: Option<&str>,
    nis: Option<&str>,
    address: Option<&str>,
) -> i64 {
    let (status, body) = call(
        app,
        "POST",
        "/customers",
        Some(json!({
            "name": name,
            "party_kind": party_kind,
            "phone": null,
            "address": address,
            "rc": rc,
            "nif": null,
            "nis": nis,
            "ai": null,
            "credit_limit_centimes": null,
            "warn_threshold_centimes": null,
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
async fn the_kind_crosses_the_wire_and_a_facture_numbers_in_its_own_series() {
    let (_dir, app) = app();
    seller_ready(&app).await;
    let p = product(&app, "Ciment", 100_000, 0).await;
    let c = party(
        &app,
        "Entreprise Amrani",
        "company",
        Some("16/00-7654321 B 22"),
        Some("098216007654321"),
        None,
    )
    .await;

    // No `kind` at all: the field defaults to a ticket, so a caller written
    // before the switch existed keeps issuing what it always did.
    let (status, ticket) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 200_000,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{ticket}");
    assert_eq!(ticket["kind"], json!("ticket"));
    assert_eq!(ticket["series"], json!(series_of("doc_ticket")));
    assert_eq!(ticket["number"], json!(1));

    let (status, facture) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "credit",
            "customer_id": c,
            "kind": "facture",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{facture}");
    assert_eq!(facture["kind"], json!("facture"));
    assert_eq!(facture["series"], json!(series_of("doc_facture")));
    assert_eq!(facture["number"], json!(1), "its own series starts at one");

    // The read back says the same: the kind is stored, not answered once.
    let id = facture["id"].as_i64().unwrap();
    let (_, again) = call(&app, "GET", &format!("/sales/{id}"), None).await;
    assert_eq!(again["kind"], json!("facture"));
    assert_eq!(again["customer_id"], json!(c));
}

#[tokio::test]
async fn a_facture_the_party_blocks_refuse_is_422_naming_the_side_and_the_fields() {
    let (_dir, app) = app();
    let p = product(&app, "Ciment", 100_000, 0).await;
    let c = party(
        &app,
        "Entreprise Amrani",
        "company",
        Some("16/00-7654321 B 22"),
        Some("098216007654321"),
        None,
    )
    .await;
    let facture = json!({
        "lines": [{ "product_id": p, "qty_milli": 1_000 }],
        "payment_mode": "cash",
        "tendered_centimes": 200_000,
        "customer_id": c,
        "kind": "facture",
    });

    // The shop has filled nothing in yet, so the seller is the side that
    // refuses and the cashier is sent to the settings.
    let (status, body) = call(&app, "POST", "/sales", Some(facture.clone())).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "party_ids");
    assert_eq!(body["error"]["party_side"], json!("seller"));
    assert_eq!(body["error"]["missing_ids"], json!(["rc", "nis"]));

    seller_ready(&app).await;
    let bare = party(&app, "Sarl Bendiba", "company", None, None, None).await;
    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 200_000,
            "customer_id": bare,
            "kind": "facture",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "party_ids");
    assert_eq!(body["error"]["party_side"], json!("buyer"));
    assert_eq!(body["error"]["missing_ids"], json!(["rc", "nis"]));

    // A facture with nobody to make it out to is the customer field, not a
    // block that is short: there is no block at all.
    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 200_000,
            "kind": "facture",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "validation");

    // Nothing above took a number: the facture series is still untouched
    // and the first one that goes through is FA number 1.
    let (status, ok) = call(&app, "POST", "/sales", Some(facture)).await;
    assert_eq!(status, StatusCode::CREATED, "{ok}");
    assert_eq!(ok["number"], json!(1));
    assert_eq!(ok["series"], json!(series_of("doc_facture")));
}

#[tokio::test]
async fn a_consumer_buying_on_a_facture_is_asked_for_an_address_and_not_for_an_rc() {
    let (_dir, app) = app();
    seller_ready(&app).await;
    let p = product(&app, "Ciment", 100_000, 0).await;
    let named = party(
        &app,
        "Karim Belkacem",
        "consumer",
        None,
        None,
        Some("12 rue des Frères Bouadou, Bir Mourad Raïs"),
    )
    .await;
    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 200_000,
            "customer_id": named,
            "kind": "facture",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let nowhere = party(&app, "Yacine Hamdi", "consumer", None, None, None).await;
    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 200_000,
            "customer_id": nowhere,
            "kind": "facture",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "party_ids");
    assert_eq!(body["error"]["party_side"], json!("buyer"));
    assert_eq!(body["error"]["missing_ids"], json!(["address"]));
}

#[tokio::test]
async fn a_kind_the_till_cannot_ring_up_is_refused_at_the_edge() {
    let (_dir, app) = app();
    let p = product(&app, "Ciment", 100_000, 0).await;
    // `avoir` is a document kind and not a sale kind: it has its own write,
    // its own rules and its own series, and the till may not reach it here.
    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 200_000,
            "kind": "avoir",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "bad_request");
}

#[tokio::test]
async fn the_list_carries_both_kinds_newest_first_and_the_filter_narrows_it() {
    let (_dir, app) = app();
    seller_ready(&app).await;
    let p = product(&app, "Ciment", 100_000, 0).await;
    let c = party(
        &app,
        "Entreprise Amrani",
        "company",
        Some("16/00-7654321 B 22"),
        Some("098216007654321"),
        None,
    )
    .await;
    let basket = json!({
        "lines": [{ "product_id": p, "qty_milli": 1_000 }],
        "payment_mode": "cash",
        "tendered_centimes": 200_000,
    });
    let (_, ticket) = call(&app, "POST", "/sales", Some(basket)).await;
    let (_, facture) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "credit",
            "customer_id": c,
            "kind": "facture",
        })),
    )
    .await;

    // Everything the till issued, the last one first. A facture that fell
    // off this list would be unreachable the moment the print panel closed.
    let (status, all) = call(&app, "GET", "/sales", None).await;
    assert_eq!(status, StatusCode::OK, "{all}");
    let rows = all.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], facture["id"]);
    assert_eq!(rows[0]["kind"], json!("facture"));
    assert_eq!(rows[0]["printed_number"], json!(printed("FA", 1)));
    assert_eq!(rows[1]["id"], ticket["id"]);
    assert_eq!(rows[1]["kind"], json!("ticket"));
    assert_eq!(rows[1]["printed_number"], json!(printed("TK", 1)));

    for (kind, expected) in [("ticket", &ticket), ("facture", &facture)] {
        let (status, only) = call(&app, "GET", &format!("/sales?kind={kind}"), None).await;
        assert_eq!(status, StatusCode::OK, "{only}");
        let rows = only.as_array().unwrap();
        assert_eq!(rows.len(), 1, "{kind}: {only}");
        assert_eq!(rows[0]["id"], expected["id"]);
        assert_eq!(rows[0]["kind"], json!(kind));
    }
}

#[tokio::test]
async fn a_kind_the_till_never_issues_is_refused_rather_than_ignored() {
    // A filter the server does not understand is not an empty filter: a
    // screen asking for `avoir` and being handed every document would be
    // showing the wrong list with no way to tell.
    let (_dir, app) = app();
    for uri in ["/sales?kind=avoir", "/sales?kind=Ticket", "/sales?kind="] {
        let (status, body) = call(&app, "GET", uri, None).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{uri}: {body}");
        assert_eq!(code(&body), "bad_request", "{uri}");
    }
}

/// A facture on credit to a company that carries its identifiers, which is
/// what an avoir and a cancellation are written against.
async fn a_credit_facture(app: &axum::Router, product_id: i64, qty_milli: i64) -> Value {
    seller_ready(app).await;
    let c = party(
        app,
        "Sarl Amine Distribution",
        "company",
        Some("16/00-7654321 B 20"),
        Some("098216007654321"),
        Some("05 boulevard Krim Belkacem, Alger"),
    )
    .await;
    let (status, body) = call(
        app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": product_id, "qty_milli": qty_milli }],
            "payment_mode": "credit",
            "customer_id": c,
            "kind": "facture",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body
}

#[tokio::test]
async fn an_avoir_answers_201_naming_the_facture_it_credits_and_lists_under_it() {
    let (_dir, app) = app();
    let p = product(&app, "Ciment", 100_000, 0).await;
    let facture = a_credit_facture(&app, p, 3_000).await;
    let facture_id = facture["id"].as_i64().unwrap();
    let line_id = facture["lines"][0]["id"].as_i64().unwrap();

    // One line of 500,00 back out of 3 000,00.
    let (status, avoir) = call(
        &app,
        "POST",
        &format!("/sales/{facture_id}/avoir"),
        Some(json!({
            "lines": [{ "document_line_id": line_id, "qty_milli": 500 }],
            "reason": "retour marchandise",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{avoir}");
    assert_eq!(avoir["kind"], "avoir");
    assert_eq!(avoir["ref_document_id"], facture_id);
    assert_eq!(avoir["totals"]["net_to_pay_centimes"], 50_000);
    // An avoir never carries the droit de timbre.
    assert_eq!(avoir["totals"]["stamp_centimes"], 0);
    assert_eq!(avoir["lines"][0]["ref_line_id"], line_id);
    assert!(avoir["printed_number"].as_str().unwrap().starts_with("AV-"));

    // The facture now asks for what is left on it.
    let (status, back) = call(&app, "GET", &format!("/sales/{facture_id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{back}");
    assert_eq!(back["balance"]["remaining_debt_centimes"], 250_000);

    // And the avoir is listed under the facture it credits.
    let (status, listed) = call(&app, "GET", &format!("/sales/{facture_id}/avoirs"), None).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["id"], avoir["id"]);
}

#[tokio::test]
async fn an_avoir_on_a_ticket_is_404_and_one_past_the_line_is_422() {
    let (_dir, app) = app();
    let p = product(&app, "Ciment", 100_000, 0).await;
    let (status, ticket) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 1_000_000,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{ticket}");
    let ticket_id = ticket["id"].as_i64().unwrap();
    let (status, body) = call(
        &app,
        "POST",
        &format!("/sales/{ticket_id}/avoir"),
        Some(json!({ "lines": null, "reason": null })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(code(&body), "not_found");

    let facture = a_credit_facture(&app, p, 1_000).await;
    let facture_id = facture["id"].as_i64().unwrap();
    let line_id = facture["lines"][0]["id"].as_i64().unwrap();
    let (status, body) = call(
        &app,
        "POST",
        &format!("/sales/{facture_id}/avoir"),
        Some(json!({
            "lines": [{ "document_line_id": line_id, "qty_milli": 5_000 }],
            "reason": null,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    // The envelope of a validation refusal is its code and its message: the
    // field a core error names is not on the wire (crates/api/src/error.rs
    // fills `field` only where a screen has a box to put it in), so what is
    // asserted here is the refusal and the message the till shows.
    assert_eq!(code(&body), "validation");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("credited"),
        "{body}"
    );
}

#[tokio::test]
async fn cancelling_a_credit_facture_answers_the_block_naming_the_avoir() {
    let (_dir, app) = app();
    let p = product(&app, "Ciment", 100_000, 0).await;
    let facture = a_credit_facture(&app, p, 2_000).await;
    let facture_id = facture["id"].as_i64().unwrap();
    let customer_id = facture["customer_id"].as_i64().unwrap();

    let (status, cancelled) = call(
        &app,
        "POST",
        &format!("/sales/{facture_id}/cancel"),
        Some(json!({ "reason": "commande annulée" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert_eq!(cancelled["status"], "cancelled");
    // The number stays, so the series never gaps.
    assert_eq!(cancelled["number"], facture["number"]);
    assert_eq!(cancelled["cancellation"]["reason"], "commande annulée");
    // The whole block travels, not the reason alone: the annulée face of the
    // paper prints the day, and a comptable asking why a numbered document
    // stopped asking for its amount is owed who decided it.
    assert_eq!(
        cancelled["cancellation"]["cancelled_at"]
            .as_str()
            .expect("the moment it was annulled")
            .len(),
        19,
        "{cancelled}"
    );
    assert_eq!(cancelled["cancellation"]["cancelled_by"], 1);
    let avoir_id = cancelled["cancellation"]["avoir_document_id"]
        .as_i64()
        .expect("a facture carrying debt is cancelled through an avoir");

    let (status, avoir) = call(&app, "GET", &format!("/sales/{avoir_id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{avoir}");
    assert_eq!(avoir["kind"], "avoir");
    assert_eq!(avoir["totals"]["net_to_pay_centimes"], 200_000);

    // The customer owes nothing now.
    let (status, fiche) = call(&app, "GET", &format!("/customers/{customer_id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{fiche}");
    assert_eq!(fiche["balance_centimes"], 0);

    // A second cancellation is refused.
    let (status, body) = call(
        &app,
        "POST",
        &format!("/sales/{facture_id}/cancel"),
        Some(json!({ "reason": "encore" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "validation");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("annulée"),
        "{body}"
    );
}

#[tokio::test]
async fn a_proforma_crosses_the_wire_takes_its_own_series_and_moves_no_stock() {
    let (_dir, app) = app();
    seller_ready(&app).await;
    let p = product(&app, "Ciment", 100_000, 0).await;
    let c = party(&app, "Sarl Amine", "company", None, None, None).await;

    let (status, before) = call(&app, "GET", &format!("/products/{p}"), None).await;
    assert_eq!(status, StatusCode::OK, "{before}");
    let qty_before = before["qty_on_hand_milli"].as_i64().unwrap();

    let (status, quote) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 2_000 }],
            "payment_mode": "credit",
            "customer_id": c,
            "kind": "proforma",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{quote}");
    assert_eq!(quote["kind"], "proforma");
    assert_eq!(quote["number"], 1);
    assert!(quote["printed_number"].as_str().unwrap().starts_with("PF-"));
    assert_eq!(quote["balance"]["total_debt_centimes"], 0);

    // Nothing left the shelf and nobody owes anything.
    let (_, after) = call(&app, "GET", &format!("/products/{p}"), None).await;
    assert_eq!(after["qty_on_hand_milli"], qty_before);
    let (_, fiche) = call(&app, "GET", &format!("/customers/{c}"), None).await;
    assert_eq!(fiche["balance_centimes"], 0);

    // And a proforma with no customer is refused on the field.
    let (status, body) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "kind": "proforma",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "validation");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("customer"),
        "{body}"
    );
}

#[tokio::test]
async fn a_cancellation_with_no_reason_is_refused_on_the_field() {
    let (_dir, app) = app();
    let p = product(&app, "Ciment", 100_000, 0).await;
    let facture = a_credit_facture(&app, p, 1_000).await;
    let facture_id = facture["id"].as_i64().unwrap();
    let (status, body) = call(
        &app,
        "POST",
        &format!("/sales/{facture_id}/cancel"),
        Some(json!({ "reason": "   " })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(code(&body), "validation");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("reason"),
        "{body}"
    );
}

/// A field the type does not know is refused rather than dropped. The one
/// that matters is a misspelt `lines` on an avoir: dropped, it reads as the
/// whole facture coming back, so a caller asking for one unit of three would
/// have credited all three and never been told.
#[tokio::test]
async fn a_stray_field_is_refused_on_every_avoir_and_cancel_body() {
    let (_dir, app) = app();
    let p = product(&app, "Ciment", 100_000, 0).await;
    let facture = a_credit_facture(&app, p, 3_000).await;
    let facture_id = facture["id"].as_i64().unwrap();
    let line_id = facture["lines"][0]["id"].as_i64().unwrap();

    for body in [
        // The misspelling itself.
        json!({ "line": [{ "document_line_id": line_id, "qty_milli": 1_000 }], "reason": null }),
        // A stray beside a good body.
        json!({ "lines": null, "reason": null, "note": "retour" }),
        // And one inside a line.
        json!({
            "lines": [{ "document_line_id": line_id, "qty_milli": 1_000, "product_id": 1 }],
            "reason": null
        }),
    ] {
        let (status, answer) = call(
            &app,
            "POST",
            &format!("/sales/{facture_id}/avoir"),
            Some(body.clone()),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{body} was taken: {answer}"
        );
    }

    let (status, answer) = call(
        &app,
        "POST",
        &format!("/sales/{facture_id}/cancel"),
        Some(json!({ "reason": "erreur", "avoir": false })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{answer}");

    // Nothing was written by any of them: the facture still stands and no
    // credit note was taken out of the series.
    let (status, still) = call(&app, "GET", &format!("/sales/{facture_id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{still}");
    assert_eq!(still["status"], "issued");
    let (status, avoirs) = call(&app, "GET", &format!("/sales/{facture_id}/avoirs"), None).await;
    assert_eq!(status, StatusCode::OK, "{avoirs}");
    assert_eq!(avoirs.as_array().map(Vec::len), Some(0), "{avoirs}");
}

/// What a cancellation would do, answered by the core on a read of one
/// document. The screen cannot work it out from the fields beside it: a
/// facture credited in full still carries debt, was still sold on credit and
/// still names a customer, and cancelling it does nothing at all.
#[tokio::test]
async fn a_read_of_one_document_says_what_cancelling_it_would_do() {
    let (_dir, app) = app();
    let p = product(&app, "Ciment", 100_000, 0).await;

    // A cash ticket: the goods and nothing else.
    let (status, ticket) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 200_000,
            "kind": "ticket"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{ticket}");
    let ticket_id = ticket["id"].as_i64().unwrap();
    let (_, read) = call(&app, "GET", &format!("/sales/{ticket_id}"), None).await;
    assert_eq!(read["cancel_effect"]["effect"], "stock_back", "{read}");

    // A facture on credit: the goods and a credit note of the whole of it.
    let facture = a_credit_facture(&app, p, 3_000).await;
    let facture_id = facture["id"].as_i64().unwrap();
    let (_, read) = call(&app, "GET", &format!("/sales/{facture_id}"), None).await;
    assert_eq!(read["cancel_effect"]["effect"], "stock_back_and_avoir");
    assert_eq!(read["cancel_effect"]["amount_centimes"], 300_000);

    // Credited in part: the figure follows what is left.
    let line_id = facture["lines"][0]["id"].as_i64().unwrap();
    let (status, made) = call(
        &app,
        "POST",
        &format!("/sales/{facture_id}/avoir"),
        Some(json!({
            "lines": [{ "document_line_id": line_id, "qty_milli": 1_000 }],
            "reason": null
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    let (_, read) = call(&app, "GET", &format!("/sales/{facture_id}"), None).await;
    assert_eq!(read["cancel_effect"]["amount_centimes"], 200_000);

    // Credited in full: nothing left to undo.
    let (status, rest) = call(
        &app,
        "POST",
        &format!("/sales/{facture_id}/avoir"),
        Some(json!({ "lines": null, "reason": null })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{rest}");
    let (_, read) = call(&app, "GET", &format!("/sales/{facture_id}"), None).await;
    assert_eq!(
        read["cancel_effect"]["effect"], "nothing_to_reverse",
        "{read}"
    );

    // The list does not carry it: it is a question about one document and it
    // costs a read of that document's credit notes.
    let (_, listed) = call(&app, "GET", "/sales", None).await;
    assert!(listed[0]["cancel_effect"].is_null(), "{listed}");
}
