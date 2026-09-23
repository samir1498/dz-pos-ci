// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// C4 of `the-first-clinic-module-patients-queue-appointments`: every route
// here exists only with the clinic built in, so `just check-clinic-only` is
// what runs this file, as it runs `patients_api.rs`.
#![cfg(feature = "clinic")]

//! The waiting queue over HTTP: a real temp SQLite file, the real router,
//! the real session and gate layers.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use dzpos_api::gates::gate_for;
use dzpos_core::services::permissions::{can, Permission, ROLES};

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

fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap().to_string()
}

fn names(list: &Value) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|q| format!("{} {}", text(q, "first_name"), text(q, "last_name")))
        .collect()
}

/// A conflict's status, code and field together.
fn assert_conflict(status: StatusCode, body: &Value, field: &str) {
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["error"]["code"], "conflict", "{body}");
    assert_eq!(body["error"]["field"], field, "{body}");
}

#[tokio::test]
async fn a_day_runs_arrival_to_seen_in_order() {
    let h = harness();
    let zidane = patient(&h.app, "Nadia", "Zidane").await;
    let benali = patient(&h.app, "Amina", "Benali").await;

    let (status, first) = call(
        &h.app,
        "POST",
        "/queue",
        Some(json!({ "patient_id": benali })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    assert_eq!(first["first_name"], "Amina");
    assert_eq!(first["called_at"], Value::Null);
    assert_eq!(first["day"].as_str().unwrap().len(), 10);
    let (_, second) = call(
        &h.app,
        "POST",
        "/queue",
        Some(json!({ "patient_id": zidane })),
    )
    .await;

    let (status, list) = call(&h.app, "GET", "/queue", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(names(&list), ["Amina Benali", "Nadia Zidane"]);

    let (status, called) = call(&h.app, "POST", "/queue/next", None).await;
    assert_eq!(status, StatusCode::OK, "{called}");
    assert_eq!(called["id"], first["id"]);
    assert!(called["called_at"].is_string());

    let (status, seen) = call(
        &h.app,
        "POST",
        &format!("/queue/{}/seen", text(&first, "id")),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{seen}");
    assert!(seen["seen_at"].is_string());

    let (status, gone) = call(
        &h.app,
        "POST",
        &format!("/queue/{}/left", text(&second, "id")),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert!(gone["left_at"].is_string());
    assert_eq!(gone["seen_at"], Value::Null);

    let (status, body) = call(&h.app, "POST", "/queue/next", None).await;
    assert_conflict(status, &body, "queue");
}

#[tokio::test]
async fn each_refusal_is_a_conflict_naming_its_field() {
    let h = harness();
    let id = patient(&h.app, "Amina", "Benali").await;
    let (_, entry) = call(&h.app, "POST", "/queue", Some(json!({ "patient_id": id }))).await;
    let entry_id = text(&entry, "id");

    let (status, body) = call(&h.app, "POST", "/queue", Some(json!({ "patient_id": id }))).await;
    assert_conflict(status, &body, "patient_id");
    let (status, body) = call(&h.app, "POST", &format!("/queue/{entry_id}/seen"), None).await;
    assert_conflict(status, &body, "called_at");
    let (status, _) = call(&h.app, "POST", &format!("/queue/{entry_id}/call"), None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = call(&h.app, "POST", &format!("/queue/{entry_id}/call"), None).await;
    assert_conflict(status, &body, "called_at");

    // An archived file is refused at the door.
    let archived = patient(&h.app, "Karim", "Haddad").await;
    let (status, _) = call(
        &h.app,
        "POST",
        &format!("/patients/{archived}/archive"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = call(
        &h.app,
        "POST",
        "/queue",
        Some(json!({ "patient_id": archived })),
    )
    .await;
    assert_conflict(status, &body, "patient_id");

    // The day and the moment are the shop clock's: a body that tries to
    // name them is refused before the service sees it.
    let (status, _) = call(
        &h.app,
        "POST",
        "/queue",
        Some(json!({ "patient_id": archived, "day": "2020-01-01" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (_, list) = call(&h.app, "GET", "/queue", None).await;
    assert_eq!(names(&list), ["Amina Benali"]);
}

#[tokio::test]
async fn another_shops_queue_and_patients_are_invisible() {
    let h = harness();
    let theirs_app = common::signed_in_router(&h.path, 2, &token());
    let their_patient = patient(&theirs_app, "Amina", "Benali").await;
    let (status, theirs) = call(
        &theirs_app,
        "POST",
        "/queue",
        Some(json!({ "patient_id": their_patient })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{theirs}");
    let id = text(&theirs, "id");

    for action in ["call", "seen", "left"] {
        let uri = format!("/queue/{id}/{action}");
        let (status, body) = call(&h.app, "POST", &uri, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}: {body}");
        assert_eq!(body["error"]["code"], "not_found", "{uri}");
    }
    let (status, body) = call(
        &h.app,
        "POST",
        "/queue",
        Some(json!({ "patient_id": their_patient })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let (_, list) = call(&h.app, "GET", "/queue", None).await;
    assert_eq!(names(&list), Vec::<String>::new());
    let (status, body) = call(&h.app, "POST", "/queue/next", None).await;
    assert_conflict(status, &body, "queue");

    // Still waiting, untouched, in its own shop.
    let (_, still) = call(&theirs_app, "GET", "/queue", None).await;
    assert_eq!(still, json!([theirs]));
}

/// The gate half, on the same terms as `patients_api.rs`'s: reads ask
/// `ViewPatients`, writes `EditPatients`, and no session is refused before
/// either. Every role holds both today, so the 403 arm runs the day one
/// loses one.
#[tokio::test]
async fn every_queue_route_is_gated_on_the_patient_files_two_permissions() {
    for (method, path, wanted) in [
        ("GET", "/queue", Permission::ViewPatients),
        ("POST", "/queue", Permission::EditPatients),
        ("POST", "/queue/next", Permission::EditPatients),
        ("POST", "/queue/{id}/call", Permission::EditPatients),
        ("POST", "/queue/{id}/seen", Permission::EditPatients),
        ("POST", "/queue/{id}/left", Permission::EditPatients),
    ] {
        assert_eq!(
            gate_for(method, path).and_then(|g| g.permission),
            Some(wanted),
            "{method} {path}"
        );
    }

    let h = harness();
    common::sign_in_as(&h.path, SHOP, "cashier", common::CASHIER_SESSION);
    common::sign_in_as(&h.path, SHOP, "manager", common::MANAGER_SESSION);
    let (status, _) = call_as(&h.app, None, "GET", "/queue", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call_as(&h.app, None, "POST", "/queue/next", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    for role in ROLES {
        let session = match role.as_str() {
            "owner" => common::OWNER_SESSION,
            "manager" => common::MANAGER_SESSION,
            _ => common::CASHIER_SESSION,
        };
        let (status, body) = call_as(&h.app, Some(session), "GET", "/queue", None).await;
        if can(role, Permission::ViewPatients) {
            assert_eq!(status, StatusCode::OK, "{role:?}: {body}");
        } else {
            assert_eq!(status, StatusCode::FORBIDDEN, "{role:?}: {body}");
        }
        // An empty line: allowed through the gate, refused by the service.
        let (status, body) = call_as(&h.app, Some(session), "POST", "/queue/next", None).await;
        if can(role, Permission::EditPatients) {
            assert_conflict(status, &body, "queue");
        } else {
            assert_eq!(status, StatusCode::FORBIDDEN, "{role:?}: {body}");
            assert_eq!(body["error"]["permission"], "edit_patients");
        }
    }
}
