// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The API crate is the product's only entry point, so these are the
//! product's tests (architecture.md, consequence of rule 2). In-process
//! router, real temp SQLite file.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const SHOP: i32 = 1;

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
        app: dzpos_api::router(state),
    }
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let req = Request::builder().method(method).uri(uri);
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

/// The CORS preflight a browser sends before a cross-origin POST, and the
/// `access-control-allow-origin` the server answered with. `None` means the
/// browser will refuse to hand the answer to the page.
async fn preflight(app: &axum::Router, origin: &str) -> Option<String> {
    let req = Request::builder()
        .method("OPTIONS")
        .uri("/products")
        .header("origin", origin)
        .header("access-control-request-method", "POST")
        .header("access-control-request-headers", "content-type")
        .body(Body::empty())
        .unwrap();
    allowed_origin(app.clone().oneshot(req).await.unwrap())
}

fn allowed_origin(res: axum::response::Response) -> Option<String> {
    res.headers()
        .get("access-control-allow-origin")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

fn draft() -> Value {
    json!({
        "name": "Huile Elio 5L",
        "barcode": null,
        "category_id": 1,
        "unit": "piece",
        "cost_centimes": 820,
        "selling_centimes": 920,
        "wholesale_centimes": null,
        "qty_on_hand_milli": 24000,
        "low_stock_at_milli": 10000,
        "rate_bps": null,
        "active": true
    })
}

#[tokio::test]
async fn health_is_ok() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn products_start_empty() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/products", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn create_then_list_one() {
    let h = harness();
    let (status, made) = call(&h.app, "POST", "/products", Some(draft())).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["name"], "Huile Elio 5L");
    assert_eq!(made["selling_centimes"], 920);
    assert_eq!(made["rate_bps"], 1900);
    assert!(made["id"].is_number());

    let (status, list) = call(&h.app, "GET", "/products", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().map(Vec::len), Some(1));
    assert_eq!(list[0]["id"], made["id"]);
}

#[tokio::test]
async fn money_crosses_the_wire_as_an_integer_number_of_centimes() {
    // architecture.md, contract between Rust and TypeScript.
    let h = harness();
    let mut d = draft();
    d["selling_centimes"] = json!(1_234_567);
    let (_, made) = call(&h.app, "POST", "/products", Some(d)).await;
    assert_eq!(made["selling_centimes"], json!(1_234_567));
    assert!(
        made["selling_centimes"].is_i64(),
        "not an integer JSON number"
    );
}

#[tokio::test]
async fn a_duplicate_barcode_is_409() {
    let h = harness();
    let mut d = draft();
    d["barcode"] = json!("6130001000018");
    let (status, _) = call(&h.app, "POST", "/products", Some(d.clone())).await;
    assert_eq!(status, StatusCode::CREATED);

    d["name"] = json!("Autre");
    let (status, body) = call(&h.app, "POST", "/products", Some(d)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "duplicate_barcode");
    assert!(body["error"]["message"].is_string());
}

#[tokio::test]
async fn one_barcode_may_exist_once_in_each_shop() {
    // Over the wire this time: the 409 above is per shop, not global.
    use diesel::prelude::*;

    let h = harness();
    let mut d = draft();
    d["barcode"] = json!("6130001000018");
    let (status, made) = call(&h.app, "POST", "/products", Some(d.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");

    // A second shop in the same file. There is no shops route yet, so the
    // row goes in raw; what is under test is the index, never this seed.
    let mut seed = dzpos_core::db::open(&h.path).unwrap();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut seed)
        .unwrap();

    let other = dzpos_api::router(dzpos_api::AppState::open(&h.path, 2).unwrap());
    // Shop 2 has no categories of its own, so it names its rate.
    d["category_id"] = json!(null);
    d["rate_bps"] = json!(1900);
    let (status, body) = call(&other, "POST", "/products", Some(d)).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["shop_id"], 2);
    assert_eq!(body["barcode"], "6130001000018");
}

