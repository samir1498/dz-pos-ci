// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The supplier fiche and its ledger over HTTP. The API is the product's only
//! entry point (architecture.md, consequence of rule 2), so what these assert
//! is the contract the desktop reads: the list's order and its search, the
//! balance on the fiche, the whole-row update, the close with its reason, and
//! the payment that settles what the shop owes.

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
    path: std::path::PathBuf,
    app: axum::Router,
}

fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    Harness {
        _dir: dir,
        path,
        app: dzpos_api::router(state, &token()),
    }
}

/// The same shop file, answered for as shop 2: the state carries the shop, so
/// this is the whole of what another shop can reach.
fn other_shop(h: &Harness) -> axum::Router {
    dzpos_api::router(dzpos_api::AppState::open(&h.path, 2).unwrap(), &token())
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

/// A fiche as the screen posts it.
fn draft(name: &str) -> Value {
    json!({
        "name": name,
        "phone": "0770 11 22 33",
        "address": null,
        "rc": null,
        "nif": null,
        "nis": null,
        "ai": null,
        "notes": null,
        "active": true,
        "opening_debt_centimes": null
    })
}

/// The same fiche as the update sends it: no opening debt, and the reason a
/// close over an open account needs.
fn whole(name: &str, active: bool, close_reason: Option<&str>) -> Value {
    json!({
        "name": name,
        "phone": "0770 11 22 33",
        "address": null,
        "rc": null,
        "nif": null,
        "nis": null,
        "ai": null,
        "notes": null,
        "active": active,
        "close_reason": close_reason
    })
}

async fn a_supplier(app: &axum::Router, name: &str, opening: Option<i64>) -> i64 {
    let mut body = draft(name);
    if let Some(amount) = opening {
        body["opening_debt_centimes"] = json!(amount);
    }
    let (status, made) = call(app, "POST", "/suppliers", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    made["id"].as_i64().unwrap()
}

#[tokio::test]
async fn a_fiche_is_created_with_its_opening_debt_and_read_back_with_the_balance() {
    let h = harness();
    let id = a_supplier(&h.app, "Sarl Amrani", Some(250_000)).await;
    let (status, one) = call(&h.app, "GET", &format!("/suppliers/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(one["name"], "Sarl Amrani");
    assert_eq!(one["balance_centimes"], 250_000);
    assert_eq!(one["active"], true);

    let (status, ledger) = call(&h.app, "GET", &format!("/suppliers/{id}/ledger"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ledger["balance_centimes"], 250_000);
    assert_eq!(ledger["entries"][0]["kind"], "opening");
    assert_eq!(ledger["entries"][0]["balance_after_centimes"], 250_000);
    assert_eq!(ledger["entries"][0]["purchase_id"], Value::Null);
    assert_eq!(ledger["entries"][0]["allocations"], json!([]));
}

#[tokio::test]
async fn a_second_fiche_under_one_name_is_a_422_naming_the_name() {
    let h = harness();
    a_supplier(&h.app, "Sarl Amrani", None).await;
    let (status, body) = call(&h.app, "POST", "/suppliers", Some(draft("Sarl Amrani"))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");
    assert_eq!(body["error"]["field"], "name");
}

#[tokio::test]
async fn the_list_carries_each_balance_and_the_search_narrows_it() {
    let h = harness();
    a_supplier(&h.app, "Zoubir", None).await;
    a_supplier(&h.app, "Bensalem", Some(100_000)).await;

    let (status, all) = call(&h.app, "GET", "/suppliers", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(all.as_array().unwrap().len(), 2);
    assert_eq!(all[0]["name"], "Bensalem");
    assert_eq!(all[0]["balance_centimes"], 100_000);
    assert_eq!(all[1]["balance_centimes"], 0);

    let (_, found) = call(&h.app, "GET", "/suppliers?q=zoub", None).await;
    assert_eq!(found.as_array().unwrap().len(), 1);
    assert_eq!(found[0]["name"], "Zoubir");
    // An emptied box reads the whole list rather than nothing.
    let (_, whole_list) = call(&h.app, "GET", "/suppliers?q=", None).await;
    assert_eq!(whole_list.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn the_update_sends_the_whole_fiche_and_a_null_clears_a_column() {
    let h = harness();
    let id = a_supplier(&h.app, "Sarl Amrani", None).await;
    let mut body = whole("Sarl Amrani et fils", true, None);
    body["phone"] = Value::Null;
    let (status, after) = call(&h.app, "PUT", &format!("/suppliers/{id}"), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after["name"], "Sarl Amrani et fils");
    assert_eq!(after["phone"], Value::Null);
}

#[tokio::test]
async fn a_field_the_fiche_does_not_have_is_refused_rather_than_ignored() {
    let h = harness();
    let mut body = draft("Sarl Amrani");
    body["credit_limit_centimes"] = json!(100);
    let (status, refused) = call(&h.app, "POST", "/suppliers", Some(body)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["code"], "bad_request");
}

#[tokio::test]
async fn closing_a_fiche_that_still_owes_asks_for_a_reason() {
    let h = harness();
    let id = a_supplier(&h.app, "Sarl Amrani", Some(150_000)).await;

    let (status, refused) = call(
        &h.app,
        "POST",
        &format!("/suppliers/{id}/close"),
        Some(json!({ "reason": null })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["field"], "reason");

    let (status, closed) = call(
        &h.app,
        "POST",
        &format!("/suppliers/{id}/close"),
        Some(json!({ "reason": "le fournisseur a fermé" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{closed}");
    assert_eq!(closed["active"], false);
    // The balance is untouched by the close: the shop still owes what it owed.
    assert_eq!(closed["balance_centimes"], 150_000);

    // And it is closed once: asking again says so on the field.
    let (status, again) = call(
        &h.app,
        "POST",
        &format!("/suppliers/{id}/close"),
        Some(json!({ "reason": "encore" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{again}");
    assert_eq!(again["error"]["field"], "active");
}

#[tokio::test]
async fn a_payment_settles_the_balance_and_answers_the_whole_ledger() {
    let h = harness();
    let id = a_supplier(&h.app, "Sarl Amrani", Some(150_000)).await;
    let (status, ledger) = call(
        &h.app,
        "POST",
        &format!("/suppliers/{id}/payments"),
        Some(json!({ "amount_centimes": 50_000, "payment_mode": "cash", "note": "acompte" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{ledger}");
    assert_eq!(ledger["balance_centimes"], 100_000);
    assert_eq!(ledger["entries"][0]["kind"], "payment");
    assert_eq!(ledger["entries"][0]["credit_centimes"], 50_000);
    assert_eq!(ledger["entries"][0]["payment_mode"], "cash");
    assert_eq!(ledger["entries"][0]["note"], "acompte");
    assert_eq!(ledger["entries"][0]["balance_after_centimes"], 100_000);
    // An opening balance carries no order, so the money settled no paper.
    assert_eq!(ledger["entries"][0]["allocations"], json!([]));

    // The fiche and the ledger answer one figure.
    let (_, fiche) = call(&h.app, "GET", &format!("/suppliers/{id}"), None).await;
    assert_eq!(fiche["balance_centimes"], 100_000);
}

#[tokio::test]
async fn a_payment_above_what_is_owed_says_what_is_owed() {
    let h = harness();
    let id = a_supplier(&h.app, "Sarl Amrani", Some(150_000)).await;
    let (status, refused) = call(
        &h.app,
        "POST",
        &format!("/suppliers/{id}/payments"),
        Some(json!({ "amount_centimes": 150_001, "payment_mode": "cash" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["code"], "validation");
    assert_eq!(refused["error"]["field"], "amount_centimes");
    assert_eq!(refused["error"]["outstanding_centimes"], 150_000);
    // The payload gains no field: `party_side` is a facture's business.
    assert_eq!(refused["error"]["party_side"], Value::Null);
}

#[tokio::test]
async fn a_correction_writes_a_movement_and_answers_the_whole_ledger() {
    let h = harness();
    let id = a_supplier(&h.app, "Sarl Amrani", Some(150_000)).await;
    let (status, ledger) = call(
        &h.app,
        "POST",
        &format!("/suppliers/{id}/adjustments"),
        Some(json!({ "amount_centimes": -50_000, "note": "erreur de saisie" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{ledger}");
    assert_eq!(ledger["balance_centimes"], 100_000);
    assert_eq!(ledger["entries"][0]["kind"], "adjustment");
    assert_eq!(ledger["entries"][0]["credit_centimes"], 50_000);
    assert_eq!(ledger["entries"][0]["note"], "erreur de saisie");
    assert_eq!(ledger["entries"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn the_statement_covers_the_range_asked_for_and_both_ends_of_it() {
    let h = harness();
    let id = a_supplier(&h.app, "Sarl Amrani", Some(150_000)).await;
    let (status, today) = call(&h.app, "GET", "/clock", None).await;
    assert_eq!(status, StatusCode::OK);
    let day = today["today"].as_str().unwrap();

    let (status, statement) = call(
        &h.app,
        "GET",
        &format!("/suppliers/{id}/statement?from={day}&to={day}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{statement}");
    assert_eq!(statement["supplier_id"], id);
    assert_eq!(statement["from"], day);
    assert_eq!(statement["opening_centimes"], 0);
    assert_eq!(statement["closing_centimes"], 150_000);
    assert_eq!(statement["entries"].as_array().unwrap().len(), 1);

    // A day the API cannot read is the caller's mistake, named.
    let (status, refused) = call(
        &h.app,
        "GET",
        &format!("/suppliers/{id}/statement?from=2026-1-5&to={day}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["field"], "from");
}

#[tokio::test]
async fn another_shop_reaches_none_of_it() {
    // Rule 3, over HTTP: the shop is the state's, not the path's.
    let h = harness();
    let id = a_supplier(&h.app, "Sarl Amrani", Some(150_000)).await;
    let theirs = other_shop(&h);

    let (status, _) = call(&theirs, "GET", &format!("/suppliers/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, list) = call(&theirs, "GET", "/suppliers", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 0);
    let (status, _) = call(
        &theirs,
        "POST",
        &format!("/suppliers/{id}/payments"),
        Some(json!({ "amount_centimes": 1_000, "payment_mode": "cash" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_id_that_is_not_a_number_leaves_in_the_same_envelope() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/suppliers/abc/ledger", None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "bad_request");
}

#[tokio::test]
async fn deleting_a_supplier_is_not_a_thing_the_api_does() {
    let h = harness();
    let id = a_supplier(&h.app, "Sarl Amrani", None).await;
    let (status, _) = call(&h.app, "DELETE", &format!("/suppliers/{id}"), None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}
