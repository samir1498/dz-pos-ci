// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The settings routes: the store block a ticket prints as the seller and
//! the dated régime fiscal. In-process router, real temp SQLite file.

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

struct Harness {
    _dir: tempfile::TempDir,
    app: axum::Router,
}

fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    Harness {
        _dir: dir,
        app: dzpos_api::router(state, &token()),
    }
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

fn full_store() -> Value {
    json!({
        "name": "Superette El Baraka",
        "rc": "16/00-1234567 B 20",
        "nif": "000016001234567",
        "nis": "000116001234567",
        "ai": "16012345678",
        "address": "12 rue Didouche Mourad, Alger",
        "phone": "0555 12 34 56"
    })
}

/// Today as the route sees it (UTC calendar day), so a date one day ahead
/// is planned and one day back is current whatever the clock says.
fn today() -> chrono::NaiveDate {
    chrono::Utc::now().date_naive()
}

fn day(d: chrono::NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

#[tokio::test]
async fn the_seeded_shop_reads_as_its_name_reel_and_nothing_planned() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/settings", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({
            "store": {
                "name": "Mon magasin",
                "rc": null, "nif": null, "nis": null, "ai": null,
                "address": null, "phone": null
            },
            "regime": { "regime": "reel", "valid_from": "2026-01-01" },
            "regime_planned": null
        })
    );
}

#[tokio::test]
async fn put_store_replaces_the_block_and_get_reads_it_back() {
    let h = harness();
    let (status, body) = call(&h.app, "PUT", "/settings/store", Some(full_store())).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, full_store());

    let (_, all) = call(&h.app, "GET", "/settings", None).await;
    assert_eq!(all["store"], full_store());

    // The block again with two identifiers gone: one null, one blank.
    let mut cleared = full_store();
    cleared["rc"] = Value::Null;
    cleared["nif"] = json!("  ");
    let (status, body) = call(&h.app, "PUT", "/settings/store", Some(cleared)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["rc"], Value::Null, "null clears");
    assert_eq!(body["nif"], Value::Null, "blank clears");
    assert_eq!(body["nis"], json!("000116001234567"));
}

#[tokio::test]
async fn a_missing_field_is_a_null_and_an_unknown_one_is_refused() {
    let h = harness();
    // The wire type defaults every identifier; only the name is required.
    let (status, body) = call(
        &h.app,
        "PUT",
        "/settings/store",
        Some(json!({ "name": "Chez Ali" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["phone"], Value::Null);

    let mut extra = full_store();
    extra["fax"] = json!("021 00 00 00");
    let (status, body) = call(&h.app, "PUT", "/settings/store", Some(extra)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "bad_request");
    let (_, all) = call(&h.app, "GET", "/settings", None).await;
    assert_eq!(
        all["store"]["name"], "Chez Ali",
        "the refused body changed nothing"
    );
}

#[tokio::test]
async fn a_blank_name_is_422_naming_the_field_and_changes_nothing() {
    let h = harness();
    let mut blank = full_store();
    blank["name"] = json!("   ");
    let (status, body) = call(&h.app, "PUT", "/settings/store", Some(blank)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "validation");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("name "),
        "{body}"
    );
    let (_, all) = call(&h.app, "GET", "/settings", None).await;
    assert_eq!(all["store"]["name"], "Mon magasin");
    assert_eq!(all["store"]["rc"], Value::Null);
}

#[tokio::test]
async fn a_regime_change_dated_today_or_earlier_is_current_at_once() {
    let h = harness();
    let yesterday = day(today().pred_opt().unwrap());
    let (status, body) = call(
        &h.app,
        "POST",
        "/settings/regime",
        Some(json!({ "regime": "ifu", "valid_from": yesterday })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["regime"],
        json!({ "regime": "ifu", "valid_from": yesterday })
    );
    assert_eq!(body["regime_planned"], Value::Null);
    assert_eq!(
        body["store"]["name"], "Mon magasin",
        "the whole page comes back"
    );

    let (_, all) = call(&h.app, "GET", "/settings", None).await;
    assert_eq!(all["regime"]["regime"], "ifu");
}

#[tokio::test]
async fn a_regime_change_dated_ahead_is_planned_and_reel_stays_current() {
    let h = harness();
    let next_year = day(today().succ_opt().unwrap());
    let (status, body) = call(
        &h.app,
        "POST",
        "/settings/regime",
        Some(json!({ "regime": "ifu", "valid_from": next_year })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["regime"],
        json!({ "regime": "reel", "valid_from": "2026-01-01" })
    );
    assert_eq!(
        body["regime_planned"],
        json!({ "regime": "ifu", "valid_from": next_year })
    );
}

#[tokio::test]
async fn a_regime_change_with_a_bad_day_or_regime_is_refused_and_leaves_no_row() {
    let h = harness();
    for (body, code) in [
        (
            json!({ "regime": "ifu", "valid_from": "2027-1-1" }),
            "validation",
        ),
        (
            json!({ "regime": "ifu", "valid_from": "2027-13-01" }),
            "validation",
        ),
        (
            json!({ "regime": "ifu", "valid_from": "2027-01-01T00:00:00" }),
            "validation",
        ),
        (json!({ "regime": "ifu", "valid_from": "" }), "validation"),
        (
            json!({ "regime": "forfait", "valid_from": "2027-01-01" }),
            "bad_request",
        ),
        (json!({ "regime": "ifu" }), "bad_request"),
        (
            json!({ "regime": "ifu", "valid_from": "2027-01-01", "note": "x" }),
            "bad_request",
        ),
    ] {
        let (status, answer) = call(&h.app, "POST", "/settings/regime", Some(body.clone())).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{body} -> {answer}"
        );
        assert_eq!(answer["error"]["code"], code, "{body} -> {answer}");
    }
    let (_, all) = call(&h.app, "GET", "/settings", None).await;
    assert_eq!(all["regime"]["regime"], "reel");
    assert_eq!(
        all["regime_planned"],
        Value::Null,
        "no refused body left a row"
    );
}

#[tokio::test]
async fn the_settings_routes_need_the_token_and_refuse_other_methods() {
    let h = harness();
    for (method, uri) in [
        ("GET", "/settings"),
        ("PUT", "/settings/store"),
        ("POST", "/settings/regime"),
    ] {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let res = h.app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{method} {uri}");
    }
    for (method, uri) in [
        ("DELETE", "/settings"),
        ("PUT", "/settings"),
        ("GET", "/settings/store"),
        ("POST", "/settings/store"),
        ("PUT", "/settings/regime"),
        ("GET", "/settings/regime"),
    ] {
        let (status, body) = call(&h.app, method, uri, Some(json!({}))).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} {uri}");
        assert_eq!(body["error"]["code"], "method_not_allowed");
    }
}
