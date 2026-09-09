// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The customer fiche and its ledger over HTTP. The API is the product's only
//! entry point (architecture.md, consequence of rule 2), so what these assert
//! is the contract the desktop, the browser preview and the phone all read:
//! the list's order and its search, the balance on the fiche, the whole-row
//! update, and the adjustment that corrects a debt without editing a
//! movement.

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

/// The same shop file, answered for as shop 2: the state carries the shop,
/// so this is the whole of what another shop can reach.
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

/// A company fiche as the screen posts it. Neither RC nor NIF is filled in:
/// the identifiers are required when a facture is issued to a company (T4),
/// not to open its fiche.
fn draft(name: &str) -> Value {
    json!({
        "name": name,
        "party_kind": "company",
        "phone": "0770 11 22 33",
        "address": null,
        "rc": null,
        "nif": null,
        "nis": null,
        "ai": null,
        "credit_limit_centimes": 5_000_000,
        "warn_threshold_centimes": 4_000_000,
        "notes": null,
        "active": true,
        "opening_debt_centimes": null
    })
}

async fn create(app: &axum::Router, body: Value) -> Value {
    let (status, made) = call(app, "POST", "/customers", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    made
}

fn id_of(customer: &Value) -> i64 {
    customer["id"].as_i64().expect("the fiche carries no id")
}

#[tokio::test]
async fn customers_start_empty() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/customers", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn a_fiche_is_created_and_read_back_with_its_balance() {
    let h = harness();
    let mut with_debt = draft("Entreprise Benali");
    with_debt["opening_debt_centimes"] = json!(150_000);
    let made = create(&h.app, with_debt).await;
    assert_eq!(made["name"], "Entreprise Benali");
    assert_eq!(made["party_kind"], "company");
    assert_eq!(made["credit_limit_centimes"], 5_000_000);
    assert_eq!(made["nif"], Value::Null);
    assert_eq!(
        made["balance_centimes"], 150_000,
        "the opening debt is on the fiche the create answered: {made}"
    );

    let (status, read) = call(&h.app, "GET", &format!("/customers/{}", id_of(&made)), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read, made);

    let entries = call(
        &h.app,
        "GET",
        &format!("/customers/{}/ledger", id_of(&made)),
        None,
    )
    .await
    .1;
    assert_eq!(entries["balance_centimes"], 150_000);
    assert_eq!(entries["entries"][0]["kind"], "opening");
    assert_eq!(entries["entries"][0]["debit_centimes"], 150_000);
    assert_eq!(entries["entries"][0]["balance_after_centimes"], 150_000);
}

#[tokio::test]
async fn the_list_reads_active_first_then_by_name_and_searches_name_or_phone() {
    let h = harness();
    let mut closed = draft("Ali");
    closed["active"] = json!(false);
    create(&h.app, closed).await;
    create(&h.app, draft("Zoubir")).await;
    let mut brahim = draft("Brahim Khelifi");
    brahim["phone"] = json!("0555 99 88 77");
    create(&h.app, brahim).await;

    let (status, body) = call(&h.app, "GET", "/customers", None).await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect();
    assert_eq!(names, ["Brahim Khelifi", "Zoubir", "Ali"], "{body}");

    let (_, found) = call(&h.app, "GET", "/customers?q=khel", None).await;
    assert_eq!(found.as_array().map(Vec::len), Some(1), "{found}");
    assert_eq!(found[0]["name"], "Brahim Khelifi");

    let (_, by_phone) = call(&h.app, "GET", "/customers?q=99%2088", None).await;
    assert_eq!(by_phone.as_array().map(Vec::len), Some(1), "{by_phone}");

    let (_, blank) = call(&h.app, "GET", "/customers?q=", None).await;
    assert_eq!(
        blank.as_array().map(Vec::len),
        Some(3),
        "an emptied search box reads the whole list: {blank}"
    );
}

