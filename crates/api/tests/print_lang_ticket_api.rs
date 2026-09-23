// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// S7 of `a-kernel-crate-and-retail-as-the-first-module`: both ticket routes
// this file walks are sales routes (`routes::mod.rs`), so this whole file
// has nothing to compile with the feature off.
#![cfg(feature = "retail")]

//! The two ticket routes that answer bytes rather than an HTML string, both
//! resolving the shop's print language the same way the HTML ticket does
//! (`preferences::print_lang_for`, proved on the HTML route in
//! `print_api.rs`): the ESC/POS bytes a thermal printer gets, and the
//! print-through-desktop route's JSON answer and the spool file beside it
//! (`context/plans/20260920-a-print-language-the-shop-keeps.md`).
//!
//! A file of its own rather than a corner of `sales_api.rs`: that file is
//! already at the size ratchet's pinned length
//! (`scripts/file-sizes.json`), which may not grow.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use dzpos_core::lang::Lang;
use dzpos_core::print::ThermalMode;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";

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

async fn call_bytes(app: &axum::Router, uri: &str) -> (StatusCode, Vec<u8>) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, common::OWNER_SESSION)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, bytes)
}

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

/// One cash ticket, through the routes.
async fn a_cash_ticket(app: &axum::Router, product_id: i64) -> i64 {
    let (status, ticket) = call(
        app,
        "POST",
        "/sales",
        Some(json!({
            "lines": [{ "product_id": product_id, "qty_milli": 1_000 }],
            "payment_mode": "cash",
            "tendered_centimes": 20_000,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{ticket}");
    ticket["id"].as_i64().unwrap()
}

/// Sets the shop's stored print language through the same route the
/// settings panel calls.
async fn set_print_lang(app: &axum::Router, lang: Lang) {
    let (status, body) = call(
        app,
        "PUT",
        "/settings/print-lang",
        Some(json!({ "print_lang": lang.tag() })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// The bytes a thermal printer gets follow the same precedence the HTML
/// ticket does: the shop's stored print language beats `?lang=`, and
/// `?print_lang=` on the one call beats the stored setting in turn.
#[tokio::test]
async fn the_ticket_escpos_follows_the_stored_print_language_over_the_callers_own() {
    let (_dir, path, app) = app();
    let p = product(&app, "Sucre", 11_000, 1900).await;
    let ticket_id = a_cash_ticket(&app, p).await;
    set_print_lang(&app, Lang::Ar).await;

    let mut conn = dzpos_core::db::open(&path).unwrap();
    let stored = dzpos_core::services::documents::get_of_kind(
        &mut conn,
        SHOP,
        i32::try_from(ticket_id).unwrap(),
        dzpos_core::services::documents::DocumentKind::Ticket,
    )
    .unwrap();
    let ar_bytes =
        dzpos_core::print::render_ticket_escpos_in(&stored, Lang::Ar, ThermalMode::Text).unwrap();
    let fr_bytes =
        dzpos_core::print::render_ticket_escpos_in(&stored, Lang::Fr, ThermalMode::Text).unwrap();
    let en_bytes =
        dzpos_core::print::render_ticket_escpos_in(&stored, Lang::En, ThermalMode::Text).unwrap();

    // The stored Arabic wins over the caller's own French.
    let (status, bytes) =
        call_bytes(&app, &format!("/sales/{ticket_id}/ticket/escpos?lang=fr")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        bytes, ar_bytes,
        "the stored Arabic did not win over ?lang=fr"
    );
    assert_ne!(
        bytes, fr_bytes,
        "the caller's own lang printed instead of the stored setting"
    );

    // A named override wins over the stored setting in turn.
    let (status, bytes) = call_bytes(
        &app,
        &format!("/sales/{ticket_id}/ticket/escpos?lang=fr&print_lang=en"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        bytes, en_bytes,
        "?print_lang= did not win over the stored Arabic"
    );
}

/// The spool file and the `"lang"` this answers with are both the resolved
/// language, not the caller's own `lang`: a shop that stores Arabic prints
/// Arabic through the phone even when it names `?lang=fr`, and a named
/// override still wins over the stored setting for one print.
#[tokio::test]
async fn the_ticket_print_spools_under_the_stored_print_language_over_the_callers_own() {
    let (_dir, path, app) = app();
    let p = product(&app, "Sucre", 11_000, 1900).await;
    let ticket_id = a_cash_ticket(&app, p).await;
    set_print_lang(&app, Lang::Ar).await;

    let mut conn = dzpos_core::db::open(&path).unwrap();
    let stored = dzpos_core::services::documents::get_of_kind(
        &mut conn,
        SHOP,
        i32::try_from(ticket_id).unwrap(),
        dzpos_core::services::documents::DocumentKind::Ticket,
    )
    .unwrap();

    let (status, body) = call(
        &app,
        "POST",
        &format!("/sales/{ticket_id}/print?lang=fr"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["lang"], json!("ar"), "{body}");
    let spooled = body["spooled"].as_str().unwrap();
    assert!(
        spooled.contains(&format!("ticket-{ticket_id}-ar-raster.bin")),
        "{body}"
    );
    // The name carries the wire as well as the language, and the answer
    // says the same thing, so a phone inheriting the desktop's spool knows
    // which of the two kinds of bytes it is holding. Arabic is `raster`
    // here under a shop that has never opened the printing panel: the
    // language decides it, not the preference.
    assert_eq!(body["thermal_mode"], json!("raster"), "{body}");
    // The name is not the claim. What a thermal printer is handed is the
    // bytes in that file, so they are read back and matched against the
    // Arabic render rather than trusted because the filename says `ar`.
    assert_eq!(
        std::fs::read(spooled).unwrap(),
        dzpos_core::print::render_ticket_escpos_in(&stored, Lang::Ar, ThermalMode::Text).unwrap(),
        "the spool file under an Arabic name does not hold the Arabic ticket"
    );

    let (status, body) = call(
        &app,
        "POST",
        &format!("/sales/{ticket_id}/print?lang=fr&print_lang=en"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["lang"], json!("en"), "{body}");
    let spooled = body["spooled"].as_str().unwrap();
    assert!(
        spooled.contains(&format!("ticket-{ticket_id}-en-text.bin")),
        "{body}"
    );
    assert_eq!(body["thermal_mode"], json!("text"), "{body}");
    assert_eq!(
        std::fs::read(spooled).unwrap(),
        dzpos_core::print::render_ticket_escpos_in(&stored, Lang::En, ThermalMode::Text).unwrap(),
        "the spool file under an English name does not hold the English ticket"
    );
}

/// The third step of the precedence on the two byte routes, which the two
/// tests above cannot reach: with nothing stored and nothing named, both
/// follow the language the caller asked in. Every assertion above calls
/// with `?lang=fr`, so a handler that stopped forwarding the caller's
/// language and fell back to French would leave them all green. This one
/// asks in Arabic and English, neither of which is that fallback.
#[tokio::test]
async fn with_nothing_stored_the_two_byte_routes_follow_the_callers_own_language() {
    let (_dir, path, app) = app();
    let p = product(&app, "Sucre", 11_000, 1900).await;
    let ticket_id = a_cash_ticket(&app, p).await;

    let mut conn = dzpos_core::db::open(&path).unwrap();
    let stored = dzpos_core::services::documents::get_of_kind(
        &mut conn,
        SHOP,
        i32::try_from(ticket_id).unwrap(),
        dzpos_core::services::documents::DocumentKind::Ticket,
    )
    .unwrap();
    let ar_bytes =
        dzpos_core::print::render_ticket_escpos_in(&stored, Lang::Ar, ThermalMode::Text).unwrap();

    let (status, bytes) =
        call_bytes(&app, &format!("/sales/{ticket_id}/ticket/escpos?lang=ar")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        bytes, ar_bytes,
        "the escpos ticket did not follow ?lang=ar with nothing stored"
    );

    let (status, body) = call(
        &app,
        "POST",
        &format!("/sales/{ticket_id}/print?lang=en"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["lang"],
        json!("en"),
        "the print route did not follow ?lang=en with nothing stored: {body}"
    );
}
