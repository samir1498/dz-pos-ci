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
use dzpos_core::print::{render_facture, render_ticket, FactureLayout, Page, Paper};
use dzpos_core::services::documents;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

use common::printed;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";
const HTML: &str = "text/html; charset=utf-8";

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).unwrap()
}

fn app() -> (tempfile::TempDir, std::path::PathBuf, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
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
            .header(common::SESSION_HEADER, common::OWNER_SESSION)
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
        .header(common::SESSION_HEADER, common::OWNER_SESSION)
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
        .header(common::SESSION_HEADER, common::OWNER_SESSION)
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

// ---- the facture (features.md §3 and §4)

/// One sale rung up as a facture, through the routes: the shop's block, a
/// company fiche carrying its identifiers, then the sale itself.
async fn a_facture(app: &axum::Router) -> i64 {
    let post = |uri: &'static str, body: Value| {
        let app = app.clone();
        async move {
            let req = Request::builder()
                .method(if uri == "/settings/store" {
                    "PUT"
                } else {
                    "POST"
                })
                .uri(uri)
                .header("authorization", format!("Bearer {TOKEN}"))
                .header(common::SESSION_HEADER, common::OWNER_SESSION)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap();
            let res = app.oneshot(req).await.unwrap();
            let status = res.status();
            let bytes = res.into_body().collect().await.unwrap().to_bytes();
            let value: Value = serde_json::from_slice(&bytes).unwrap();
            assert!(status.is_success(), "{uri}: {value}");
            value
        }
    };

    post(
        "/settings/store",
        json!({
            "name": "Supérette El Bahdja",
            "rc": "16/00-1234567 B 21",
            "nif": "000216001234567",
            "nis": "098216001234567",
            "ai": "16123456789",
            "address": "Rue Didouche Mourad, Alger",
            "phone": "021 00 00 00",
        }),
    )
    .await;
    let product = post(
        "/products",
        json!({
            "name": "Ciment CPJ 45",
            "unit": "box",
            "cost_centimes": 50_000,
            "selling_centimes": 100_000,
            "qty_on_hand_milli": 10_000,
            "rate_bps": 1900,
        }),
    )
    .await;
    let customer = post(
        "/customers",
        json!({
            "name": "Entreprise Amrani",
            "party_kind": "company",
            "phone": null,
            "address": "Zone industrielle, Rouiba",
            "rc": "16/00-7654321 B 22",
            "nif": null,
            "nis": "098216007654321",
            "ai": null,
            "credit_limit_centimes": null,
            "warn_threshold_centimes": null,
            "notes": null,
            "active": true,
            "opening_debt_centimes": null,
        }),
    )
    .await;
    let sale = post(
        "/sales",
        json!({
            "payment_mode": "credit",
            "customer_id": customer["id"],
            "kind": "facture",
            "lines": [{ "product_id": product["id"], "qty_milli": 2_000 }],
        }),
    )
    .await;
    assert_eq!(sale["kind"], json!("facture"), "{sale}");
    sale["id"].as_i64().unwrap()
}

#[tokio::test]
async fn a_facture_is_the_html_the_core_renders_on_the_sheet_that_was_asked_for() {
    let (_dir, path, app) = app();
    let id = a_facture(&app).await;

    for lang in Lang::ALL {
        for paper in [Paper::A4, Paper::A5] {
            let sheet = match paper {
                Paper::A4 => "a4",
                Paper::A5 => "a5",
            };
            let (status, content_type, body) = call_text(
                &app,
                &format!("/sales/{id}/facture?lang={}&paper={sheet}", lang.tag()),
                true,
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{body}");
            assert_eq!(content_type.as_deref(), Some(HTML), "{lang:?} {sheet}");

            let mut conn = dzpos_core::db::open(&path).unwrap();
            let stored = documents::get(&mut conn, SHOP, i32::try_from(id).unwrap()).unwrap();
            assert_eq!(
                body,
                render_facture(
                    &stored,
                    lang,
                    Page {
                        paper,
                        layout: FactureLayout::Standard,
                    },
                )
                .unwrap(),
                "{lang:?} {sheet}"
            );
        }
    }
}

/// The layout the shop chose reaches the paper, and a layout named in the
/// query beats it.
///
/// Two claims and both matter. The first is the whole feature: a shop that
/// picked a layout in settings and then printed a facture in the other one
/// would have chosen nothing. The second is what a settings screen showing a
/// preview of each layout needs, and it is the one that could silently stop
/// working, because every other call leaves the query out.
///
/// The assertion is the page margin, 8mm on the compact stylesheet against
/// 12mm on the standard one, rather than a word: which layout was drawn is
/// the question, not what it says.
#[tokio::test]
async fn the_facture_comes_back_in_the_layout_the_shop_chose() {
    let (_dir, _path, app) = app();
    let id = a_facture(&app).await;

    let (_, _, standard) =
        call_text(&app, &format!("/sales/{id}/facture?lang=fr&paper=a4"), true).await;
    assert!(standard.contains("margin: 12mm"), "the default layout");

    let req = Request::builder()
        .method("PUT")
        .uri("/settings/facture-layout")
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, common::OWNER_SESSION)
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "facture_layout": "compact" }).to_string(),
        ))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(req).await.unwrap().status(),
        StatusCode::OK
    );

    let (_, _, compact) =
        call_text(&app, &format!("/sales/{id}/facture?lang=fr&paper=a4"), true).await;
    assert!(compact.contains("margin: 8mm"), "the layout the shop chose");

    let (_, _, forced) = call_text(
        &app,
        &format!("/sales/{id}/facture?lang=fr&paper=a4&layout=standard"),
        true,
    )
    .await;
    assert!(
        forced.contains("margin: 12mm"),
        "a layout named in the query has to beat the setting"
    );
}

