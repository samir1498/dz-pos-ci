// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Which wire the shop's thermal head is sent down, over HTTP.
//!
//! `ThermalMode::for_lang` is unit-tested in the core and the preference is
//! unit-tested in `services::preferences`. What neither of them can say is
//! whether the three routes that answer ESC/POS bytes actually read the
//! stored row: a handler that resolved the mode from a constant would leave
//! every one of those unit tests green and still print a row of boxes on an
//! Arabic ticket. So this file changes the preference through the settings
//! route a shop would use, then asks for the bytes and looks at what kind
//! of bytes came back
//! (`context/plans/20260921-arabic-on-a-cheap-thermal-head.md`).
//!
//! Signed in as a manager rather than the owner every other print test
//! uses: the gate table gives `PUT /settings/thermal-mode`
//! `EditSettings`, which a manager holds, and a shop's printing panel is
//! opened by whoever is on the floor that evening.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use dzpos_core::lang::Lang;
use dzpos_core::print::{
    render_facture_escpos, render_facture_escpos_text, render_ticket_escpos_in, FactureInput,
    ThermalMode,
};
use dzpos_core::services::documents;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";

/// `GS v 0`, the raster bit-image command every band opens with. Its
/// presence in an answer is what "these are dots, not characters" means on
/// the wire (`crates/core/src/print/escpos.rs`).
const BANDS: [u8; 3] = [0x1D, 0x76, 0x30];

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).unwrap()
}

/// A router with a manager signed in beside the owner `sign_in` opens.
fn app() -> (tempfile::TempDir, std::path::PathBuf, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    common::sign_in_as(&path, SHOP, "manager", common::MANAGER_SESSION);
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
        .header(common::SESSION_HEADER, common::MANAGER_SESSION);
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
        .header(common::SESSION_HEADER, common::MANAGER_SESSION)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, bytes)
}