#[tokio::test]
async fn a_validation_failure_is_422() {
    let h = harness();
    let mut d = draft();
    d["name"] = json!("   ");
    let (status, body) = call(&h.app, "POST", "/products", Some(d)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "validation");

    let mut e = draft();
    e["selling_centimes"] = json!(-1);
    let (status, body) = call(&h.app, "POST", "/products", Some(e)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "validation");
}

#[tokio::test]
async fn an_amount_past_what_a_json_number_carries_is_refused_at_the_edge() {
    // 2^53 rounds silently in every JavaScript caller; 2^53 - 1 does not.
    // The bound in dto.rs is enforced, not only written in a comment.
    let h = harness();
    for field in ["selling_centimes", "cost_centimes", "qty_on_hand_milli"] {
        let mut d = draft();
        d[field] = json!(9_007_199_254_740_992_i64);
        let (status, body) = call(&h.app, "POST", "/products", Some(d)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{field}");
        assert_eq!(body["error"]["code"], "validation", "{field}");
    }
    let mut ok = draft();
    ok["name"] = json!("Juste en dessous");
    ok["selling_centimes"] = json!(9_007_199_254_740_991_i64);
    let (status, made) = call(&h.app, "POST", "/products", Some(ok)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(made["selling_centimes"], json!(9_007_199_254_740_991_i64));
}

#[tokio::test]
async fn a_rate_above_one_whole_is_422_with_the_codes_own_name() {
    // architecture.md: the API maps the core's code, it never derives one.
    // The core calls this `money`, so the wire says `money` and the UI's
    // error_money key is what renders.
    let h = harness();
    let mut d = draft();
    d["rate_bps"] = json!(190_000);
    let (status, body) = call(&h.app, "POST", "/products", Some(d)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "money");
}

#[tokio::test]
async fn a_money_error_coming_out_of_a_read_is_a_storage_failure() {
    // A rate the migration's CHECK refuses cannot be written any more, so a
    // row carrying one is a file written by something else: 500, not a 422
    // telling the user to correct a form they never filled in.
    use diesel::prelude::*;

    let h = harness();
    let (status, made) = call(&h.app, "POST", "/products", Some(draft())).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");

    // diesel lives here only to plant a row no service can write; the
    // product's own layers still reach the file through dzpos_core.
    let mut planted = dzpos_core::db::open(&h.path).unwrap();
    diesel::sql_query("PRAGMA ignore_check_constraints = ON")
        .execute(&mut planted)
        .unwrap();
    diesel::sql_query("UPDATE products SET rate_bps = 190000")
        .execute(&mut planted)
        .unwrap();

    let (status, body) = call(&h.app, "GET", "/products", None).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(body["error"]["code"], "money");
}

#[tokio::test]
async fn a_malformed_body_is_422_with_a_message_that_quotes_no_serde() {
    // The message used to be serde's own text, internals and all: "Failed to
    // parse the request body as JSON: name: EOF while parsing...". It says
    // nothing a caller can act on and it leaks the DTO's shape.
    let h = harness();
    for body in [json!({ "name": 3 }), json!({}), json!("nope")] {
        let (status, got) = call(&h.app, "POST", "/products", Some(body)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(got["error"]["code"], "bad_request");
        assert_eq!(got["error"]["message"], "invalid JSON body");
    }
}

#[tokio::test]
async fn an_unknown_field_is_named_without_listing_the_ones_that_exist() {
    let h = harness();
    let mut d = draft();
    d["couleur"] = json!("rouge");
    let (status, body) = call(&h.app, "POST", "/products", Some(d)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "bad_request");
    assert_eq!(body["error"]["message"], "unknown field couleur");
}

#[tokio::test]
async fn an_unknown_unit_is_rejected_before_it_reaches_the_database() {
    let h = harness();
    let mut d = draft();
    d["unit"] = json!("barrel");
    let (status, body) = call(&h.app, "POST", "/products", Some(d)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "bad_request");
}

#[tokio::test]
async fn an_unknown_route_is_404_with_the_same_error_shape() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
}

#[tokio::test]
async fn every_error_body_has_a_code_and_a_message() {
    // The UI translates the code; the message is for a log, never a screen.
    let h = harness();
    let mut d = draft();
    d["category_id"] = json!(4242);
    let (status, body) = call(&h.app, "POST", "/products", Some(d)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
    assert!(body["error"]["message"].is_string());
    assert_eq!(body.as_object().map(|o| o.len()), Some(1));
}

#[tokio::test]
async fn categories_are_listed_with_the_rate_a_product_would_inherit() {
    // The add form used to hardcode category 1 and a null rate, so every
    // product came out at 19 %. It needs the shop's real categories.
    let h = harness();
    let (status, list) = call(&h.app, "GET", "/categories", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().map(Vec::len), Some(1));
    assert_eq!(list[0]["name"], "Général");
    assert_eq!(list[0]["default_rate_bps"], 1900);
    assert_eq!(list[0]["shop_id"], SHOP);
    assert!(list[0]["id"].is_number());
}

#[tokio::test]
async fn categories_are_scoped_to_the_servers_own_shop() {
    // Rule 3, the same way products are.
    let h = harness();
    let other = dzpos_api::router(dzpos_api::AppState::open(&h.path, 2).unwrap());
    let (status, list) = call(&other, "GET", "/categories", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn only_the_apps_own_origins_may_call_it() {
    // allow_origin(Any) let any page open in any browser on this machine
    // read and write the till's database over loopback.
    let h = harness();
    for origin in [
        "http://127.0.0.1:5173",
        "http://localhost:5173",
        "tauri://localhost",
        "http://tauri.localhost",
    ] {
        assert_eq!(
            preflight(&h.app, origin).await.as_deref(),
            Some(origin),
            "{origin} is one of the app's own and was refused"
        );
    }
    assert_eq!(
        preflight(&h.app, "https://evil.example").await,
        None,
        "a stranger's page was cleared to call the till"
    );
}

#[tokio::test]
async fn an_answer_to_a_stranger_carries_no_cors_header() {
    let h = harness();
    let req = Request::builder()
        .method("GET")
        .uri("/products")
        .header("origin", "https://evil.example")
        .body(Body::empty())
        .unwrap();
    let res = h.app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        allowed_origin(res),
        None,
        "the browser would have handed this answer to a stranger's page"
    );
}

#[tokio::test]
async fn one_more_origin_can_be_named_for_the_ssh_case() {
    // The UI served from the WSL box and opened on the laptop is a fourth
    // origin, and it is the operator's to name, never a default.
    use axum::http::HeaderValue;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    let app = dzpos_api::router_with_origin(
        state,
        Some(HeaderValue::from_static("http://100.111.55.62:5173")),
    );

    assert_eq!(
        preflight(&app, "http://100.111.55.62:5173")
            .await
            .as_deref(),
        Some("http://100.111.55.62:5173")
    );
    assert_eq!(preflight(&app, "https://evil.example").await, None);
    assert_eq!(
        preflight(&app, "http://127.0.0.1:5173").await.as_deref(),
        Some("http://127.0.0.1:5173"),
        "naming one more origin must not drop the built-in ones"
    );
}

#[tokio::test]
async fn the_server_only_ever_answers_for_its_own_shop() {
    // Rule 3: the shop is the server's, never the caller's to choose.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mine = dzpos_api::router(dzpos_api::AppState::open(&path, SHOP).unwrap());
    let (status, _) = call(&mine, "POST", "/products", Some(draft())).await;
    assert_eq!(status, StatusCode::CREATED);

    let other = dzpos_api::router(dzpos_api::AppState::open(&path, 2).unwrap());
    let (_, list) = call(&other, "GET", "/products", None).await;
    assert_eq!(list.as_array().map(Vec::len), Some(0));

    // A shop_id in the query string changes nothing.
    let (_, list) = call(&other, "GET", "/products?shop_id=1", None).await;
    assert_eq!(list.as_array().map(Vec::len), Some(0));
}
