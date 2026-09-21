// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The workbooks leaving and the filled file coming back.
//!
//! The core's own tests read every cell of every workbook (`export_service`,
//! `import_service`). What is left for this layer is what the core cannot
//! see: the media type, the filename header a browser saves under, and the
//! counts on the way back. That a file with a refusal in it writes nothing
//! is the core's rule and the core's test; what is checked here is that the
//! refusal crosses as the envelope every other one does.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";
const XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

/// The first four bytes of every .xlsx: a workbook is a zip, and a file that
/// does not start `PK\x03\x04` is not one whatever the header claims.
const ZIP: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).unwrap()
}

fn app() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    let router = dzpos_api::router(state, &token());
    (dir, router)
}

struct Answer {
    status: StatusCode,
    content_type: Option<String>,
    disposition: Option<String>,
    body: Vec<u8>,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }
}

async fn call(app: &axum::Router, method: &str, uri: &str, body: Option<Vec<u8>>) -> Answer {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, common::OWNER_SESSION);
    let req = match body {
        Some(bytes) => req.body(Body::from(bytes)).unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let header = |name: &str| {
        res.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    let content_type = header("content-type");
    let disposition = header("content-disposition");
    let body = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    Answer {
        status,
        content_type,
        disposition,
        body,
    }
}

async fn post_json(app: &axum::Router, uri: &str, body: Value) -> Answer {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, common::OWNER_SESSION)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let body = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    Answer {
        status,
        content_type: None,
        disposition: None,
        body,
    }
}

