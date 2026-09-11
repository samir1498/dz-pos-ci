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

mod common;

use common::printed;

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
    common::sign_in(&path, SHOP);
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
    common::signed_in_router(&h.path, 2, &token())
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

/// A company fiche as the screen posts it. Neither RC nor NIF is filled in:
/// the identifiers are required when a facture is issued to a company, not
/// to open its fiche.
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

/// The same fields without the opening debt, which is a create-only field:
/// what a PUT takes.
fn write_body(name: &str) -> Value {
    let mut body = draft(name);
    body.as_object_mut()
        .map(|o| o.remove("opening_debt_centimes"));
    body
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
            "active": false,
            "close_reason": "dossier au contentieux"
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

/// A fiche that still carries something is not closed by a checkbox: the
/// refusal names the `reason` field, which is where the screen puts the
/// message, and the fiche is still open afterwards.
#[tokio::test]
async fn closing_a_fiche_with_a_balance_without_a_reason_is_refused_on_the_field() {
    let h = harness();
    let mut with_debt = draft("Entreprise Benali");
    with_debt["opening_debt_centimes"] = json!(150_000);
    let made = create(&h.app, with_debt).await;
    let id = id_of(&made);

    let mut closing = write_body("Entreprise Benali");
    closing["active"] = json!(false);
    let (status, refused) = call(
        &h.app,
        "PUT",
        &format!("/customers/{id}"),
        Some(closing.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["code"], "validation");
    assert_eq!(
        refused["error"]["field"], "reason",
        "the screen has nowhere to put this message: {refused}"
    );

    let (_, still) = call(&h.app, "GET", &format!("/customers/{id}"), None).await;
    assert_eq!(
        still["active"], true,
        "the refused close was written: {still}"
    );

    closing["close_reason"] = json!("dossier au contentieux");
    let (status, closed) = call(&h.app, "PUT", &format!("/customers/{id}"), Some(closing)).await;
    assert_eq!(status, StatusCode::OK, "{closed}");
    assert_eq!(closed["active"], false);
    assert_eq!(closed["balance_centimes"], 150_000, "the close moved money");
}

/// A fiche with nothing on it is closed the way any field is changed.
#[tokio::test]
async fn closing_a_settled_fiche_asks_for_nothing() {
    let h = harness();
    let made = create(&h.app, draft("Entreprise Benali")).await;
    let mut closing = write_body("Entreprise Benali");
    closing["active"] = json!(false);
    let (status, closed) = call(
        &h.app,
        "PUT",
        &format!("/customers/{}", id_of(&made)),
        Some(closing),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{closed}");
    assert_eq!(closed["active"], false);
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
        ("GET", format!("/customers/{id}/payments"), None),
        (
            "GET",
            format!("/customers/{id}/statement?from=2026-01-01&to=2099-12-31&lang=fr"),
            None,
        ),
        ("GET", format!("/customers/{id}/debt-slip?lang=fr"), None),
        (
            "POST",
            format!("/customers/{id}/payments"),
            Some(json!({ "amount_centimes": 1_000, "payment_mode": "cash", "note": null })),
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
        ("GET", "/customers/1/payments"),
        ("POST", "/customers/1/payments"),
        (
            "GET",
            "/customers/1/statement?from=2026-01-01&to=2099-12-31&lang=fr",
        ),
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

// ---------------------------------------------------------------- payments
//
// The unpaid documents below are written straight through the core, on a
// second connection to the same shop file, rather than sold on credit: the
// credit sale is being written on its own branch and what these assert is the
// settlement half of the contract.

/// A facture made out to `customer` for `net` centimes, unpaid, issued on the
/// day of August 2026 given, so the oldest-first order is a fact of the
/// fixture. August because the payment beside it is stamped by the server's
/// own clock: a fixture dated after that clock would sit on the wrong side of
/// the payment and the running balance of the statement would read backwards.
fn a_facture_on_credit(path: &std::path::Path, customer_id: i32, net: i64, day: u32) -> i32 {
    use dzpos_core::money::{Money, PaymentMode, Regime, Totals};
    use dzpos_core::services::debt::{self, DebtKind, NewDebtEntry};
    use dzpos_core::services::documents::{
        self, BalanceTriple, DocumentKind, NewDocument, PartyBlock, PartyKind, SellerBlock,
    };

    let mut conn = dzpos_core::db::open(path).unwrap();
    let net = Money::centimes(net);
    let issued_at = chrono::NaiveDate::from_ymd_opt(2026, 8, day)
        .and_then(|d| d.and_hms_opt(10, 0, 0))
        .unwrap();
    let before = debt::balance(&mut conn, SHOP, customer_id).unwrap();
    let doc = documents::issue(
        &mut conn,
        SHOP,
        NewDocument {
            kind: DocumentKind::Facture,
            issued_at,
            user_id: 1,
            regime: Regime::Reel,
            payment_mode: PaymentMode::Credit,
            seller: SellerBlock {
                name: "Mon magasin".to_string(),
                rc: None,
                nif: None,
                nis: None,
                ai: None,
                address: None,
                phone: None,
            },
            customer_id: Some(customer_id),
            buyer: Some(PartyBlock {
                name: "Entreprise Benali".to_string(),
                party_kind: PartyKind::Company,
                rc: None,
                nif: None,
                nis: None,
                ai: None,
                address: None,
            }),
            ref_document_id: None,
            balance: Some(BalanceTriple {
                old_balance: before,
                remaining_debt: net,
                total_debt: before.checked_add(net).unwrap(),
            }),
            totals: Totals {
                total_ht: net,
                discount: Money::ZERO,
                subtotal_ht: net,
                tva_by_rate: Vec::new(),
                tva: Money::ZERO,
                total_ttc: net,
                stamp: Money::ZERO,
                net_to_pay: net,
            },
            tendered: None,
            change: None,
            lines: Vec::new(),
        },
    )
    .unwrap();
    // Stamped with the day the facture was issued rather than with the moment
    // the test runs, so the day it lands on is the fixture's and not the
    // calendar's.
    debt::append_at(
        &mut conn,
        SHOP,
        NewDebtEntry {
            customer_id,
            document_id: Some(doc.id),
            kind: DebtKind::Sale,
            debit: net,
            credit: Money::ZERO,
            user_id: 1,
            note: None,
        },
        Some(issued_at),
    )
    .unwrap();
    doc.id
}

fn remaining_debt(path: &std::path::Path, document_id: i32) -> i64 {
    let mut conn = dzpos_core::db::open(path).unwrap();
    dzpos_core::services::documents::get(&mut conn, SHOP, document_id)
        .unwrap()
        .balance
        .expect("a facture issued on credit carries the balance triple")
        .remaining_debt
        .as_centimes()
}

#[tokio::test]
async fn a_payment_settles_the_oldest_facture_first_and_answers_the_new_balance() {
    let h = harness();
    let made = create(&h.app, draft("Entreprise Benali")).await;
    let id = i32::try_from(id_of(&made)).unwrap();
    let first = a_facture_on_credit(&h.path, id, 100_000, 10);
    let second = a_facture_on_credit(&h.path, id, 200_000, 11);

    let (status, answer) = call(
        &h.app,
        "POST",
        &format!("/customers/{id}/payments"),
        Some(json!({ "amount_centimes": 150_000, "payment_mode": "cash", "note": "acompte" })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "{answer}");
    assert_eq!(answer["customer_id"], id);
    assert_eq!(answer["balance_centimes"], 150_000, "{answer}");
    let payment = &answer["payments"][0];
    assert_eq!(payment["amount_centimes"], 150_000);
    assert_eq!(payment["payment_mode"], "cash");
    assert_eq!(payment["note"], "acompte");
    assert_eq!(payment["balance_after_centimes"], 150_000);
    assert_eq!(payment["allocations"][0]["document_id"], first);
    assert_eq!(payment["allocations"][0]["amount_centimes"], 100_000);
    assert_eq!(payment["allocations"][1]["document_id"], second);
    assert_eq!(payment["allocations"][1]["amount_centimes"], 50_000);

    // The document's own column moved with the money, so a reprint says what
    // is left on this facture and not what it asked for.
    assert_eq!(remaining_debt(&h.path, first), 0);
    assert_eq!(remaining_debt(&h.path, second), 150_000);

    // The fiche beside the list carries the same figure.
    let (_, fiche) = call(&h.app, "GET", &format!("/customers/{id}"), None).await;
    assert_eq!(fiche["balance_centimes"], 150_000);
}

#[tokio::test]
async fn a_payment_above_the_debt_is_422_carrying_what_is_outstanding() {
    let h = harness();
    let made = create(&h.app, draft("Entreprise Benali")).await;
    let id = i32::try_from(id_of(&made)).unwrap();
    a_facture_on_credit(&h.path, id, 100_000, 10);

    let (status, answer) = call(
        &h.app,
        "POST",
        &format!("/customers/{id}/payments"),
        Some(json!({ "amount_centimes": 200_000, "payment_mode": "cash", "note": null })),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{answer}");
    assert_eq!(answer["error"]["code"], "validation");
    // "too much" is useless without the amount that would not have been.
    assert_eq!(answer["error"]["field"], "amount_centimes");
    assert_eq!(answer["error"]["outstanding_centimes"], 100_000);

    let (_, payments) = call(&h.app, "GET", &format!("/customers/{id}/payments"), None).await;
    assert_eq!(
        payments["payments"].as_array().map(Vec::len),
        Some(0),
        "a refused payment landed anyway: {payments}"
    );
    assert_eq!(payments["balance_centimes"], 100_000);
}

#[tokio::test]
async fn a_refusal_with_nothing_to_add_carries_no_figures_at_all() {
    let h = harness();
    let made = create(&h.app, draft("Entreprise Benali")).await;
    let id = id_of(&made);

    let (status, answer) = call(
        &h.app,
        "POST",
        &format!("/customers/{id}/adjustments"),
        Some(json!({ "amount_centimes": 0, "note": null })),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{answer}");
    assert_eq!(answer["error"]["code"], "validation");
    // The field it is about is on every validation refusal, so it is what
    // this one carries and the whole of it. The figures below belong to the
    // three refusals that have something to say beyond a code and a
    // sentence, and none of them is this one.
    assert_eq!(answer["error"]["field"], "amount");
    for extra in [
        "outstanding_centimes",
        "balance_after_centimes",
        "credit_limit_centimes",
        "party_side",
        "missing_ids",
    ] {
        assert!(
            answer["error"].get(extra).is_none(),
            "an error with nothing to add grew {extra}: {answer}"
        );
    }
}

#[tokio::test]
async fn the_payments_of_a_customer_read_back_newest_first() {
    let h = harness();
    let made = create(&h.app, draft("Entreprise Benali")).await;
    let id = i32::try_from(id_of(&made)).unwrap();
    a_facture_on_credit(&h.path, id, 100_000, 10);
    let second = a_facture_on_credit(&h.path, id, 200_000, 11);
    for amount in [150_000, 20_000] {
        let (status, answer) = call(
            &h.app,
            "POST",
            &format!("/customers/{id}/payments"),
            Some(json!({ "amount_centimes": amount, "payment_mode": "card", "note": null })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{answer}");
    }

    let (status, answer) = call(&h.app, "GET", &format!("/customers/{id}/payments"), None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(answer["balance_centimes"], 130_000);
    let payments = answer["payments"].as_array().unwrap();
    assert_eq!(payments.len(), 2);
    assert_eq!(payments[0]["amount_centimes"], 20_000);
    assert_eq!(payments[0]["allocations"][0]["document_id"], second);
    assert_eq!(payments[1]["amount_centimes"], 150_000);
}

#[tokio::test]
async fn a_payment_of_nothing_and_a_mode_that_is_not_one_are_both_refused() {
    let h = harness();
    let made = create(&h.app, draft("Entreprise Benali")).await;
    let id = i32::try_from(id_of(&made)).unwrap();
    a_facture_on_credit(&h.path, id, 100_000, 10);

    for (body, code) in [
        (
            json!({ "amount_centimes": 0, "payment_mode": "cash", "note": null }),
            "validation",
        ),
        // Settling a credit with more credit is not a payment.
        (
            json!({ "amount_centimes": 1_000, "payment_mode": "credit", "note": null }),
            "bad_request",
        ),
        (
            json!({ "amount_centimes": 1_000, "payment_mode": "cash", "bogus": 1 }),
            "bad_request",
        ),
    ] {
        let (status, answer) = call(
            &h.app,
            "POST",
            &format!("/customers/{id}/payments"),
            Some(body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}: {answer}");
        assert_eq!(answer["error"]["code"], code, "{body}");
    }

    let (_, payments) = call(&h.app, "GET", &format!("/customers/{id}/payments"), None).await;
    assert_eq!(payments["payments"].as_array().map(Vec::len), Some(0));
}

/// The statement route answers a page, not JSON, so it is called on its own
/// rather than through `call`.
async fn page(app: &axum::Router, uri: &str) -> (StatusCode, String) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, common::OWNER_SESSION)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn the_statement_prints_the_range_with_the_closing_balance_in_words() {
    let h = harness();
    let mut with_debt = draft("Entreprise Benali");
    with_debt["opening_debt_centimes"] = json!(150_000);
    let made = create(&h.app, with_debt).await;
    let id = i32::try_from(id_of(&made)).unwrap();
    a_facture_on_credit(&h.path, id, 200_000, 5);
    let (status, answer) = call(
        &h.app,
        "POST",
        &format!("/customers/{id}/payments"),
        Some(json!({ "amount_centimes": 50_000, "payment_mode": "cash", "note": null })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{answer}");

    // A range wide enough that the movement the route stamped with the
    // server's own clock is inside it whatever day the test runs on.
    let (status, html) = page(
        &h.app,
        &format!("/customers/{id}/statement?from=2026-01-01&to=2099-12-31&lang=fr"),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{html}");
    assert!(html.contains("RELEVÉ DE COMPTE"), "{html}");
    assert!(html.contains("Entreprise Benali"));
    // 3 000,00 owed: 1 500,00 carried over, 2 000,00 on the facture, 500,00
    // paid off it.
    assert!(
        html.contains("amount-closing\">3\u{202f}000,00"),
        "the closing balance is not on the page"
    );
    assert!(html.contains("trois-mille dinars"), "the words are missing");
    // Every movement of the range, and the facture named under the number a
    // customer quotes.
    assert!(html.contains(&printed("FA", 1)));
    assert!(html.contains("Paiement"));
}

#[tokio::test]
async fn a_statement_range_that_is_not_two_days_the_right_way_round_is_422() {
    let h = harness();
    let made = create(&h.app, draft("Entreprise Benali")).await;
    let id = id_of(&made);

    for (query, field) in [
        ("from=2026-1-1&to=2026-09-30&lang=fr", "from"),
        ("from=2026-01-01&to=2026-09-31&lang=fr", "to"),
        ("from=2026-09-30&to=2026-09-01&lang=fr", "to"),
    ] {
        let (status, html) = page(&h.app, &format!("/customers/{id}/statement?{query}")).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{query}: {html}");
        assert!(html.contains("\"validation\""), "{query}: {html}");
        assert!(html.contains(field), "{query}: {html}");
    }

    // A language the app does not print is the caller's mistake and leaves in
    // the same envelope.
    let (status, html) = page(
        &h.app,
        &format!("/customers/{id}/statement?from=2026-01-01&to=2026-09-30&lang=de"),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{html}");
    assert!(html.contains("\"bad_request\""), "{html}");
}

#[tokio::test]
async fn the_debt_slip_prints_the_balance_the_ledger_sums_to_and_says_it_proves_nothing() {
    let h = harness();
    let mut with_debt = draft("Entreprise Benali");
    with_debt["opening_debt_centimes"] = json!(150_000);
    let made = create(&h.app, with_debt).await;
    let id = i32::try_from(id_of(&made)).unwrap();
    a_facture_on_credit(&h.path, id, 200_000, 5);
    let (status, answer) = call(
        &h.app,
        "POST",
        &format!("/customers/{id}/payments"),
        Some(json!({ "amount_centimes": 50_000, "payment_mode": "cash", "note": null })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{answer}");

    let (status, html) = page(&h.app, &format!("/customers/{id}/debt-slip?lang=fr")).await;

    assert_eq!(status, StatusCode::OK, "{html}");
    assert!(html.contains("Situation de compte"), "{html}");
    assert!(html.contains("Entreprise Benali"));
    // The shop's own block, read as it stands today rather than snapshotted:
    // the slip is a paper about the account and carries no series.
    assert!(html.contains("Mon magasin"), "{html}");
    // 3 000,00 owed: 1 500,00 carried over, 2 000,00 on the facture, 500,00
    // paid off it.
    assert!(
        html.contains("amount-balance\">3\u{202f}000,00"),
        "the balance is not on the page"
    );
    assert!(html.contains("trois-mille dinars"), "the words are missing");
    assert!(html.contains(&printed("FA", 1)));
    assert!(html.contains("Paiement"));
    // The line that keeps it out of a comptable's file.
    assert!(html.contains("Document sans valeur fiscale"), "{html}");
}

#[tokio::test]
async fn a_debt_slip_in_a_language_the_app_does_not_print_is_the_callers_mistake() {
    let h = harness();
    let made = create(&h.app, draft("Entreprise Benali")).await;
    let id = id_of(&made);

    let (status, html) = page(&h.app, &format!("/customers/{id}/debt-slip?lang=de")).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{html}");
    assert!(html.contains("\"bad_request\""), "{html}");
}

#[tokio::test]
async fn a_debt_slip_for_a_customer_this_shop_does_not_have_is_not_found() {
    let h = harness();

    let (status, html) = page(&h.app, "/customers/4242/debt-slip?lang=fr").await;

    assert_eq!(status, StatusCode::NOT_FOUND, "{html}");
    assert!(html.contains("\"not_found\""), "{html}");
}