#[tokio::test]
async fn an_update_carries_the_whole_row_and_a_null_clears_a_field() {
    let h = harness();
    let mut with_ids = draft("Entreprise Benali");
    with_ids["rc"] = json!("16/00-7654321 B 22");
    with_ids["opening_debt_centimes"] = json!(150_000);
    let made = create(&h.app, with_ids).await;

    let (status, after) = call(
        &h.app,
        "PUT",
        &format!("/customers/{}", id_of(&made)),
        Some(json!({
            "name": "Entreprise Benali et fils",
            "party_kind": "consumer",
            "phone": null,
            "address": null,
            "rc": null,
            "nif": null,
            "nis": null,
            "ai": null,
            "credit_limit_centimes": null,
            "warn_threshold_centimes": null,
            "notes": "passe le jeudi",
            "active": false
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after["name"], "Entreprise Benali et fils");
    assert_eq!(after["party_kind"], "consumer");
    assert_eq!(
        after["rc"],
        Value::Null,
        "a cleared identifier stayed: {after}"
    );
    assert_eq!(after["credit_limit_centimes"], Value::Null);
    assert_eq!(after["active"], false);
    assert_eq!(
        after["balance_centimes"], 150_000,
        "the update touched the ledger: {after}"
    );
}

/// The opening debt is not a field of the fiche, so the update type does not
/// take it: sending it is a mistake the edge names rather than swallows.
#[tokio::test]
async fn an_update_that_tries_to_set_the_opening_debt_is_refused() {
    let h = harness();
    let made = create(&h.app, draft("Entreprise Benali")).await;
    let mut body = draft("Entreprise Benali");
    body["opening_debt_centimes"] = json!(999);
    let (status, answer) = call(
        &h.app,
        "PUT",
        &format!("/customers/{}", id_of(&made)),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{answer}");
    assert_eq!(answer["error"]["code"], "bad_request");
}

#[tokio::test]
async fn a_field_longer_than_a_facture_prints_is_422_naming_it() {
    let h = harness();
    let mut long = draft("Entreprise Benali");
    long["address"] = json!("a".repeat(201));
    let (status, body) = call(&h.app, "POST", "/customers", Some(long)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("address "),
        "{body}"
    );
    let (_, list) = call(&h.app, "GET", "/customers", None).await;
    assert_eq!(list.as_array().map(Vec::len), Some(0));
}

/// The search box reaches a LIKE pattern, so it is bounded like the fields
/// that are stored rather than left to carry whatever a paste holds.
#[tokio::test]
async fn a_search_longer_than_a_field_is_422_naming_it() {
    let h = harness();
    create(&h.app, draft("Entreprise Benali")).await;
    let (ok, found) = call(
        &h.app,
        "GET",
        &format!("/customers?q={}", "e".repeat(200)),
        None,
    )
    .await;
    assert_eq!(ok, StatusCode::OK, "{found}");
    assert_eq!(found.as_array().unwrap().len(), 0);

    let (status, body) = call(
        &h.app,
        "GET",
        &format!("/customers?q={}", "e".repeat(201)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");
    assert!(
        body["error"]["message"].as_str().unwrap().starts_with("q "),
        "{body}"
    );
}

#[tokio::test]
async fn an_adjustment_writes_a_movement_and_answers_the_new_balance() {
    let h = harness();
    let mut with_debt = draft("Entreprise Benali");
    with_debt["opening_debt_centimes"] = json!(150_000);
    let made = create(&h.app, with_debt).await;
    let id = id_of(&made);

    let (status, ledger) = call(
        &h.app,
        "POST",
        &format!("/customers/{id}/adjustments"),
        Some(json!({ "amount_centimes": -50_000, "note": "erreur de saisie" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{ledger}");
    assert_eq!(ledger["balance_centimes"], 100_000);
    assert_eq!(ledger["entries"][0]["kind"], "adjustment");
    assert_eq!(ledger["entries"][0]["credit_centimes"], 50_000);
    assert_eq!(ledger["entries"][0]["debit_centimes"], 0);
    assert_eq!(ledger["entries"][0]["balance_after_centimes"], 100_000);
    assert_eq!(ledger["entries"][0]["note"], "erreur de saisie");
    assert_eq!(
        ledger["entries"][1]["balance_after_centimes"], 150_000,
        "the older movement kept the balance it left behind: {ledger}"
    );

    let (_, read) = call(&h.app, "GET", &format!("/customers/{id}"), None).await;
    assert_eq!(read["balance_centimes"], 100_000, "{read}");
}

#[tokio::test]
async fn an_adjustment_of_nothing_is_422_and_writes_no_movement() {
    let h = harness();
    let made = create(&h.app, draft("Entreprise Benali")).await;
    let id = id_of(&made);
    let (status, body) = call(
        &h.app,
        "POST",
        &format!("/customers/{id}/adjustments"),
        Some(json!({ "amount_centimes": 0, "note": null })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("amount "),
        "{body}"
    );
    let (_, ledger) = call(&h.app, "GET", &format!("/customers/{id}/ledger"), None).await;
    assert_eq!(ledger["entries"].as_array().map(Vec::len), Some(0));
}

/// Rule 3, over HTTP: the shop is the server's, never the caller's, so
/// another shop's id is a 404 with nothing in it that says the row exists.
#[tokio::test]
async fn another_shops_customer_is_not_found_on_every_route() {
    let h = harness();
    let mut with_debt = draft("Entreprise Benali");
    with_debt["opening_debt_centimes"] = json!(150_000);
    let made = create(&h.app, with_debt).await;
    let id = id_of(&made);
    let other = other_shop(&h);

    for (method, uri, body) in [
        ("GET", format!("/customers/{id}"), None),
        (
            "PUT",
            format!("/customers/{id}"),
            Some(json!({
                "name": "Repris",
                "party_kind": "company",
                "phone": null,
                "address": null,
                "rc": null,
                "nif": null,
                "nis": null,
                "ai": null,
                "credit_limit_centimes": null,
                "warn_threshold_centimes": null,
                "notes": null,
                "active": true
            })),
        ),
        ("GET", format!("/customers/{id}/ledger"), None),
        (
            "POST",
            format!("/customers/{id}/adjustments"),
            Some(json!({ "amount_centimes": 1_000, "note": null })),
        ),
    ] {
        let (status, answer) = call(&other, method, &uri, body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}: {answer}");
        assert_eq!(answer["error"]["code"], "not_found");
        assert!(
            !answer["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Benali"),
            "the answer named the other shop's row: {answer}"
        );
    }

    let (_, list) = call(&other, "GET", "/customers", None).await;
    assert_eq!(list.as_array().map(Vec::len), Some(0));
    let (_, mine) = call(&h.app, "GET", &format!("/customers/{id}"), None).await;
    assert_eq!(mine["name"], "Entreprise Benali");
    assert_eq!(
        mine["balance_centimes"], 150_000,
        "the refused write landed anyway: {mine}"
    );
}

#[tokio::test]
async fn every_customer_route_needs_the_launch_token() {
    let h = harness();
    for (method, uri) in [
        ("GET", "/customers"),
        ("POST", "/customers"),
        ("GET", "/customers/1"),
        ("PUT", "/customers/1"),
        ("GET", "/customers/1/ledger"),
        ("POST", "/customers/1/adjustments"),
    ] {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let res = h.app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri} answered without the token"
        );
    }
}

#[tokio::test]
async fn an_id_that_is_not_a_number_is_the_same_envelope_as_every_other_refusal() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/customers/abc/ledger", None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "bad_request");
}

/// A centime figure a JSON number cannot carry without rounding is refused at
/// the edge rather than stored as something near it: past 2^53 - 1 the number
/// that comes back is not the number that was sent (dto, `within_js_safe_range`).
#[tokio::test]
async fn an_amount_past_the_safe_integer_bound_is_422_naming_the_field() {
    let h = harness();
    let past = 9_007_199_254_740_992i64;

    let mut huge = draft("Entreprise Benali");
    huge["credit_limit_centimes"] = json!(past);
    let (status, body) = call(&h.app, "POST", "/customers", Some(huge)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("credit_limit_centimes "),
        "{body}"
    );

    let made = create(&h.app, draft("Entreprise Benali")).await;
    let (status, body) = call(
        &h.app,
        "POST",
        &format!("/customers/{}/adjustments", id_of(&made)),
        Some(json!({ "amount_centimes": -past, "note": null })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("amount_centimes "),
        "{body}"
    );

    let (_, ledger) = call(
        &h.app,
        "GET",
        &format!("/customers/{}/ledger", id_of(&made)),
        None,
    )
    .await;
    assert_eq!(
        ledger["entries"].as_array().unwrap().len(),
        0,
        "the refused correction landed anyway: {ledger}"
    );
}
