// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The shelf label over HTTP. The core renders it, so these tests compare
//! the answer to `render_label` of the stored fiche rather than to a fixture
//! of their own: the goldens live in `dzpos-core`, and what is left here is
//! that the route reads the right product, in the right shop, and refuses
//! the same things the core refuses.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use dzpos_core::lang::Lang;
use dzpos_core::print::{render_label, render_label_sheet};
use dzpos_core::services::products;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";
const HTML: &str = "text/html; charset=utf-8";

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).unwrap()
}

struct Harness {
    _dir: tempfile::TempDir,
    state: dzpos_api::AppState,
    app: axum::Router,
}

fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    let app = dzpos_api::router(state.clone(), &token());
    Harness {
        _dir: dir,
        state,
        app,
    }
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Option<String>, String) {
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

fn a_product(name: &str, barcode: Option<&str>) -> Value {
    json!({
        "name": name,
        "barcode": barcode,
        "category_id": null,
        "unit": "piece",
        "cost_centimes": 20_000,
        "selling_centimes": 32_050,
        "wholesale_centimes": null,
        "qty_on_hand_milli": 12_000,
        "low_stock_at_milli": 3_000,
        "rate_bps": 1900,
        "active": true
    })
}

async fn create(app: &axum::Router, name: &str, barcode: Option<&str>) -> i32 {
    let (status, _, body) = call(app, "POST", "/products", Some(a_product(name, barcode))).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let value: Value = serde_json::from_str(&body).unwrap();
    i32::try_from(value["id"].as_i64().unwrap()).unwrap()
}

#[tokio::test]
async fn one_products_label_is_the_page_the_core_renders_in_each_language() {
    let h = harness();
    let id = create(&h.app, "Café moulu 250 g", Some("2000010000074")).await;

    for lang in Lang::ALL {
        let (status, content_type, body) = call(
            &h.app,
            "GET",
            &format!("/products/{id}/label?lang={}", lang.tag()),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(content_type.as_deref(), Some(HTML));

        let stored = h.state.with_conn(|c| products::get(c, SHOP, id)).unwrap();
        assert_eq!(body, render_label(&stored, lang).unwrap());
    }
}

#[tokio::test]
async fn a_product_with_no_ean13_and_an_id_this_shop_never_gave_out_both_refuse() {
    let h = harness();
    // A supplier's own reference typed into the fiche: no EAN-13 picture,
    // and a label with digits and no bars scans as nothing on a shelf.
    let typed = create(&h.app, "Sac de semoule", Some("REF-4471")).await;
    let (status, _, body) = call(
        &h.app,
        "GET",
        &format!("/products/{typed}/label?lang=fr"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["error"]["code"], json!("validation"));
    assert_eq!(value["error"]["field"], json!("barcode"));

    let (missing, _, _) = call(&h.app, "GET", "/products/4471/label?lang=fr", None).await;
    assert_eq!(missing, StatusCode::NOT_FOUND);

    let (bad_lang, _, _) = call(
        &h.app,
        "GET",
        &format!("/products/{typed}/label?lang=de"),
        None,
    )
    .await;
    assert_eq!(bad_lang, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn the_sheet_is_the_selection_in_the_order_it_was_named() {
    let h = harness();
    let first = create(&h.app, "Café moulu 250 g", Some("2000010000074")).await;
    let second = create(&h.app, "Pain de campagne", Some("6130001234563")).await;

    let (status, content_type, body) = call(
        &h.app,
        "POST",
        "/labels/sheet?lang=fr",
        Some(json!({ "ids": [second, first] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(content_type.as_deref(), Some(HTML));

    let stored = h
        .state
        .with_conn(|c| {
            Ok(vec![
                products::get(c, SHOP, second)?,
                products::get(c, SHOP, first)?,
            ])
        })
        .unwrap();
    assert_eq!(body, render_label_sheet(&stored, Lang::Fr).unwrap());

    // One id nobody in this shop owns takes the whole sheet down: a page
    // missing one label looks complete, and the product left off it is the
    // one somebody was looking for.
    let (missing, _, _) = call(
        &h.app,
        "POST",
        "/labels/sheet?lang=fr",
        Some(json!({ "ids": [first, 4471] })),
    )
    .await;
    assert_eq!(missing, StatusCode::NOT_FOUND);

    let (empty, _, _) = call(
        &h.app,
        "POST",
        "/labels/sheet?lang=fr",
        Some(json!({ "ids": [] })),
    )
    .await;
    assert_eq!(empty, StatusCode::UNPROCESSABLE_ENTITY);
}
