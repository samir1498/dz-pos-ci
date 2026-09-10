// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Expenses and the cash position over HTTP. The API is the product's only
//! entry point (architecture.md, consequence of rule 2), so what these assert
//! is the contract the desktop reads: the seeded categories, the month's list
//! with its total, what a refused write answers, and the one `/cash` route
//! that takes a day or a month and never both.

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

async fn a_category(app: &axum::Router, key: &str) -> i64 {
    let (status, body) = call(app, "GET", "/expense-categories", None).await;
    assert_eq!(status, StatusCode::OK);
    body.as_array()
        .unwrap()
        .iter()
        .find(|c| c["key"] == key)
        .unwrap()["id"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn the_seven_seeded_categories_come_back_in_their_order_with_their_keys() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/expense-categories", None).await;
    assert_eq!(status, StatusCode::OK);
    let keys: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["key"].as_str().unwrap())
        .collect();
    assert_eq!(
        keys,
        vec![
            "rent",
            "electricity",
            "water",
            "salaries",
            "transport",
            "maintenance",
            "other"
        ]
    );
    assert_eq!(body[0]["active"], json!(true));
}

#[tokio::test]
async fn a_month_answers_its_own_rows_and_the_total_the_core_summed() {
    let h = harness();
    let rent = a_category(&h.app, "rent").await;
    let water = a_category(&h.app, "water").await;
    for (category, centimes, day) in [
        (rent, 3_000_000, "2026-09-01"),
        (water, 150_000, "2026-09-30"),
        (rent, 999_999, "2026-08-31"),
    ] {
        let (status, _) = call(
            &h.app,
            "POST",
            "/expenses",
            Some(json!({
                "category_id": category,
                "amount_centimes": centimes,
                "expense_date": day,
                "note": null,
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }
    let (status, body) = call(&h.app, "GET", "/expenses?month=2026-09", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["month"], json!("2026-09"));
    assert_eq!(body["total_centimes"], json!(3_150_000));
    let rows = body["expenses"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    // Newest first.
    assert_eq!(rows[0]["expense_date"], json!("2026-09-30"));
    assert_eq!(rows[0]["amount_centimes"], json!(150_000));
    assert_eq!(rows[1]["category_id"], json!(rent));

    let (status, empty) = call(&h.app, "GET", "/expenses?month=2026-07", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(empty["total_centimes"], json!(0));
    assert!(empty["expenses"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn a_bad_month_a_bad_day_and_a_bad_amount_are_refused_by_the_field_they_are_wrong_on() {
    let h = harness();
    let rent = a_category(&h.app, "rent").await;
    for month in ["2026-9", "2026-13", "septembre", "2026-09-01"] {
        let (status, body) = call(&h.app, "GET", &format!("/expenses?month={month}"), None).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "month {month}");
        assert_eq!(body["error"]["field"], json!("month"), "month {month}");
    }
    let (status, body) = call(
        &h.app,
        "POST",
        "/expenses",
        Some(json!({
            "category_id": rent,
            "amount_centimes": 1_000,
            "expense_date": "2026-9-1",
            "note": null,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["field"], json!("expense_date"));

    let (status, body) = call(
        &h.app,
        "POST",
        "/expenses",
        Some(json!({
            "category_id": rent,
            "amount_centimes": 0,
            "expense_date": "2026-09-01",
            "note": null,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["field"], json!("amount"));

    let (status, body) = call(
        &h.app,
        "POST",
        "/expenses",
        Some(json!({
            "category_id": 9_999,
            "amount_centimes": 1_000,
            "expense_date": "2026-09-01",
            "note": null,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["field"], json!("category_id"));
}

#[tokio::test]
async fn the_cash_position_takes_a_day_or_a_month_and_never_both_or_neither() {
    let h = harness();
    let rent = a_category(&h.app, "rent").await;
    let (status, _) = call(
        &h.app,
        "POST",
        "/expenses",
        Some(json!({
            "category_id": rent,
            "amount_centimes": 25_000,
            "expense_date": "2026-09-10",
            "note": "electricite",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, day) = call(&h.app, "GET", "/cash?day=2026-09-10", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(day["from"], json!("2026-09-10"));
    assert_eq!(day["to"], json!("2026-09-10"));
    assert_eq!(day["cash_in"]["sales_centimes"], json!(0));
    // Nothing was sold, so the takings carry no droit de timbre either. The
    // figure travels on every answer rather than only when there is one.
    assert_eq!(day["cash_in"]["stamp_centimes"], json!(0));
    assert_eq!(day["cash_in"]["total_centimes"], json!(0));
    assert_eq!(day["cash_out"]["expenses_centimes"], json!(25_000));
    assert_eq!(day["cash_out"]["refunds_centimes"], json!(0));
    assert_eq!(day["cash_out"]["total_centimes"], json!(25_000));
    assert_eq!(day["cash_centimes"], json!(-25_000));
    assert_eq!(day["card_in"]["total_centimes"], json!(0));

    let (status, month) = call(&h.app, "GET", "/cash?month=2026-09", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(month["from"], json!("2026-09-01"));
    assert_eq!(month["to"], json!("2026-09-30"));
    assert_eq!(month["cash_out"]["expenses_centimes"], json!(25_000));

    // A day the shop spent nothing on answers zeros, not nothing.
    let (status, quiet) = call(&h.app, "GET", "/cash?day=2026-09-11", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(quiet["cash_centimes"], json!(0));

    // Neither and both are the same refusal: the caller has not said which
    // range it wants, and answering one of them would be the route choosing.
    for uri in ["/cash", "/cash?day=2026-09-10&month=2026-09"] {
        let (status, body) = call(&h.app, "GET", uri, None).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{uri}");
        assert_eq!(body["error"]["field"], json!("period"), "{uri}");
    }
    let (status, body) = call(&h.app, "GET", "/cash?day=2026-9-10", None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["field"], json!("day"));
}

#[tokio::test]
async fn an_expense_is_never_edited_or_deleted_over_the_wire() {
    let h = harness();
    let rent = a_category(&h.app, "rent").await;
    let (_, made) = call(
        &h.app,
        "POST",
        "/expenses",
        Some(json!({
            "category_id": rent,
            "amount_centimes": 1_000,
            "expense_date": "2026-09-10",
            "note": null,
        })),
    )
    .await;
    let id = made["id"].as_i64().unwrap();
    for method in ["PUT", "DELETE", "PATCH"] {
        let (status, _) = call(&h.app, method, &format!("/expenses/{id}"), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method}");
    }
}
