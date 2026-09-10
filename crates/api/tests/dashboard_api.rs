// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The dashboard over HTTP. The API is the product's only entry point
//! (architecture.md, consequence of rule 2), so what these assert is the
//! contract the desktop reads: the shape of the answer, the day it defaults
//! to, and what a day nobody can parse gets back.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
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

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"))
        .body(Body::empty())
        .unwrap();
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

#[tokio::test]
async fn a_day_nobody_sold_on_answers_zeros_and_every_block_of_the_screen() {
    let h = harness();
    let (status, body) = get(&h.app, "/dashboard?day=2026-09-15").await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body["day"], "2026-09-15");
    assert_eq!(body["month"], "2026-09");
    for column in ["today", "this_month"] {
        assert_eq!(body[column]["sales_ttc_centimes"], 0);
        assert_eq!(body[column]["sales_count"], 0);
        assert_eq!(body[column]["sales_ht_centimes"], 0);
        assert_eq!(body[column]["cost_of_goods_centimes"], 0);
        assert_eq!(body[column]["margin_centimes"], 0);
        assert_eq!(body[column]["expenses_centimes"], 0);
    }
    // The cash position comes whole rather than as a number: the screen shows
    // both sides of it, and the range it covers.
    assert_eq!(body["cash_today"]["from"], "2026-09-15");
    assert_eq!(body["cash_today"]["to"], "2026-09-15");
    assert_eq!(body["cash_this_month"]["from"], "2026-09-01");
    assert_eq!(body["cash_this_month"]["to"], "2026-09-30");
    assert_eq!(body["low_stock"].as_array().unwrap().len(), 0);
    assert_eq!(body["top_by_quantity"].as_array().unwrap().len(), 0);
    assert_eq!(body["top_by_margin"].as_array().unwrap().len(), 0);
    assert_eq!(body["customer_debt"]["total_centimes"], 0);
    assert_eq!(body["customer_debt"]["parties"], 0);
    assert_eq!(body["supplier_debt"]["parties"], 0);
    assert_eq!(body["open_purchases"], 0);
}

#[tokio::test]
async fn no_day_means_the_shop_s_today_and_not_the_machine_s() {
    let h = harness();
    let (_, clock) = get(&h.app, "/clock").await;
    let today = clock["today"].as_str().unwrap().to_string();

    let (status, body) = get(&h.app, "/dashboard").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["day"], today,
        "the route read a clock the /clock route does not"
    );
    assert_eq!(body["month"], today[..7].to_string());
}

#[tokio::test]
async fn a_day_the_calendar_has_no_room_for_is_refused_by_the_edge() {
    let h = harness();
    for day in ["2026-13-01", "2026-9-15", "hier", "+026-09-15"] {
        let (status, body) = get(&h.app, &format!("/dashboard?day={day}")).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{day} was taken for a day"
        );
        assert_eq!(body["error"]["code"], "validation", "{day}");
        assert_eq!(body["error"]["field"], "day", "{day}");
    }
}
