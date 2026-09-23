// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// Every route here exists only with the clinic built in, as in
// `queue_api.rs`.
#![cfg(feature = "clinic")]

//! C6b of `the-first-clinic-module-patients-queue-appointments` over HTTP:
//! the book and the waiting room as one list. A real temp SQLite file, the
//! real router, the real session and gate layers. The book refuses the
//! past on the wall clock, so every booking here is for tomorrow or later.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Duration;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use dzpos_api::gates::gate_for;
use dzpos_core::services::clock;
use dzpos_core::services::permissions::{can, Permission, ROLES};

mod common;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";

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
    let token = dzpos_api::LaunchToken::from_secret(TOKEN).unwrap();
    Harness {
        _dir: dir,
        path,
        app: dzpos_api::router(state, &token),
    }
}

async fn call_as(
    app: &axum::Router,
    session: Option<&str>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"));
    if let Some(session) = session {
        req = req.header(common::SESSION_HEADER, session);
    }
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

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    call_as(app, Some(common::OWNER_SESSION), method, uri, body).await
}

/// Opens a file and returns its id.
async fn patient(app: &axum::Router, first: &str, last: &str) -> String {
    let body = json!({
        "first_name": first,
        "last_name": last,
        "sex": null,
        "date_of_birth": null,
        "phone": null,
        "notes": null,
    });
    let (status, made) = call(app, "POST", "/patients", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    made["id"].as_str().unwrap().to_string()
}

/// `days` after today on the shop's clock at `hh:mm`, as the wire writes it.
fn at(days: i64, hh: u32, mm: u32) -> String {
    let day = clock::now().date() + Duration::days(days);
    format!("{day} {hh:02}:{mm:02}:00")
}

/// Books `patient_id` at `starts_at` and returns the appointment's id.
async fn book(app: &axum::Router, patient_id: &str, starts_at: &str) -> String {
    let body = json!({ "patient_id": patient_id, "starts_at": starts_at, "note": null });
    let (status, made) = call(app, "POST", "/appointments", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    made["id"].as_str().unwrap().to_string()
}

fn assert_refused(status: StatusCode, body: &Value, wanted: StatusCode, field: &str) {
    assert_eq!(status, wanted, "{body}");
    assert_eq!(body["error"]["field"], field, "{body}");
}

/// The session of each role, signed in on `h`.
fn sessions(h: &Harness) -> Vec<(dzpos_core::services::permissions::Role, &'static str)> {
    common::sign_in_as(&h.path, SHOP, "cashier", common::CASHIER_SESSION);
    common::sign_in_as(&h.path, SHOP, "manager", common::MANAGER_SESSION);
    ROLES
        .iter()
        .map(|role| {
            let session = match role.as_str() {
                "owner" => common::OWNER_SESSION,
                "manager" => common::MANAGER_SESSION,
                _ => common::CASHIER_SESSION,
            };
            (*role, session)
        })
        .collect()
}

#[tokio::test]
async fn a_booked_patient_marked_arrived_is_in_todays_queue_once() {
    let h = harness();
    let benali = patient(&h.app, "Amina", "Benali").await;
    let starts = at(1, 9, 0);
    let id = book(&h.app, &benali, &starts).await;

    let uri = format!("/appointments/{id}/arrive");
    let (status, first) = call(&h.app, "POST", &uri, None).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["appointment_id"], id.as_str());
    assert_eq!(first["appointment_starts_at"], starts.as_str());
    let (status, again) = call(&h.app, "POST", &uri, None).await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(again, first);

    let (status, queue) = call(&h.app, "GET", "/queue", None).await;
    assert_eq!(status, StatusCode::OK);
    let queue = queue.as_array().unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0]["appointment_starts_at"], starts.as_str());

    let (status, body) = call(&h.app, "POST", "/appointments/nobody/arrive", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let (status, _) = call(&h.app, "POST", &format!("/appointments/{id}/cancel"), None).await;
    assert_eq!(status, StatusCode::OK);
    let later = book(&h.app, &benali, &at(2, 9, 0)).await;
    call(
        &h.app,
        "POST",
        &format!("/appointments/{later}/cancel"),
        None,
    )
    .await;
    let (status, body) = call(
        &h.app,
        "POST",
        &format!("/appointments/{later}/arrive"),
        None,
    )
    .await;
    assert_refused(status, &body, StatusCode::CONFLICT, "cancelled_at");
}