/// Stores the shop's choice through the route the printing panel calls.
async fn set_thermal_mode(app: &axum::Router, mode: ThermalMode) {
    let (status, body) = call(
        app,
        "PUT",
        "/settings/thermal-mode",
        Some(json!({ "thermal_mode": mode.as_str() })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["thermal_mode"], json!(mode.as_str()), "{body}");
}

/// A shop with identifiers, one product, one company customer, and both
/// papers issued: a credit facture and a cash ticket.
async fn a_shop_with_both_papers(app: &axum::Router) -> (i64, i64) {
    let post = |method: &'static str, uri: &'static str, body: Value| {
        let app = app.clone();
        async move {
            let (status, value) = call(&app, method, uri, Some(body)).await;
            assert!(status.is_success(), "{uri}: {value}");
            value
        }
    };
    post(
        "PUT",
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
        "POST",
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
        "POST",
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
    let facture = post(
        "POST",
        "/sales",
        json!({
            "payment_mode": "credit",
            "customer_id": customer["id"],
            "kind": "facture",
            "lines": [{ "product_id": product["id"], "qty_milli": 2_000 }],
        }),
    )
    .await;
    assert_eq!(facture["kind"], json!("facture"), "{facture}");
    let ticket = post(
        "POST",
        "/sales",
        json!({
            "payment_mode": "cash",
            "lines": [{ "product_id": product["id"], "qty_milli": 1_000 }],
            "tendered_centimes": 200_000,
        }),
    )
    .await;
    (
        facture["id"].as_i64().unwrap(),
        ticket["id"].as_i64().unwrap(),
    )
}

/// Two streams of ESC/POS, compared without printing either of them: a
/// drawn roll is about 86 KB and a failure report that dumps both twice is
/// a screen of numbers nobody reads.
fn same_bytes(got: &[u8], want: &[u8], why: &str) {
    assert!(
        got == want,
        "{why}: {} bytes answered against {} rendered",
        got.len(),
        want.len()
    );
}

/// Does `haystack` carry `needle` anywhere in it.
fn carries(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// The facture roll and the ticket both take the wire the shop stored, and
/// the bytes are the core's own render of that document in that mode — not
/// merely bytes of the right shape.
#[tokio::test]
async fn both_escpos_routes_answer_the_wire_the_shop_stored() {
    let (_dir, path, app) = app();
    let (facture_id, ticket_id) = a_shop_with_both_papers(&app).await;

    let mut conn = dzpos_core::db::open(&path).unwrap();
    let facture = documents::get(&mut conn, SHOP, i32::try_from(facture_id).unwrap()).unwrap();
    let ticket = documents::get(&mut conn, SHOP, i32::try_from(ticket_id).unwrap()).unwrap();
    let input = FactureInput {
        referenced: None,
        cancellation: None,
    };
    // A plain facture references nothing and is not cancelled, so the two
    // reads the handler makes beside the document are both empty here; the
    // referenced-document path is covered in `print_api.rs`.
    let facture_text = render_facture_escpos_text(&facture, &input, Lang::Fr).unwrap();
    let facture_bands =
        render_facture_escpos(&facture, &input, Lang::Fr, ThermalMode::Raster).unwrap();
    let ticket_text = render_ticket_escpos_in(&ticket, Lang::Fr, ThermalMode::Text).unwrap();
    let ticket_bands = render_ticket_escpos_in(&ticket, Lang::Fr, ThermalMode::Raster).unwrap();
    // The number is ASCII in either language and is on every facture
    // (décret 05-468 art. 3), so it is the one string that says "a head
    // reading this in text mode prints characters". Drawn, it is dots and
    // the bytes are gone.
    let number = dzpos_core::print::number(&facture).into_bytes();
    assert!(
        carries(&facture_text, &number),
        "the text roll lost its number"
    );

    set_thermal_mode(&app, ThermalMode::Text).await;
    let (status, bytes) =
        call_bytes(&app, &format!("/sales/{facture_id}/facture/escpos?lang=fr")).await;
    assert_eq!(status, StatusCode::OK);
    same_bytes(&bytes, &facture_text, "the stored text wire did not answer");
    assert!(carries(&bytes, &number), "a text roll prints its number");
    assert!(!carries(&bytes, &BANDS), "text mode sent raster bands");

    let (status, bytes) =
        call_bytes(&app, &format!("/sales/{ticket_id}/ticket/escpos?lang=fr")).await;
    assert_eq!(status, StatusCode::OK);
    same_bytes(
        &bytes,
        &ticket_text,
        "the ticket ignored the stored text wire",
    );
    assert!(!carries(&bytes, &BANDS), "text mode sent raster bands");

    set_thermal_mode(&app, ThermalMode::Raster).await;
    let (status, bytes) =
        call_bytes(&app, &format!("/sales/{facture_id}/facture/escpos?lang=fr")).await;
    assert_eq!(status, StatusCode::OK);
    same_bytes(
        &bytes,
        &facture_bands,
        "the stored raster wire did not answer",
    );
    assert!(carries(&bytes, &BANDS), "a drawn roll is sent as bands");
    assert!(
        !carries(&bytes, &number),
        "a drawn roll carries no readable text, so its number is dots"
    );

    let (status, bytes) =
        call_bytes(&app, &format!("/sales/{ticket_id}/ticket/escpos?lang=fr")).await;
    assert_eq!(status, StatusCode::OK);
    same_bytes(
        &bytes,
        &ticket_bands,
        "the ticket ignored the stored raster wire",
    );
    assert!(carries(&bytes, &BANDS), "a drawn ticket is sent as bands");
}

/// The one thing the preference does not reach. Arabic has no single-byte
/// table on a cheap head, so both routes draw it whatever the shop stored;
/// a shop on `text` that hands an Arabic facture to a customer should get
/// Arabic, not a column of boxes.
#[tokio::test]
async fn arabic_is_drawn_on_both_routes_under_either_stored_wire() {
    let (_dir, _path, app) = app();
    let (facture_id, ticket_id) = a_shop_with_both_papers(&app).await;

    for stored in ThermalMode::ALL {
        set_thermal_mode(&app, stored).await;
        for uri in [
            format!("/sales/{facture_id}/facture/escpos?lang=ar"),
            format!("/sales/{ticket_id}/ticket/escpos?lang=ar"),
        ] {
            let (status, bytes) = call_bytes(&app, &uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}");
            assert!(
                carries(&bytes, &BANDS),
                "{uri} under stored {}: Arabic was not drawn",
                stored.as_str()
            );
        }
    }
}

/// The preference outlives the request that set it, which is the difference
/// between a setting and a query string: the panel is opened once and every
/// paper after it takes the new wire.
#[tokio::test]
async fn the_stored_wire_is_read_again_on_the_next_paper() {
    let (_dir, _path, app) = app();
    let (facture_id, _ticket) = a_shop_with_both_papers(&app).await;
    set_thermal_mode(&app, ThermalMode::Raster).await;

    let (status, settings) = call(&app, "GET", "/settings", None).await;
    assert_eq!(status, StatusCode::OK, "{settings}");
    assert_eq!(settings["thermal_mode"], json!("raster"), "{settings}");

    for _ in 0..2 {
        let (status, bytes) =
            call_bytes(&app, &format!("/sales/{facture_id}/facture/escpos?lang=en")).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            carries(&bytes, &BANDS),
            "the stored wire was read once only"
        );
    }
}
