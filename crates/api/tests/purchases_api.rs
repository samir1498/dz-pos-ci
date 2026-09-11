// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Orders, deliveries and returns over HTTP. The API is the product's only
//! entry point (architecture.md, consequence of rule 2), so what these assert
//! is the contract the desktop reads: the whole order comes back on every
//! change, the stock and the supplier's balance move on a delivery and not on
//! a save, and a refusal names the field it is about.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

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

/// The same shop file, answered for as shop 2: the state carries the shop, so
/// this is the whole of what another shop can reach.
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

async fn a_supplier(app: &axum::Router, name: &str) -> i64 {
    let (status, made) = call(
        app,
        "POST",
        "/suppliers",
        Some(json!({
            "name": name,
            "phone": null,
            "address": null,
            "rc": null,
            "nif": null,
            "nis": null,
            "ai": null,
            "notes": null,
            "active": true,
            "opening_debt_centimes": null
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    made["id"].as_i64().unwrap()
}

async fn a_product(app: &axum::Router, name: &str) -> i64 {
    let (status, made) = call(
        app,
        "POST",
        "/products",
        Some(json!({
            "name": name,
            "barcode": null,
            "category_id": null,
            "unit": "piece",
            "cost_centimes": 0,
            "selling_centimes": 30_000,
            "wholesale_centimes": null,
            "qty_on_hand_milli": 0,
            "low_stock_at_milli": 0,
            "rate_bps": 1900,
            "active": true
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    made["id"].as_i64().unwrap()
}

fn an_order(supplier_id: i64, product_id: i64) -> Value {
    json!({
        "supplier_id": supplier_id,
        "purchase_date": "2026-09-10",
        "lines": [
            { "product_id": product_id, "qty_ordered_milli": 10_000, "unit_cost_centimes": 20_000 }
        ]
    })
}

async fn balance(app: &axum::Router, supplier_id: i64) -> i64 {
    let (_, one) = call(app, "GET", &format!("/suppliers/{supplier_id}"), None).await;
    one["balance_centimes"].as_i64().unwrap()
}

async fn on_hand(app: &axum::Router, product_id: i64) -> i64 {
    let (_, list) = call(app, "GET", "/products", None).await;
    list.as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"].as_i64() == Some(product_id))
        .unwrap()["qty_on_hand_milli"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn an_order_is_written_and_read_back_with_its_lines_and_no_delivery() {
    let h = harness();
    let supplier = a_supplier(&h.app, "Sarl Amrani").await;
    let product = a_product(&h.app, "Farine 5kg").await;
    let (status, made) = call(
        &h.app,
        "POST",
        "/purchases",
        Some(an_order(supplier, product)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["purchase"]["status"], "ordered");
    assert_eq!(made["lines"].as_array().unwrap().len(), 1);
    assert!(made["receipts"].as_array().unwrap().is_empty());
    // Nothing moved: debt follows goods, and no goods have arrived.
    assert_eq!(balance(&h.app, supplier).await, 0);
    assert_eq!(on_hand(&h.app, product).await, 0);

    let id = made["purchase"]["id"].as_i64().unwrap();
    let (status, one) = call(&h.app, "GET", &format!("/purchases/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(one["purchase"]["id"], id);
    // Another shop reaches none of it.
    let (status, _) = call(&other_shop(&h), "GET", &format!("/purchases/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_extra_costs_land_on_the_lines_and_the_delivery_moves_stock_and_debt() {
    let h = harness();
    let supplier = a_supplier(&h.app, "Sarl Amrani").await;
    let product = a_product(&h.app, "Farine 5kg").await;
    let mut order = an_order(supplier, product);
    order["transport_centimes"] = json!(10_000);
    order["receive_now"] = json!(true);
    let (status, made) = call(&h.app, "POST", "/purchases", Some(order)).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    // 10 000 centimes over ten units is 1 000 a unit on top of 20 000.
    assert_eq!(made["lines"][0]["landed_unit_cost_centimes"], 21_000);
    assert_eq!(made["purchase"]["status"], "received");
    assert_eq!(made["receipts"][0]["series"], "reception:2026");
    assert_eq!(made["receipts"][0]["number"], 1);
    assert_eq!(on_hand(&h.app, product).await, 10_000);
    assert_eq!(balance(&h.app, supplier).await, 210_000);
}

#[tokio::test]
async fn a_delivery_in_two_parts_answers_the_whole_order_each_time() {
    let h = harness();
    let supplier = a_supplier(&h.app, "Sarl Amrani").await;
    let product = a_product(&h.app, "Farine 5kg").await;
    let (_, made) = call(
        &h.app,
        "POST",
        "/purchases",
        Some(an_order(supplier, product)),
    )
    .await;
    let id = made["purchase"]["id"].as_i64().unwrap();
    let line = made["lines"][0]["id"].as_i64().unwrap();

    let (status, first) = call(
        &h.app,
        "POST",
        &format!("/purchases/{id}/receipts"),
        Some(json!({ "lines": [{ "purchase_line_id": line, "qty_milli": 4_000 }] })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    assert_eq!(first["purchase"]["status"], "partially_received");
    assert_eq!(first["lines"][0]["qty_received_milli"], 4_000);
    assert_eq!(balance(&h.app, supplier).await, 80_000);

    let (status, second) = call(
        &h.app,
        "POST",
        &format!("/purchases/{id}/receipts"),
        Some(json!({ "lines": [{ "purchase_line_id": line, "qty_milli": 6_000 }] })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{second}");
    assert_eq!(second["purchase"]["status"], "received");
    assert_eq!(second["receipts"].as_array().unwrap().len(), 2);
    assert_eq!(on_hand(&h.app, product).await, 10_000);
    assert_eq!(balance(&h.app, supplier).await, 200_000);
}

#[tokio::test]
async fn a_return_takes_the_goods_and_the_debt_back_off() {
    let h = harness();
    let supplier = a_supplier(&h.app, "Sarl Amrani").await;
    let product = a_product(&h.app, "Farine 5kg").await;
    let mut order = an_order(supplier, product);
    order["receive_now"] = json!(true);
    let (_, made) = call(&h.app, "POST", "/purchases", Some(order)).await;
    let id = made["purchase"]["id"].as_i64().unwrap();
    let line = made["lines"][0]["id"].as_i64().unwrap();

    let (status, after) = call(
        &h.app,
        "POST",
        &format!("/purchases/{id}/returns"),
        Some(json!({
            "lines": [{ "purchase_line_id": line, "qty_milli": 3_000 }],
            "note": "trois sacs éventrés"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{after}");
    assert_eq!(after["lines"][0]["qty_returned_milli"], 3_000);
    assert_eq!(on_hand(&h.app, product).await, 7_000);
    assert_eq!(balance(&h.app, supplier).await, 140_000);

    // More than arrived is refused, and nothing of it landed.
    let (status, refused) = call(
        &h.app,
        "POST",
        &format!("/purchases/{id}/returns"),
        Some(json!({ "lines": [{ "purchase_line_id": line, "qty_milli": 8_000 }] })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["code"], "validation");
    assert_eq!(on_hand(&h.app, product).await, 7_000);
}

#[tokio::test]
async fn an_order_is_cancelled_before_a_delivery_and_closed_short_after_one() {
    let h = harness();
    let supplier = a_supplier(&h.app, "Sarl Amrani").await;
    let product = a_product(&h.app, "Farine 5kg").await;
    let (_, made) = call(
        &h.app,
        "POST",
        "/purchases",
        Some(an_order(supplier, product)),
    )
    .await;
    let id = made["purchase"]["id"].as_i64().unwrap();
    let line = made["lines"][0]["id"].as_i64().unwrap();

    // A close short before anything arrived has nothing to close short.
    let (status, refused) = call(
        &h.app,
        "POST",
        &format!("/purchases/{id}/close-short"),
        Some(json!({ "reason": "rien ne viendra" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["field"], "status");

    call(
        &h.app,
        "POST",
        &format!("/purchases/{id}/receipts"),
        Some(json!({ "lines": [{ "purchase_line_id": line, "qty_milli": 4_000 }] })),
    )
    .await;

    // And now the cancel is the one with nothing to do.
    let (status, refused) = call(
        &h.app,
        "POST",
        &format!("/purchases/{id}/cancel"),
        Some(json!({ "reason": "annulée" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["field"], "status");

    let (status, after) = call(
        &h.app,
        "POST",
        &format!("/purchases/{id}/close-short"),
        Some(json!({ "reason": "le fournisseur ne livre plus" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after["purchase"]["status"], "closed_short");
    // What arrived stays owed; what never came owes nothing.
    assert_eq!(balance(&h.app, supplier).await, 80_000);
}

#[tokio::test]
async fn the_list_is_narrowed_by_state_and_by_supplier() {
    let h = harness();
    let amrani = a_supplier(&h.app, "Sarl Amrani").await;
    let bouzid = a_supplier(&h.app, "Sarl Bouzid").await;
    let product = a_product(&h.app, "Farine 5kg").await;
    call(
        &h.app,
        "POST",
        "/purchases",
        Some(an_order(amrani, product)),
    )
    .await;
    let mut second = an_order(bouzid, product);
    second["receive_now"] = json!(true);
    call(&h.app, "POST", "/purchases", Some(second)).await;

    let (status, all) = call(&h.app, "GET", "/purchases", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(all.as_array().unwrap().len(), 2);

    let (_, ordered) = call(&h.app, "GET", "/purchases?status=ordered", None).await;
    assert_eq!(ordered.as_array().unwrap().len(), 1);
    assert_eq!(ordered[0]["supplier_id"], amrani);

    let (_, of_bouzid) = call(
        &h.app,
        "GET",
        &format!("/purchases?supplier_id={bouzid}"),
        None,
    )
    .await;
    assert_eq!(of_bouzid.as_array().unwrap().len(), 1);
    assert_eq!(of_bouzid[0]["status"], "received");

    // A state nobody wrote is the caller's mistake, not an empty list.
    let (status, _) = call(&h.app, "GET", "/purchases?status=delivered", None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn money_handed_over_above_what_the_order_is_worth_is_refused_on_its_field() {
    let h = harness();
    let supplier = a_supplier(&h.app, "Sarl Amrani").await;
    let product = a_product(&h.app, "Farine 5kg").await;
    let mut order = an_order(supplier, product);
    order["paid_now"] = json!({ "amount_centimes": 200_001, "payment_mode": "cash" });
    let (status, refused) = call(&h.app, "POST", "/purchases", Some(order)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["field"], "paid_now_centimes");
    // Nothing was written on the way past.
    let (_, all) = call(&h.app, "GET", "/purchases", None).await;
    assert!(all.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn money_handed_over_with_the_goods_settles_the_order() {
    let h = harness();
    let supplier = a_supplier(&h.app, "Sarl Amrani").await;
    let product = a_product(&h.app, "Farine 5kg").await;
    let mut order = an_order(supplier, product);
    order["receive_now"] = json!(true);
    order["paid_now"] = json!({ "amount_centimes": 50_000, "payment_mode": "card" });
    let (status, made) = call(&h.app, "POST", "/purchases", Some(order)).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(balance(&h.app, supplier).await, 150_000);

    let (_, ledger) = call(
        &h.app,
        "GET",
        &format!("/suppliers/{supplier}/ledger"),
        None,
    )
    .await;
    let payment = ledger["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "payment")
        .unwrap();
    assert_eq!(payment["payment_mode"], "card");
    assert_eq!(payment["allocations"][0]["amount_centimes"], 50_000);
}

#[tokio::test]
async fn a_body_the_form_should_not_send_is_refused_at_the_edge() {
    let h = harness();
    let supplier = a_supplier(&h.app, "Sarl Amrani").await;
    let product = a_product(&h.app, "Farine 5kg").await;

    // No line at all.
    let mut empty = an_order(supplier, product);
    empty["lines"] = json!([]);
    let (status, refused) = call(&h.app, "POST", "/purchases", Some(empty)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["field"], "lines");

    // A day spelled another way.
    let mut wrong_day = an_order(supplier, product);
    wrong_day["purchase_date"] = json!("10/09/2026");
    let (status, refused) = call(&h.app, "POST", "/purchases", Some(wrong_day)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["field"], "purchase_date");

    // A field nobody declared.
    let mut extra = an_order(supplier, product);
    extra["discount_centimes"] = json!(100);
    let (status, _) = call(&h.app, "POST", "/purchases", Some(extra)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // And the id in the path.
    let (status, _) = call(&h.app, "GET", "/purchases/abc", None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
