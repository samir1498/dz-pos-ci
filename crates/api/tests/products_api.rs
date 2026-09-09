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

/// The launch token every request in this file shows. The desktop makes a
/// random one per launch; a fixed one here keeps the tests readable.
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

    let other = dzpos_api::router(dzpos_api::AppState::open(&h.path, 2).unwrap(), &token());
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
async fn a_bad_path_and_a_wrong_method_answer_in_the_same_envelope() {
    // Both used to leave as axum's bare text or an empty body, which the
    // client can only read as "unreachable"; the UI then blamed the network
    // for a wrong URL.
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/products/abc", None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "bad_request");
    assert!(
        !body["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("i32"),
        "a Rust type name reached the wire: {body}"
    );

    let (status, body) = call(&h.app, "DELETE", "/products", None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(body["error"]["code"], "method_not_allowed");

    let (status, body) = call(&h.app, "GET", "/nowhere", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
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
    let other = dzpos_api::router(dzpos_api::AppState::open(&h.path, 2).unwrap(), &token());
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
        &token(),
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
    let mine = dzpos_api::router(dzpos_api::AppState::open(&path, SHOP).unwrap(), &token());
    let (status, _) = call(&mine, "POST", "/products", Some(draft())).await;
    assert_eq!(status, StatusCode::CREATED);

    let other = dzpos_api::router(dzpos_api::AppState::open(&path, 2).unwrap(), &token());
    let (_, list) = call(&other, "GET", "/products", None).await;
    assert_eq!(list.as_array().map(Vec::len), Some(0));

    // A shop_id in the query string changes nothing.
    let (_, list) = call(&other, "GET", "/products?shop_id=1", None).await;
    assert_eq!(list.as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn the_smallest_i64_is_refused_like_any_other_out_of_range_amount() {
    // `i64::MIN` has no positive twin, so a guard written as `abs() > MAX`
    // overflows on it: a panic in debug, a wrapped value that slips past the
    // guard in release. It is out of the JSON-safe range like 2^53 is, and
    // gets the same 422.
    let h = harness();
    for field in [
        "cost_centimes",
        "selling_centimes",
        "wholesale_centimes",
        "qty_on_hand_milli",
        "low_stock_at_milli",
    ] {
        let mut body = draft();
        body[field] = json!(i64::MIN);
        let (status, answer) = call(&h.app, "POST", "/products", Some(body)).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{field}: {answer}"
        );
        assert_eq!(answer["error"]["code"], "validation", "{field}: {answer}");
    }
}

#[tokio::test]
async fn a_spent_barcode_series_is_a_conflict_on_the_wire() {
    // The core says `exhausted` (products_service.rs); the API turns that
    // into 409 with the same code, so the screen can name the series rather
    // than blame the form.
    use diesel::prelude::*;

    let h = harness();
    let mut conn = diesel::SqliteConnection::establish(h.path.to_str().unwrap()).unwrap();
    diesel::sql_query(format!(
        "INSERT INTO counters (shop_id, name, next_value) VALUES ({SHOP}, 'in_store_barcode', {}) \
         ON CONFLICT (shop_id, name) DO UPDATE SET next_value = excluded.next_value",
        i64::MAX
    ))
    .execute(&mut conn)
    .unwrap();
    let (status, answer) = call(&h.app, "POST", "/products", Some(draft())).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert_eq!(answer["error"]["code"], "exhausted", "{answer}");
}

/// A request built by hand, with whatever `authorization` value the test
/// wants, or none. `call` above always shows the right token.
async fn call_with_auth(
    app: &axum::Router,
    method: &str,
    uri: &str,
    authorization: Option<&str>,
) -> axum::response::Response {
    let req = Request::builder().method(method).uri(uri);
    let req = match authorization {
        Some(value) => req.header("authorization", value),
        None => req,
    };
    app.clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn envelope(res: axum::response::Response) -> (StatusCode, Value) {
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn a_request_without_the_launch_token_is_401_in_the_envelope() {
    // Loopback is not a boundary: any process on the machine can open
    // 127.0.0.1. Without the token the desktop handed its own webview, a
    // caller learns nothing, not even that the route exists.
    let h = harness();
    for shown in [
        None,
        Some("Bearer wrong"),
        Some("Basic dGVzdA=="),
        Some("test-launch-token"),
    ] {
        let res = call_with_auth(&h.app, "GET", "/products", shown).await;
        assert_eq!(
            res.headers()
                .get("www-authenticate")
                .and_then(|v| v.to_str().ok()),
            Some("Bearer"),
            "{shown:?}"
        );
        let (status, body) = envelope(res).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{shown:?}");
        assert_eq!(body["error"]["code"], "unauthorized", "{shown:?}");
        assert!(body["error"]["message"].is_string(), "{shown:?}");
    }
    let (status, _) = call(&h.app, "GET", "/products", None).await;
    assert_eq!(status, StatusCode::OK, "the right token still gets through");
}

#[tokio::test]
async fn a_token_that_differs_only_at_the_end_is_refused() {
    let h = harness();
    for shown in [
        "Bearer test-launch-tokeN",
        "Bearer test-launch-toke",
        "Bearer test-launch-token1",
    ] {
        let res = call_with_auth(&h.app, "GET", "/products", Some(shown)).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{shown}");
    }
}

#[tokio::test]
async fn the_scheme_is_read_the_way_rfc_7235_spells_it() {
    // The scheme is case-insensitive and may be followed by more than one
    // space; the token itself is exact.
    let h = harness();
    for shown in [
        "bearer test-launch-token",
        "BEARER test-launch-token",
        "Bearer  test-launch-token",
    ] {
        let res = call_with_auth(&h.app, "GET", "/products", Some(shown)).await;
        assert_eq!(res.status(), StatusCode::OK, "{shown}");
    }
    for shown in [
        "Bearertest-launch-token",
        "Bearer test-launch-token extra",
        "Token test-launch-token",
    ] {
        let res = call_with_auth(&h.app, "GET", "/products", Some(shown)).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{shown}");
    }
}

#[tokio::test]
async fn a_wrong_method_on_health_answers_in_the_envelope_too() {
    // /health lives outside the token guard, and it still owes the same
    // JSON 405 every other route gives.
    let h = harness();
    let (status, body) = envelope(call_with_auth(&h.app, "PUT", "/health", None).await).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(body["error"]["code"], "method_not_allowed");
}

#[tokio::test]
async fn an_unknown_route_needs_the_token_too() {
    // 404 versus 401 would tell a stranger which routes exist.
    let h = harness();
    let (status, body) = envelope(call_with_auth(&h.app, "GET", "/nowhere", None).await).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn health_needs_no_token() {
    // The e2e harness and the desktop wait on it before the page has the
    // token; it says the process is up and which shop, nothing more.
    let h = harness();
    let (status, body) = envelope(call_with_auth(&h.app, "GET", "/health", None).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn the_preflight_clears_the_authorization_header() {
    // A browser sends OPTIONS without any authorization header and only
    // then the real request with it; the preflight must name the header
    // as allowed or the browser never sends the token at all.
    let h = harness();
    let req = Request::builder()
        .method("OPTIONS")
        .uri("/products")
        .header("origin", "http://127.0.0.1:5173")
        .header("access-control-request-method", "POST")
        .header(
            "access-control-request-headers",
            "authorization, content-type",
        )
        .body(Body::empty())
        .unwrap();
    let res = h.app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let allow_headers = res
        .headers()
        .get("access-control-allow-headers")
        .and_then(|v| v.to_str().ok())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    assert!(allow_headers.contains("authorization"), "{allow_headers}");
    assert_eq!(
        allowed_origin(res).as_deref(),
        Some("http://127.0.0.1:5173")
    );
}

#[tokio::test]
async fn put_products_id_updates_the_row_and_answers_it() {
    let h = harness();
    let (_, made) = call(&h.app, "POST", "/products", Some(draft())).await;
    let id = made["id"].as_i64().unwrap();
    let mut edited = draft();
    edited["name"] = json!("Huile Elio 5L (promo)");
    edited["selling_centimes"] = json!(880);
    edited["wholesale_centimes"] = json!(850);
    edited["low_stock_at_milli"] = json!(5_000);
    edited["rate_bps"] = json!(900);
    edited["active"] = json!(false);
    edited["barcode"] = Value::Null;
    let (status, body) = call(&h.app, "PUT", &format!("/products/{id}"), Some(edited)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["id"], id);
    assert_eq!(body["name"], "Huile Elio 5L (promo)");
    assert_eq!(body["selling_centimes"], 880);
    assert_eq!(body["wholesale_centimes"], 850);
    assert_eq!(body["low_stock_at_milli"], 5_000);
    assert_eq!(body["rate_bps"], 900);
    assert_eq!(body["active"], false);
    assert_eq!(
        body["barcode"], made["barcode"],
        "a null barcode keeps the number"
    );

    let (_, one) = call(&h.app, "GET", &format!("/products/{id}"), None).await;
    assert_eq!(one, body, "the read answers what the update answered");
}

#[tokio::test]
async fn put_on_a_missing_product_is_404_and_a_bad_id_is_422() {
    let h = harness();
    let (status, body) = call(&h.app, "PUT", "/products/999", Some(draft())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
    let (status, body) = call(&h.app, "PUT", "/products/abc", Some(draft())).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "bad_request");
}

#[tokio::test]
async fn put_with_another_products_barcode_is_409_and_a_bad_body_is_422() {
    let h = harness();
    let (_, a) = call(&h.app, "POST", "/products", Some(draft())).await;
    let mut other = draft();
    other["name"] = json!("B");
    let (_, b) = call(&h.app, "POST", "/products", Some(other)).await;
    let mut taken = draft();
    taken["barcode"] = b["barcode"].clone();
    let uri = format!("/products/{}", a["id"]);
    let (status, body) = call(&h.app, "PUT", &uri, Some(taken)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "duplicate_barcode");

    let mut bad = draft();
    bad["selling_centimes"] = json!(-1);
    let (status, body) = call(&h.app, "PUT", &uri, Some(bad)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "validation");
    let (_, still) = call(&h.app, "GET", &uri, None).await;
    assert_eq!(still["name"], a["name"], "a refused update changes nothing");
}

#[tokio::test]
async fn the_browser_is_allowed_to_send_a_put() {
    // CORS names the methods too; without PUT in the list the preflight
    // refuses the edit the screen sends.
    let h = harness();
    let req = Request::builder()
        .method("OPTIONS")
        .uri("/products/1")
        .header("origin", "http://127.0.0.1:5173")
        .header("access-control-request-method", "PUT")
        .header(
            "access-control-request-headers",
            "authorization, content-type",
        )
        .body(Body::empty())
        .unwrap();
    let res = h.app.clone().oneshot(req).await.unwrap();
    let methods = res
        .headers()
        .get("access-control-allow-methods")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(methods.contains("PUT"), "{methods}");
    assert_eq!(
        allowed_origin(res).as_deref(),
        Some("http://127.0.0.1:5173")
    );
}