#[tokio::test]
async fn the_desk_puts_todays_queue_in_its_own_order_and_the_next_call_follows_it() {
    let h = harness();
    let mut ids = Vec::new();
    for first in ["Amina", "Karim", "Sami"] {
        let p = patient(&h.app, first, "Test").await;
        let (status, added) =
            call(&h.app, "POST", "/queue", Some(json!({ "patient_id": p }))).await;
        assert_eq!(status, StatusCode::CREATED, "{added}");
        ids.push(added["id"].as_str().unwrap().to_string());
    }
    let (status, list) = call(
        &h.app,
        "PUT",
        "/queue/order",
        Some(json!({ "ids": [ids[2], ids[0]] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{list}");
    let order: Vec<(&str, i64)> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|q| {
            (
                q["first_name"].as_str().unwrap(),
                q["position"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(order, [("Sami", 1), ("Amina", 2), ("Karim", 3)]);
    let (status, next) = call(&h.app, "POST", "/queue/next", None).await;
    assert_eq!(status, StatusCode::OK, "{next}");
    assert_eq!(next["id"], ids[2].as_str());
    // Karim alone to the top of Sami (called), Amina, Karim: the two left
    // out keep the desk's order, Sami before Amina, not arrival's.
    let (status, list) = call(
        &h.app,
        "PUT",
        "/queue/order",
        Some(json!({ "ids": [ids[1]] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{list}");
    let names: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["first_name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Karim", "Sami", "Amina"]);

    for bad in [
        json!({ "ids": ["nobody"] }),
        json!({ "ids": [ids[0], ids[0]] }),
    ] {
        let (status, body) = call(&h.app, "PUT", "/queue/order", Some(bad)).await;
        assert_refused(status, &body, StatusCode::UNPROCESSABLE_ENTITY, "ids");
    }
    let (status, _) = call(&h.app, "PUT", "/queue/order", Some(json!({ "order": [] }))).await;
    assert!(status.is_client_error(), "{status}");
}

#[tokio::test]
async fn a_confirmation_call_is_recorded_on_the_booking_and_cleared() {
    let h = harness();
    let benali = patient(&h.app, "Amina", "Benali").await;
    let id = book(&h.app, &benali, &at(1, 9, 0)).await;
    let uri = format!("/appointments/{id}/call-outcome");
    let (status, called) = call(
        &h.app,
        "POST",
        &uri,
        Some(json!({ "outcome": "no_answer" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{called}");
    assert_eq!(called["call_outcome"], "no_answer");
    assert!(called["call_at"].is_string(), "{called}");
    assert_eq!(called["phone"], Value::Null);
    let (status, called) = call(
        &h.app,
        "POST",
        &uri,
        Some(json!({ "outcome": "confirmed" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{called}");
    assert_eq!(called["call_outcome"], "confirmed");
    let (_, day) = call(
        &h.app,
        "GET",
        &format!("/appointments?day={}", &at(1, 9, 0)[..10]),
        None,
    )
    .await;
    assert_eq!(day["appointments"][0]["call_outcome"], "confirmed");

    for bad in [
        json!({ "outcome": "maybe" }),
        json!({}),
        json!({ "outcome": "confirmed", "x": 1 }),
    ] {
        let (status, body) = call(&h.app, "POST", &uri, Some(bad)).await;
        assert!(
            status.is_client_error() && status != StatusCode::NOT_FOUND,
            "{status}: {body}"
        );
    }
    let (status, cleared) = call(&h.app, "POST", &format!("{uri}/clear"), None).await;
    assert_eq!(status, StatusCode::OK, "{cleared}");
    assert_eq!(cleared["call_outcome"], Value::Null);
    assert_eq!(cleared["call_at"], Value::Null);
    let (status, again) = call(&h.app, "POST", &format!("{uri}/clear"), None).await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(again, cleared);
}

#[tokio::test]
async fn every_c6b_route_is_gated_on_the_patient_files_two_permissions() {
    for (method, path, wanted) in [
        (
            "POST",
            "/appointments/{id}/arrive",
            Permission::EditPatients,
        ),
        ("PUT", "/queue/order", Permission::EditPatients),
        (
            "POST",
            "/appointments/{id}/call-outcome",
            Permission::EditPatients,
        ),
        (
            "POST",
            "/appointments/{id}/call-outcome/clear",
            Permission::EditPatients,
        ),
    ] {
        assert_eq!(
            gate_for(method, path).and_then(|g| g.permission),
            Some(wanted),
            "{method} {path}"
        );
    }
    let h = harness();
    let (status, _) = call_as(&h.app, None, "POST", "/appointments/x/arrive", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    for (role, session) in sessions(&h) {
        // An id nobody made: through the gate it is not found.
        for (write, body) in [
            ("/appointments/nobody/arrive", None),
            (
                "/appointments/nobody/call-outcome",
                Some(json!({ "outcome": "confirmed" })),
            ),
            ("/appointments/nobody/call-outcome/clear", None),
        ] {
            let (status, answer) = call_as(&h.app, Some(session), "POST", write, body).await;
            if can(role, Permission::EditPatients) {
                assert_eq!(status, StatusCode::NOT_FOUND, "{role:?} {write}: {answer}");
            } else {
                assert_eq!(status, StatusCode::FORBIDDEN, "{role:?} {write}: {answer}");
                assert_eq!(answer["error"]["permission"], "edit_patients");
            }
        }
        // An empty order of an empty day: through the gate it is the day.
        let (status, body) = call_as(
            &h.app,
            Some(session),
            "PUT",
            "/queue/order",
            Some(json!({ "ids": [] })),
        )
        .await;
        if can(role, Permission::EditPatients) {
            assert_eq!(status, StatusCode::OK, "{role:?}: {body}");
        } else {
            assert_eq!(status, StatusCode::FORBIDDEN, "{role:?}: {body}");
            assert_eq!(body["error"]["permission"], "edit_patients");
        }
    }
}