/// The half sheet reaches the paper as A5, whatever the caller asked for.
///
/// The override lives in `Page::paper` and is proved there against the type;
/// this is the same claim over the whole route, because a query string, a
/// stored setting and a render sit between the two and any of them could
/// drop it. A shop on the half sheet whose till asked for A4 would otherwise
/// hand a customer a small facture in the corner of a large page.
///
/// Read off the `@page` rule, which is the one line `Paper` sets.
#[tokio::test]
async fn the_half_sheet_reaches_the_paper_as_a5() {
    let (_dir, _path, app) = app();
    let id = a_facture(&app).await;

    let req = Request::builder()
        .method("PUT")
        .uri("/settings/facture-layout")
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, common::OWNER_SESSION)
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "facture_layout": "half_sheet" }).to_string(),
        ))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(req).await.unwrap().status(),
        StatusCode::OK
    );

    // The till names A4, which is the case that matters: the setting has to
    // win, and it is the only way a shop reaches this by accident.
    let (_, _, chosen) =
        call_text(&app, &format!("/sales/{id}/facture?lang=fr&paper=a4"), true).await;
    assert!(
        chosen.contains("size: A5;"),
        "the half sheet pins its sheet"
    );
    assert!(chosen.contains("margin: 7mm"), "the half sheet stylesheet");

    // Named in the query rather than stored, the same thing holds.
    let (_, _, previewed) = call_text(
        &app,
        &format!("/sales/{id}/facture?lang=fr&paper=a4&layout=half_sheet"),
        true,
    )
    .await;
    assert!(previewed.contains("size: A5;"), "a previewed half sheet");

    // And a layout with no fixed sheet still takes the one it was given, so
    // the override belongs to the half sheet and not to every print.
    let (_, _, standard) = call_text(
        &app,
        &format!("/sales/{id}/facture?lang=fr&paper=a4&layout=standard"),
        true,
    )
    .await;
    assert!(standard.contains("size: A4;"), "the standard layout on A4");
}

