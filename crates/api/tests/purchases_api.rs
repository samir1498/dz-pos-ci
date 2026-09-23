// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// S7 of `a-kernel-crate-and-retail-as-the-first-module`: purchase orders,
// deliveries and returns are all retail-only (`routes::mod.rs`), so this
// whole file has nothing to compile with the feature off.
#![cfg(feature = "retail")]

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
    // The two cost columns as they were sent, and the one amount the core
    // added out of them: 100,00 of transport and nothing else is 100,00.
    assert_eq!(made["purchase"]["transport_centimes"], 10_000);
    assert_eq!(made["purchase"]["extra_costs_centimes"], 0);
    assert_eq!(made["purchase"]["extras_centimes"], 10_000);
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

/// The two cost columns cross as they are stored, and their sum crosses with
/// them: the screens print what the goods cost to get here under one heading,
/// and before T9 two components added the two columns inside their JSX. The
/// expected figures here are written from the rule — migration 000008 keeps
/// transport and the other extra costs in two columns because they are two
/// things agreed once for the whole order — and not read back off the answer.
#[tokio::test]
async fn the_extras_are_answered_as_one_amount_by_every_route_that_names_an_order() {
    let h = harness();
    let supplier = a_supplier(&h.app, "Sarl Amrani").await;
    let product = a_product(&h.app, "Farine 5kg").await;

    // 100,00 of transport and 25,00 of other costs is 125,00.
    let mut order = an_order(supplier, product);
    order["transport_centimes"] = json!(10_000);
    order["extra_costs_centimes"] = json!(2_500);
    let (status, made) = call(&h.app, "POST", "/purchases", Some(order)).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["purchase"]["extras_centimes"], 12_500);

    // The same order read back, and read again in the list: one answer, not
    // three chances to disagree.
    let id = made["purchase"]["id"].as_i64().unwrap();
    let (_, one) = call(&h.app, "GET", &format!("/purchases/{id}"), None).await;
    assert_eq!(one["purchase"]["extras_centimes"], 12_500);
    let (_, all) = call(&h.app, "GET", "/purchases", None).await;
    assert_eq!(all[0]["extras_centimes"], 12_500);

    // An order that cost nothing to bring in answers zero, not nothing: the
    // list prints a figure on every row.
    let (_, free) = call(
        &h.app,
        "POST",
        "/purchases",
        Some(an_order(supplier, product)),
    )
    .await;
    assert_eq!(free["purchase"]["transport_centimes"], 0);
    assert_eq!(free["purchase"]["extra_costs_centimes"], 0);
    assert_eq!(free["purchase"]["extras_centimes"], 0);

    // Only one of the two set is the common shape, and the empty column must
    // not take the other one with it.
    let mut carried = an_order(supplier, product);
    carried["extra_costs_centimes"] = json!(2_500);
    let (_, carried) = call(&h.app, "POST", "/purchases", Some(carried)).await;
    assert_eq!(carried["purchase"]["extras_centimes"], 2_500);

    // The four routes that change an order answer it too, each through its
    // own conversion, and a conversion nothing calls is where a wrong figure
    // would sit unnoticed. Driving them is the only way to tell.
    let mut moving = an_order(supplier, product);
    moving["transport_centimes"] = json!(10_000);
    moving["extra_costs_centimes"] = json!(2_500);
    let (_, moving) = call(&h.app, "POST", "/purchases", Some(moving)).await;
    let moving_id = moving["purchase"]["id"].as_i64().unwrap();
    let moving_line = moving["lines"][0]["id"].as_i64().unwrap();

    let (status, received) = call(
        &h.app,
        "POST",
        &format!("/purchases/{moving_id}/receipts"),
        Some(json!({ "lines": [{ "purchase_line_id": moving_line, "qty_milli": 6_000 }] })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{received}");
    assert_eq!(received["purchase"]["extras_centimes"], 12_500);

    let (status, returned) = call(
        &h.app,
        "POST",
        &format!("/purchases/{moving_id}/returns"),
        Some(json!({ "lines": [{ "purchase_line_id": moving_line, "qty_milli": 1_000 }] })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{returned}");
    assert_eq!(returned["purchase"]["extras_centimes"], 12_500);

    let (status, short) = call(
        &h.app,
        "POST",
        &format!("/purchases/{moving_id}/close-short"),
        Some(json!({ "reason": "le fournisseur ne livre plus" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{short}");
    assert_eq!(short["purchase"]["extras_centimes"], 12_500);

    // A cancellation only succeeds before a delivery, so it needs an order of
    // its own. Until this block nothing in the suite reached that success
    // path: the one other `/cancel` call is the refusal after a delivery.
    let mut doomed = an_order(supplier, product);
    doomed["transport_centimes"] = json!(10_000);
    doomed["extra_costs_centimes"] = json!(2_500);
    let (_, doomed) = call(&h.app, "POST", "/purchases", Some(doomed)).await;
    let doomed_id = doomed["purchase"]["id"].as_i64().unwrap();
    let (status, cancelled) = call(
        &h.app,
        "POST",
        &format!("/purchases/{doomed_id}/cancel"),
        Some(json!({ "reason": "commande en double" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert_eq!(cancelled["purchase"]["extras_centimes"], 12_500);
}

/// A cost cannot be negative: the column is `INTEGER >= 0` in migration 000008
/// and the service refuses the field before the row is written, so the sum
/// above is never asked to answer for one.
#[tokio::test]
async fn a_negative_cost_is_refused_on_its_own_field_and_never_reaches_the_sum() {
    let h = harness();
    let supplier = a_supplier(&h.app, "Sarl Amrani").await;
    let product = a_product(&h.app, "Farine 5kg").await;
    let mut order = an_order(supplier, product);
    order["extra_costs_centimes"] = json!(-1);
    let (status, refused) = call(&h.app, "POST", "/purchases", Some(order)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["error"]["field"], "extra_costs_centimes");
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
