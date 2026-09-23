// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The shelf label over HTTP. The core renders it, so these tests compare
//! the answer to `render_label` of the stored fiche rather than to a fixture
//! of their own: the goldens live in `dzpos-core`, and what is left here is
//! that the route reads the right product, in the right shop, and refuses
//! the same things the core refuses.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use dzpos_core::error::CoreError;
use dzpos_core::lang::Lang;
use dzpos_core::print::{render_label, render_label_sheet};
use dzpos_core::services::products;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

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
    common::sign_in(&path, SHOP);
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
    assert_eq!(value["error"]["field"], json!("barcode_digits"));

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
        .with_conn(|c| -> Result<_, CoreError> {
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

#[tokio::test]
async fn a_selection_past_the_cap_is_refused_before_a_single_row_is_read() {
    let h = harness();
    let id = create(&h.app, "Café moulu 250 g", Some("2000010000074")).await;

    // Two hundred is eleven A4 pages and already more than anybody stands
    // at a printer for. The refusal comes before the reads, so a body
    // naming fifty thousand ids is not fifty thousand queries first.
    let at_the_cap: Vec<i32> = std::iter::repeat_n(id, 200).collect();
    let (ok, _, body) = call(
        &h.app,
        "POST",
        "/labels/sheet?lang=fr",
        Some(json!({ "ids": at_the_cap })),
    )
    .await;
    assert_eq!(ok, StatusCode::OK, "{body}");

    let one_over: Vec<i32> = std::iter::repeat_n(id, 201).collect();
    let (refused, _, over) = call(
        &h.app,
        "POST",
        "/labels/sheet?lang=fr",
        Some(json!({ "ids": one_over })),
    )
    .await;
    assert_eq!(refused, StatusCode::UNPROCESSABLE_ENTITY);
    let value: Value = serde_json::from_str(&over).unwrap();
    assert_eq!(value["error"]["code"], json!("validation"));
    assert_eq!(value["error"]["field"], json!("ids"));
}

/// Neither the label nor the sheet is a fiscal paper, and Samir's ruling
/// keeps both on the caller's own `?lang=` regardless of what the shop has
/// stored (`context/plans/20260920-a-print-language-the-shop-keeps.md`). A
/// later change that quietly routed either one through the stored
/// preference should turn this red.
#[tokio::test]
async fn the_label_and_the_sheet_keep_following_the_callers_lang_with_arabic_stored() {
    let h = harness();
    let id = create(&h.app, "Café moulu 250 g", Some("2000010000074")).await;

    let (status, _, _) = call(
        &h.app,
        "PUT",
        "/settings/print-lang",
        Some(json!({ "print_lang": "ar" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _, body) = call(
        &h.app,
        "GET",
        &format!("/products/{id}/label?lang=fr"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let stored = h.state.with_conn(|c| products::get(c, SHOP, id)).unwrap();
    assert_eq!(
        body,
        render_label(&stored, Lang::Fr).unwrap(),
        "the label followed the stored Arabic instead of ?lang=fr"
    );

    let (status, _, body) = call(
        &h.app,
        "POST",
        "/labels/sheet?lang=fr",
        Some(json!({ "ids": [id] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        render_label_sheet(&[stored], Lang::Fr).unwrap(),
        "the sheet followed the stored Arabic instead of ?lang=fr"
    );
}