fn a_product(name: &str, barcode: &str) -> Value {
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

#[tokio::test]
async fn the_four_workbooks_leave_as_spreadsheets_with_a_filename_on_them() {
    let (_dir, app) = app();
    let made = post_json(
        &app,
        "/products",
        a_product("Café moulu 250 g", "2000010000074"),
    )
    .await;
    assert_eq!(made.status, StatusCode::CREATED);

    for (path, stem) in [
        ("/export/products?lang=fr", "produits"),
        ("/export/sales?lang=fr", "ventes"),
        ("/export/customers?lang=fr", "clients"),
        ("/export/suppliers?lang=fr", "fournisseurs"),
    ] {
        let answer = call(&app, "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{path}");
        assert_eq!(answer.content_type.as_deref(), Some(XLSX), "{path}");
        let disposition = answer.disposition.unwrap_or_default();
        assert!(
            disposition.starts_with(&format!("attachment; filename=\"{stem}-")),
            "{path}: {disposition}"
        );
        assert!(disposition.ends_with(".xlsx\""), "{path}: {disposition}");
        assert_eq!(&answer.body[..4], &ZIP, "{path} is not a workbook");
    }
}

#[tokio::test]
async fn a_sales_export_takes_a_range_and_a_lang_it_does_not_know_is_the_callers_mistake() {
    let (_dir, app) = app();
    let ranged = call(
        &app,
        "GET",
        "/export/sales?lang=ar&from=2026-01-01&to=2026-12-31",
        None,
    )
    .await;
    assert_eq!(ranged.status, StatusCode::OK);
    assert_eq!(&ranged.body[..4], &ZIP);

    // The envelope every other refusal leaves in, never axum's own text.
    let bad_lang = call(&app, "GET", "/export/products?lang=de", None).await;
    assert_eq!(bad_lang.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(bad_lang.json()["error"]["code"], json!("bad_request"));

    let bad_day = call(&app, "GET", "/export/sales?lang=fr&from=le-1er", None).await;
    assert_eq!(bad_day.status, StatusCode::UNPROCESSABLE_ENTITY);

    // And an empty `from=` is a day too, not an absent one: serde reads the
    // key as a date it cannot parse. The client leaves the key out rather
    // than sending it blank (`packages/shared/src/client/export.ts`), so a range
    // nobody filled in is no range at all instead of a 422 on a screen that
    // asked for everything.
    let blank = call(&app, "GET", "/export/sales?lang=fr&from=&to=", None).await;
    assert_eq!(blank.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn the_token_is_what_lets_a_workbook_out() {
    let (_dir, app) = app();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/export/products?lang=fr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn the_template_downloads_and_comes_back_through_the_dry_run_clean() {
    let (_dir, app) = app();
    let template = call(&app, "GET", "/import/products/template?lang=fr", None).await;
    assert_eq!(template.status, StatusCode::OK);
    assert_eq!(template.content_type.as_deref(), Some(XLSX));
    assert!(template
        .disposition
        .unwrap_or_default()
        .contains("modele-produits-"));
    assert_eq!(&template.body[..4], &ZIP);

    // The example row the template ships with is a row the import accepts:
    // a shop that downloads the file, adds its rows under the example and
    // sends it back must not be refused by the line it was shown.
    let dry = call(
        &app,
        "POST",
        "/import/products/dry-run",
        Some(template.body.clone()),
    )
    .await;
    assert_eq!(dry.status, StatusCode::OK, "{:?}", dry.json());
    let report = dry.json();
    assert_eq!(report["refused"], json!(0), "{report}");
    assert_eq!(report["accepted"], json!(1), "{report}");
    assert_eq!(report["rows"][0]["row"], json!(2), "{report}");
    assert_eq!(report["rows"][0]["outcome"], json!("created"), "{report}");
    assert_eq!(report["rows"][0]["field"], Value::Null, "{report}");

    // Nothing was written by the dry run.
    let before = call(&app, "GET", "/products", None).await;
    assert_eq!(before.json().as_array().map(Vec::len), Some(0));

    let applied = call(&app, "POST", "/import/products", Some(template.body)).await;
    assert_eq!(applied.status, StatusCode::OK, "{:?}", applied.json());
    assert_eq!(applied.json()["created"], json!(1));
    assert_eq!(applied.json()["updated"], json!(0));
    // The template names a category the shop did not have.
    assert_eq!(applied.json()["categories_created"], json!(1));

    let after = call(&app, "GET", "/products", None).await;
    assert_eq!(after.json().as_array().map(Vec::len), Some(1));
    assert_eq!(after.json()[0]["name"], json!("Café moulu 250 g"));
}

/// The import template is not a fiscal paper, and Samir's ruling keeps it on
/// the caller's own `?lang=` regardless of what the shop has stored
/// (`context/plans/20260920-a-print-language-the-shop-keeps.md`). A later
/// change that quietly routed it through the stored preference should turn
/// this red.
#[tokio::test]
async fn the_template_keeps_following_the_callers_lang_with_arabic_stored() {
    let (_dir, app) = app();

    let put = Request::builder()
        .method("PUT")
        .uri("/settings/print-lang")
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, common::OWNER_SESSION)
        .header("content-type", "application/json")
        .body(Body::from(json!({ "print_lang": "ar" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(put).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let template = call(&app, "GET", "/import/products/template?lang=fr", None).await;
    assert_eq!(template.status, StatusCode::OK);
    assert_eq!(
        template.body,
        dzpos_core::services::import::template(dzpos_core::lang::Lang::Fr).unwrap(),
        "the template followed the stored Arabic instead of ?lang=fr"
    );
}

#[tokio::test]
async fn a_body_past_the_import_cap_is_refused_before_the_parser_sees_it() {
    let (_dir, app) = app();
    // Eight megabytes is the cap the two import routes carry, above axum's
    // 2 MB default (`routes::import::IMPORT_BODY_LIMIT`). A catalogue of
    // twenty thousand products is around one megabyte, so nothing a shop
    // sends comes near it; what this refuses is a file that would be read
    // whole into memory before anybody could say it was not a workbook.
    let over = vec![0_u8; dzpos_api::routes::import::IMPORT_BODY_LIMIT + 1];
    let refused = call(&app, "POST", "/import/products/dry-run", Some(over)).await;
    assert_eq!(refused.status, StatusCode::PAYLOAD_TOO_LARGE);

    // And a body inside the cap reaches the parser, which is what says the
    // cap is the thing that refused the one above and not the content.
    let inside = vec![0_u8; 64];
    let read = call(&app, "POST", "/import/products/dry-run", Some(inside)).await;
    assert_eq!(read.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(read.json()["error"]["field"], json!("file"));
}

#[tokio::test]
async fn a_file_that_is_not_a_workbook_is_refused_in_the_envelope() {
    let (_dir, app) = app();
    let refused = call(
        &app,
        "POST",
        "/import/products/dry-run",
        Some(b"name,price\nCafe,120".to_vec()),
    )
    .await;
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(refused.json()["error"]["code"], json!("validation"));
    assert_eq!(refused.json()["error"]["field"], json!("file"));
}
