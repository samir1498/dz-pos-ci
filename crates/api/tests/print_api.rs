// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The printed ticket over HTTP. The core renders it; this route hands the
//! bytes over and decides nothing about them, so the test compares the
//! answer to `render_ticket` of the stored document rather than to a
//! fixture of its own.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use diesel::connection::SimpleConnection;
use dzpos_core::lang::Lang;
use dzpos_core::print::render_ticket;
use dzpos_core::services::documents;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";
const HTML: &str = "text/html; charset=utf-8";

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).unwrap()
}

fn app() -> (tempfile::TempDir, std::path::PathBuf, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    let router = dzpos_api::router(state, &token());
    (dir, path, router)
}

/// The raw body and its content type. The JSON helper the other suites use
/// would parse an HTML page into `Value::Null` and prove nothing.
async fn call_text(
    app: &axum::Router,
    uri: &str,
    bearer: bool,
) -> (StatusCode, Option<String>, String) {
    let req = Request::builder().method("GET").uri(uri);
    let req = if bearer {
        req.header("authorization", format!("Bearer {TOKEN}"))
    } else {
        req
    };
    let res = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let content_type = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        content_type,
        String::from_utf8(bytes.to_vec()).unwrap(),
    )
}

fn error_code(body: &str) -> String {
    let parsed: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    parsed["error"]["code"].as_str().unwrap_or("").to_owned()
}

/// One cash sale through the routes, so the test never reaches past them to
/// make the document it then prints.
async fn a_sale(app: &axum::Router) -> i64 {
    let req = Request::builder()
        .method("POST")
        .uri("/products")
        .header("authorization", format!("Bearer {TOKEN}"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Café moulu 250 g",
                "unit": "piece",
                "cost_centimes": 9_000,
                "selling_centimes": 15_000,
                "qty_on_hand_milli": 10_000,
                "rate_bps": 1900,
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let product: Value = serde_json::from_slice(&bytes).unwrap();
    let product_id = product["id"].as_i64().unwrap();

    let req = Request::builder()
        .method("POST")
        .uri("/sales")
        .header("authorization", format!("Bearer {TOKEN}"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "payment_mode": "cash",
                "tendered_centimes": 100_000,
                "lines": [{ "product_id": product_id, "qty_milli": 2_000 }],
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let sale: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status, StatusCode::CREATED, "{sale}");
    sale["id"].as_i64().unwrap()
}

#[tokio::test]
async fn a_ticket_is_the_html_the_core_renders_for_the_stored_document() {
    let (_dir, path, app) = app();
    let id = a_sale(&app).await;

    for lang in Lang::ALL {
        let (status, content_type, body) = call_text(
            &app,
            &format!("/sales/{id}/ticket?lang={}", lang.tag()),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(content_type.as_deref(), Some(HTML), "{lang:?}");

        // The same document, read back through the core on its own
        // connection: the route is a courier and this is the proof.
        let mut conn = dzpos_core::db::open(&path).unwrap();
        let stored = documents::get(&mut conn, SHOP, i32::try_from(id).unwrap()).unwrap();
        assert_eq!(body, render_ticket(&stored, lang).unwrap(), "{lang:?}");
    }
}

#[tokio::test]
async fn a_language_the_app_does_not_print_is_422_in_the_envelope() {
    let (_dir, _path, app) = app();
    let id = a_sale(&app).await;
    for uri in [
        format!("/sales/{id}/ticket?lang=es"),
        format!("/sales/{id}/ticket?lang=FR"),
        format!("/sales/{id}/ticket"),
    ] {
        let (status, content_type, body) = call_text(&app, &uri, true).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{uri}: {body}");
        assert_eq!(error_code(&body), "bad_request", "{uri}");
        assert_ne!(content_type.as_deref(), Some(HTML), "{uri}");
    }
}

#[tokio::test]
async fn a_ticket_for_a_sale_that_does_not_exist_is_404_in_the_envelope() {
    let (_dir, _path, app) = app();
    let (status, _, body) = call_text(&app, "/sales/404/ticket?lang=fr", true).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error_code(&body), "not_found");
}

/// Rule 3: every query is scoped by shop. The server answers for one shop
/// and a document of another one is not its to print, even when the id is
/// real.
#[tokio::test]
async fn a_ticket_of_another_shops_document_is_404() {
    let (_dir, path, app) = app();
    let id = a_sale(&app).await;

    let mut conn = dzpos_core::db::open(&path).unwrap();
    conn.batch_execute(
        "INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin');
         INSERT INTO documents (
             shop_id, kind, series, number, issued_at, user_id, regime,
             payment_mode, seller_name, total_ht_centimes, discount_centimes,
             subtotal_ht_centimes, tva_centimes, total_ttc_centimes,
             stamp_centimes, net_to_pay_centimes, status, created_at)
         SELECT 2, kind, series, number, issued_at, user_id, regime,
             payment_mode, seller_name, total_ht_centimes, discount_centimes,
             subtotal_ht_centimes, tva_centimes, total_ttc_centimes,
             stamp_centimes, net_to_pay_centimes, status, created_at
         FROM documents WHERE shop_id = 1;",
    )
    .unwrap();
    // The row is real and readable, for the shop that owns it. Without this
    // the 404 below would prove nothing.
    let other = i32::try_from(id).unwrap() + 1;
    assert!(documents::get(&mut conn, 2, other).is_ok());

    let (status, _, body) = call_text(&app, &format!("/sales/{other}/ticket?lang=fr"), true).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(error_code(&body), "not_found");
}

#[tokio::test]
async fn the_ticket_route_refuses_a_call_without_the_launch_token() {
    let (_dir, _path, app) = app();
    let id = a_sale(&app).await;
    let (status, _, body) = call_text(&app, &format!("/sales/{id}/ticket?lang=fr"), false).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(error_code(&body), "unauthorized");
}