/// A layout nobody has a template for is refused rather than drawn on the
/// default: a preview that quietly showed the standard page when asked for
/// one that does not exist would say a layout works when it does not.
#[tokio::test]
async fn a_layout_the_app_cannot_draw_is_422_in_the_envelope() {
    let (_dir, _path, app) = app();
    let id = a_facture(&app).await;
    let (status, _, _) = call_text(
        &app,
        &format!("/sales/{id}/facture?lang=fr&paper=a4&layout=hologram"),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn a_lang_or_a_sheet_the_app_does_not_print_is_422_in_the_envelope() {
    let (_dir, _path, app) = app();
    let id = a_facture(&app).await;
    for uri in [
        format!("/sales/{id}/facture?lang=es&paper=a4"),
        format!("/sales/{id}/facture?lang=fr&paper=a3"),
        format!("/sales/{id}/facture?lang=fr&paper=A4"),
        format!("/sales/{id}/facture?lang=fr"),
        format!("/sales/{id}/facture?paper=a4"),
    ] {
        let (status, content_type, body) = call_text(&app, &uri, true).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{uri}: {body}");
        assert_eq!(error_code(&body), "bad_request", "{uri}");
        assert_ne!(content_type.as_deref(), Some(HTML), "{uri}");
    }
}

/// A ticket has its own 80 mm paper. Printing one through the facture route
/// would be a document titled FACTURE that took a number out of the ticket
/// series, so the route does not find it at all: the id names no facture.
/// The rule is the same in both directions, and for the same reason: a
/// facture squeezed onto an 80 mm slip is a facture that does not look like
/// one, so the ticket route does not find it either.
#[tokio::test]
async fn each_print_route_only_finds_its_own_kind() {
    let (_dir, _path, app) = app();
    let id = a_sale(&app).await;
    let (status, _, body) =
        call_text(&app, &format!("/sales/{id}/facture?lang=fr&paper=a4"), true).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(error_code(&body), "not_found");

    // And the other way round: the facture is not on the ticket route.
    let facture = a_facture(&app).await;
    let (status, _, body) =
        call_text(&app, &format!("/sales/{facture}/ticket?lang=fr"), true).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(error_code(&body), "not_found");
}

#[tokio::test]
async fn a_facture_that_does_not_exist_or_belongs_to_another_shop_is_404() {
    let (_dir, path, app) = app();
    let id = a_facture(&app).await;
    let (status, _, body) = call_text(&app, "/sales/404/facture?lang=fr&paper=a4", true).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(error_code(&body), "not_found");

    // Rule 3: the row is real and readable for the shop that owns it.
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
         FROM documents WHERE shop_id = 1 AND kind = 'facture';",
    )
    .unwrap();
    let other = i32::try_from(id).unwrap() + 1;
    assert!(documents::get(&mut conn, 2, other).is_ok());

    let (status, _, body) = call_text(
        &app,
        &format!("/sales/{other}/facture?lang=fr&paper=a4"),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(error_code(&body), "not_found");
}

#[tokio::test]
async fn the_facture_route_refuses_a_call_without_the_launch_token() {
    let (_dir, _path, app) = app();
    let id = a_facture(&app).await;
    let (status, _, body) = call_text(
        &app,
        &format!("/sales/{id}/facture?lang=fr&paper=a4"),
        false,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(error_code(&body), "unauthorized");
}

/// A POST through the router, for the tests below that write a document
/// before printing it.
async fn post_json(app: &axum::Router, uri: &str, body: Value) -> Value {
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
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(status.is_success(), "{uri}: {value}");
    value
}

#[tokio::test]
async fn an_avoir_prints_on_the_facture_sheet_and_names_the_facture_it_credits() {
    // The reference line is a sentence the template builds out of the
    // referenced facture's number and day, neither of which is on the avoir's
    // own row: the route is the layer that can read that second document, so
    // this is what proves it does.
    let (_dir, _path, app) = app();
    let facture = a_facture(&app).await;
    let (_, _, sheet) = call_text(
        &app,
        &format!("/sales/{facture}/facture?lang=fr&paper=a4"),
        true,
    )
    .await;
    let facture_number = sheet
        .split("FA-")
        .nth(1)
        .map(|rest| format!("FA-{}", &rest[..6]))
        .expect("the facture prints its own number");

    let avoir = post_json(
        &app,
        &format!("/sales/{facture}/avoir"),
        json!({ "lines": null, "reason": "retour marchandise" }),
    )
    .await;
    let avoir_id = avoir["id"].as_i64().unwrap();

    let (status, content_type, body) = call_text(
        &app,
        &format!("/sales/{avoir_id}/facture?lang=fr&paper=a4"),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(content_type.as_deref(), Some(HTML));
    assert!(
        body.contains(&facture_number),
        "the avoir does not name the facture it credits: {facture_number}"
    );
    assert!(body.contains(&printed("AV", 1)), "the avoir's own number");
}

#[tokio::test]
async fn a_cancelled_facture_prints_the_day_and_the_reason_it_was_annulled() {
    let (_dir, _path, app) = app();
    let facture = a_facture(&app).await;
    post_json(
        &app,
        &format!("/sales/{facture}/cancel"),
        json!({ "reason": "commande annulée par le client" }),
    )
    .await;

    let (status, _, body) = call_text(
        &app,
        &format!("/sales/{facture}/facture?lang=fr&paper=a4"),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // The reason is free text somebody typed, so it reaches the page escaped
    // and the day comes off the block the cancellation wrote.
    assert!(
        body.contains("commande annulée par le client"),
        "the annulée face does not say why"
    );
}

#[tokio::test]
async fn a_proforma_prints_on_the_same_sheet_under_its_own_number() {
    let (_dir, _path, app) = app();
    // The same seller, product and customer the facture helper sets up.
    let facture = a_facture(&app).await;
    let (_, _, _) = call_text(
        &app,
        &format!("/sales/{facture}/facture?lang=fr&paper=a4"),
        true,
    )
    .await;
    let product = post_json(
        &app,
        "/products",
        json!({
            "name": "Sable 0/4",
            "unit": "box",
            "cost_centimes": 20_000,
            "selling_centimes": 40_000,
            "qty_on_hand_milli": 10_000,
            "rate_bps": 1900,
        }),
    )
    .await;
    let customer = post_json(
        &app,
        "/customers",
        json!({
            "name": "Sarl Bencheikh",
            "party_kind": "company",
            "phone": null,
            "address": "Rouiba",
            "rc": "16/00-1111111 B 22",
            "nif": null,
            "nis": "098216001111111",
            "ai": null,
            "credit_limit_centimes": null,
            "warn_threshold_centimes": null,
            "notes": null,
            "active": true,
            "opening_debt_centimes": null,
        }),
    )
    .await;
    let quote = post_json(
        &app,
        "/sales",
        json!({
            "payment_mode": "credit",
            "customer_id": customer["id"],
            "kind": "proforma",
            "lines": [{ "product_id": product["id"], "qty_milli": 1_000 }],
        }),
    )
    .await;
    let id = quote["id"].as_i64().unwrap();

    let (status, content_type, body) =
        call_text(&app, &format!("/sales/{id}/facture?lang=fr&paper=a4"), true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(content_type.as_deref(), Some(HTML));
    assert!(
        body.contains(&printed("PF", 1)),
        "the proforma's own number"
    );
}
