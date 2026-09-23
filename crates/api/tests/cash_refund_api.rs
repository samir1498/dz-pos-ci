// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// S7 of `a-kernel-crate-and-retail-as-the-first-module`: a refund is a sale
// cancellation, so this whole file has nothing to compile with the feature
// off.
#![cfg(feature = "retail")]

//! Cash handed back, over HTTP (features.md §1; ruling 5 of the 2026-09-20
//! loop). The API crate is the product's only entry point (architecture.md,
//! consequence of rule 2), so what these assert is the contract the desktop
//! reads.
//!
//! Every figure is written by hand from the rule and never read back off the
//! answer being checked.

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

/// A product through the API, so nothing here reaches past the routes.
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
            "qty_on_hand_milli": 10_000,
            "rate_bps": 0,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_i64().unwrap()
}

/// The seller block a facture may not be issued without (décret 05-468
/// art. 3 and 4).
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

/// A company buyer carrying the identifiers a facture asks of one.
async fn a_company(app: &axum::Router) -> i64 {
    seller_ready(app).await;
    let (status, body) = call(
        app,
        "POST",
        "/customers",
        Some(json!({
            "name": "Sarl Amine Distribution",
            "party_kind": "company",
            "phone": null,
            "address": "05 boulevard Krim Belkacem, Alger",
            "rc": "16/00-7654321 B 20",
            "nif": null,
            "nis": "098216007654321",
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

/// The day the server stamped a document with, so the cash position is asked
/// about the day the sale is actually on rather than about the day this test
/// was written.
fn day_of(document: &Value) -> String {
    document["issued_at"].as_str().unwrap()[..10].to_string()
}

/// A cancelled cash ticket with `refund: "cash"` puts the notes on the cash
/// position, and the sale stays in the takings of the day it was rung.
///
/// 300,00 is under the 300 DA the droit de timbre starts at, so the ticket
/// carries no stamp and the day nets to exactly nothing.
#[tokio::test]
async fn a_cancellation_asking_for_cash_puts_the_notes_on_the_cash_position() {
    let (_dir, app) = app();
    let p = product(&app, "Sucre", 30_000).await;
    let (status, sale) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 30_000,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{sale}");
    assert_eq!(sale["totals"]["stamp_centimes"], json!(0), "{sale}");
    assert_eq!(sale["totals"]["net_to_pay_centimes"], json!(30_000));
    let id = sale["id"].as_i64().unwrap();
    let day = day_of(&sale);

    let (status, cancelled) = call(
        &app,
        "POST",
        &format!("/sales/{id}/cancel"),
        Some(json!({ "reason": "article défectueux", "refund": "cash" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert_eq!(cancelled["status"], "cancelled");

    let (status, position) = call(&app, "GET", &format!("/cash?day={day}"), None).await;
    assert_eq!(status, StatusCode::OK, "{position}");
    // The sale stays where it was rung: the drawer did take that money.
    assert_eq!(position["cash_in"]["sales_centimes"], json!(30_000));
    // And 300,00 went back out again.
    assert_eq!(position["cash_out"]["refunds_centimes"], json!(30_000));
    assert_eq!(position["cash_centimes"], json!(0));
}

/// The same cancellation with no `refund` field settles the way it always
/// did: the goods go back, nothing says the drawer opened, and the sale drops
/// out of its day.
///
/// This is what `#[serde(default)]` buys — every body written before the
/// field existed keeps meaning what it meant.
#[tokio::test]
async fn a_cancel_body_with_no_refund_field_moves_no_cash_at_all() {
    let (_dir, app) = app();
    let p = product(&app, "Sucre", 30_000).await;
    let (_, sale) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 30_000,
        })),
    )
    .await;
    let id = sale["id"].as_i64().unwrap();
    let day = day_of(&sale);

    let (status, cancelled) = call(
        &app,
        "POST",
        &format!("/sales/{id}/cancel"),
        Some(json!({ "reason": "erreur de saisie" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");

    let (_, position) = call(&app, "GET", &format!("/cash?day={day}"), None).await;
    assert_eq!(position["cash_in"]["sales_centimes"], json!(0));
    assert_eq!(position["cash_out"]["refunds_centimes"], json!(0));
    assert_eq!(position["cash_centimes"], json!(0));
}

/// A refund the core refuses comes back as a 400-class answer naming the
/// field, never a 500. Cash on a credit sale is the case: its money is on the
/// customer's account and comes off it there.
#[tokio::test]
async fn cash_on_a_credit_sale_is_refused_on_the_field_and_not_with_a_500() {
    let (_dir, app) = app();
    let p = product(&app, "Ciment", 100_000).await;
    let customer = a_company(&app).await;
    let (status, sale) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "credit",
            "customer_id": customer,
            "kind": "facture",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{sale}");
    let id = sale["id"].as_i64().unwrap();

    let (status, refused) = call(
        &app,
        "POST",
        &format!("/sales/{id}/cancel"),
        Some(json!({ "reason": "commande annulée", "refund": "cash" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["field"], json!("refund"), "{refused}");

    // Nothing moved: the facture still stands.
    let (_, still) = call(&app, "GET", &format!("/sales/{id}"), None).await;
    assert_eq!(still["status"], "issued");
}

/// An avoir body may carry the field too, and a value the enum does not know
/// is refused rather than read as the ledger path.
#[tokio::test]
async fn an_avoir_takes_the_refund_field_and_refuses_a_value_it_does_not_know() {
    let (_dir, app) = app();
    let p = product(&app, "Ciment", 100_000).await;
    let customer = a_company(&app).await;
    // Paid over the counter, so there is nothing owed and cash may go back.
    let (status, sale) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 200_000,
            "customer_id": customer,
            "kind": "facture",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{sale}");
    let id = sale["id"].as_i64().unwrap();
    let day = day_of(&sale);
    // Typed from the fixture, not read off the answer: one unit at 1 000,00
    // with no TVA is 1 000,00 TTC, and the droit de timbre on that is 10
    // tranches of 100 DA at 1 DA each, 10,00. The customer handed 1 010,00
    // over and the credit note gives 1 000,00 back.
    assert_eq!(
        sale["totals"]["total_ttc_centimes"],
        json!(100_000),
        "{sale}"
    );
    assert_eq!(sale["totals"]["stamp_centimes"], json!(1_000), "{sale}");
    assert_eq!(
        sale["totals"]["net_to_pay_centimes"],
        json!(101_000),
        "{sale}"
    );

    let (status, refused) = call(
        &app,
        "POST",
        &format!("/sales/{id}/avoir"),
        Some(json!({ "lines": null, "reason": null, "refund": "cheque" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");

    let (status, credit) = call(
        &app,
        "POST",
        &format!("/sales/{id}/avoir"),
        Some(json!({ "lines": null, "reason": "retour", "refund": "cash" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{credit}");
    // A credit note never carries the stamp, so what went back is the
    // facture's figure less the droit de timbre it collected.
    assert_eq!(credit["totals"]["stamp_centimes"], json!(0));
    assert_eq!(credit["totals"]["net_to_pay_centimes"], json!(100_000));

    let (_, position) = call(&app, "GET", &format!("/cash?day={day}"), None).await;
    assert_eq!(position["cash_in"]["sales_centimes"], json!(101_000));
    assert_eq!(position["cash_out"]["refunds_centimes"], json!(100_000));
    // The shop keeps the 10,00 of stamp and nothing else.
    assert_eq!(position["cash_centimes"], json!(1_000));
}

/// A shift's report carries the refunds beside the takings, and the expected
/// figure is the one less the other.
#[tokio::test]
async fn a_shift_report_carries_the_refunds_and_subtracts_them_from_what_is_expected() {
    let (_dir, app) = app();
    let p = product(&app, "Sucre", 30_000).await;
    let (status, shift) = call(
        &app,
        "POST",
        "/till/shifts",
        Some(json!({ "opening_cash_centimes": 1_000_000 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{shift}");
    let shift_id = shift["id"].as_i64().unwrap();

    let (_, first) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 30_000,
        })),
    )
    .await;
    let (_, second) = call(
        &app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": p, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 30_000,
        })),
    )
    .await;
    let refunded = second["id"].as_i64().unwrap();
    let (status, cancelled) = call(
        &app,
        "POST",
        &format!("/sales/{refunded}/cancel"),
        Some(json!({ "reason": "retour", "refund": "cash" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert!(first["id"].as_i64().unwrap() != refunded);

    let (status, report) = call(&app, "GET", &format!("/till/shifts/{shift_id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    // Two cash tickets of 300,00 each were rung and one of them was handed
    // back, so the drawer is holding 10 000,00 + 300,00.
    assert_eq!(report["takings"]["sales_centimes"], json!(60_000));
    assert_eq!(report["refunds_centimes"], json!(30_000));
    assert_eq!(report["expected_centimes"], json!(1_030_000));
}
