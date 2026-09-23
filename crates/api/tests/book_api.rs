// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// C5 of `the-first-clinic-module-patients-queue-appointments`: every route
// here exists only with the clinic built in, so `just check-clinic-only` is
// what runs this file, as it runs `queue_api.rs`.
#![cfg(feature = "clinic")]

//! The appointment book over HTTP: a real temp SQLite file, the real
//! router, the real session and gate layers. The book refuses the past on
//! the wall clock, so every case books on days counted from tomorrow.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{Datelike, Duration, NaiveDate};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use dzpos_api::gates::gate_for;
use dzpos_core::services::clock;
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

fn day_ahead(days: i64) -> NaiveDate {
    clock::now().date() + Duration::days(days)
}

async fn book(app: &axum::Router, patient_id: &str, starts_at: &str) -> (StatusCode, Value) {
    call(
        app,
        "POST",
        "/appointments",
        Some(json!({ "patient_id": patient_id, "starts_at": starts_at, "note": null })),
    )
    .await
}

fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap().to_string()
}

/// Each appointment of a list answer as `start name`.
fn read(list: &Value) -> Vec<String> {
    list["appointments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| format!("{} {}", text(a, "starts_at"), text(a, "last_name")))
        .collect()
}

fn assert_refused(status: StatusCode, body: &Value, wanted: StatusCode, field: &str) {
    assert_eq!(status, wanted, "{body}");
    assert_eq!(body["error"]["field"], field, "{body}");
}

#[tokio::test]
async fn a_day_is_booked_moved_cancelled_and_read_back_in_time_order() {
    let h = harness();
    let benali = patient(&h.app, "Amina", "Benali").await;
    let haddad = patient(&h.app, "Karim", "Haddad").await;
    let day = day_ahead(1);

    // The later slot first, so the ids sort the other way from the starts.
    let (status, late) = book(&h.app, &benali, &format!("{day} 11:00:00")).await;
    assert_eq!(status, StatusCode::CREATED, "{late}");
    assert_eq!(late["slot_minutes"], 15);
    assert_eq!(late["first_name"], "Amina");
    assert_eq!(late["cancelled_at"], Value::Null);
    let (status, early) = book(&h.app, &haddad, &format!("{day} 09:00:00")).await;
    assert_eq!(status, StatusCode::CREATED, "{early}");

    let (status, list) = call(&h.app, "GET", &format!("/appointments?day={day}"), None).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(list["from"], day.to_string());
    assert_eq!(list["to"], day.to_string());
    assert_eq!(
        read(&list),
        [
            format!("{day} 09:00:00 Haddad"),
            format!("{day} 11:00:00 Benali")
        ]
    );

    let late_id = text(&late, "id");
    let (status, moved) = call(
        &h.app,
        "POST",
        &format!("/appointments/{late_id}/move"),
        Some(json!({ "starts_at": format!("{day} 08:30:00") })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{moved}");
    assert_eq!(moved["id"], late["id"]);
    assert_eq!(moved["starts_at"], format!("{day} 08:30:00"));

    let early_id = text(&early, "id");
    let (status, cancelled) = call(
        &h.app,
        "POST",
        &format!("/appointments/{early_id}/cancel"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert!(cancelled["cancelled_at"].is_string());
    let (status, again) = call(
        &h.app,
        "POST",
        &format!("/appointments/{early_id}/cancel"),
        None,
    )
    .await;
    assert_refused(status, &again, StatusCode::CONFLICT, "cancelled_at");

    let (_, list) = call(&h.app, "GET", &format!("/appointments?day={day}"), None).await;
    assert_eq!(read(&list), [format!("{day} 08:30:00 Benali")]);
    let (status, one) = call(&h.app, "GET", &format!("/appointments/{early_id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(one, cancelled);
}

#[tokio::test]
async fn a_week_is_sunday_to_saturday_whatever_day_is_asked_for() {
    let h = harness();
    let benali = patient(&h.app, "Amina", "Benali").await;
    // The Sunday of the week holding the day nine days ahead: at least
    // three days out, so the Saturday before it is in the future too.
    let base = day_ahead(9);
    let sunday = base - Duration::days(i64::from(base.weekday().num_days_from_sunday()));
    let saturday = sunday + Duration::days(6);
    assert_eq!(sunday.format("%A").to_string(), "Sunday");

    for (day, hh) in [
        (saturday, "16"),
        (sunday, "09"),
        (sunday - Duration::days(1), "09"),
        (sunday + Duration::days(7), "09"),
    ] {
        let (status, body) = book(&h.app, &benali, &format!("{day} {hh}:00:00")).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
    }
    let wednesday = sunday + Duration::days(3);
    let (status, week) = call(
        &h.app,
        "GET",
        &format!("/appointments?week={wednesday}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{week}");
    assert_eq!(week["from"], sunday.to_string());
    assert_eq!(week["to"], saturday.to_string());
    assert_eq!(
        read(&week),
        [
            format!("{sunday} 09:00:00 Benali"),
            format!("{saturday} 16:00:00 Benali")
        ]
    );
}

#[tokio::test]
async fn each_refusal_names_its_field_and_its_kind() {
    let h = harness();
    let benali = patient(&h.app, "Amina", "Benali").await;
    let haddad = patient(&h.app, "Karim", "Haddad").await;
    let day = day_ahead(1);
    let (status, _) = book(&h.app, &benali, &format!("{day} 09:00:00")).await;
    assert_eq!(status, StatusCode::CREATED);

    // Taken: a 409.
    let (status, body) = book(&h.app, &haddad, &format!("{day} 09:00:00")).await;
    assert_refused(status, &body, StatusCode::CONFLICT, "starts_at");
    // Off the grid, in the past, or written another way: a 422.
    let (status, body) = book(&h.app, &haddad, &format!("{day} 09:10:00")).await;
    assert_refused(status, &body, StatusCode::UNPROCESSABLE_ENTITY, "starts_at");
    let yesterday = day_ahead(-1);
    let (status, body) = book(&h.app, &haddad, &format!("{yesterday} 09:00:00")).await;
    assert_refused(status, &body, StatusCode::UNPROCESSABLE_ENTITY, "starts_at");
    for written in [format!("{day}T09:15:00"), format!("{day} 09:15")] {
        let (status, body) = book(&h.app, &haddad, &written).await;
        assert_refused(status, &body, StatusCode::UNPROCESSABLE_ENTITY, "starts_at");
    }
    // The length is the setting's: a body that names one is refused.
    let (status, _) = call(
        &h.app,
        "POST",
        "/appointments",
        Some(json!({
            "patient_id": haddad,
            "starts_at": format!("{day} 09:15:00"),
            "note": null,
            "slot_minutes": 60,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    // An archived file: a 409 on the patient.
    let (status, _) = call(&h.app, "POST", &format!("/patients/{haddad}/archive"), None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = book(&h.app, &haddad, &format!("{day} 09:15:00")).await;
    assert_refused(status, &body, StatusCode::CONFLICT, "patient_id");
    // A list asks for exactly one of a day or a week.
    for uri in [
        "/appointments".to_string(),
        format!("/appointments?day={day}&week={day}"),
    ] {
        let (status, body) = call(&h.app, "GET", &uri, None).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{uri}: {body}");
        assert_eq!(body["error"]["code"], "bad_request", "{uri}: {body}");
    }
    let (status, body) = call(&h.app, "GET", "/appointments?day=2026-9-1", None).await;
    assert_refused(status, &body, StatusCode::UNPROCESSABLE_ENTITY, "day");
}

#[tokio::test]
async fn the_slot_length_is_read_open_and_set_as_a_setting() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/settings/slot-minutes", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "slot_minutes": 15 }));
    let (status, body) = call(
        &h.app,
        "PUT",
        "/settings/slot-minutes",
        Some(json!({ "slot_minutes": 7 })),
    )
    .await;
    assert_refused(
        status,
        &body,
        StatusCode::UNPROCESSABLE_ENTITY,
        "slot_minutes",
    );
    let (status, body) = call(
        &h.app,
        "PUT",
        "/settings/slot-minutes",
        Some(json!({ "slot_minutes": 20 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!({ "slot_minutes": 20 }));

    // A booking now takes twenty minutes, on the twenty-minute grid.
    let benali = patient(&h.app, "Amina", "Benali").await;
    let day = day_ahead(1);
    let (status, body) = book(&h.app, &benali, &format!("{day} 09:15:00")).await;
    assert_refused(status, &body, StatusCode::UNPROCESSABLE_ENTITY, "starts_at");
    let (status, made) = book(&h.app, &benali, &format!("{day} 09:20:00")).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["slot_minutes"], 20);
}

#[tokio::test]
async fn another_shops_book_and_patients_are_invisible() {
    let h = harness();
    let theirs_app = common::signed_in_router(&h.path, 2, &token());
    let their_patient = patient(&theirs_app, "Amina", "Benali").await;
    let day = day_ahead(1);
    let (status, theirs) = book(&theirs_app, &their_patient, &format!("{day} 09:00:00")).await;
    assert_eq!(status, StatusCode::CREATED, "{theirs}");
    let id = text(&theirs, "id");

    let (status, body) = call(&h.app, "GET", &format!("/appointments/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(body["error"]["code"], "not_found");
    let (status, body) = call(&h.app, "POST", &format!("/appointments/{id}/cancel"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let (status, body) = call(
        &h.app,
        "POST",
        &format!("/appointments/{id}/move"),
        Some(json!({ "starts_at": format!("{day} 10:00:00") })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let (status, body) = book(&h.app, &their_patient, &format!("{day} 10:00:00")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");

    let (_, mine) = call(&h.app, "GET", &format!("/appointments?day={day}"), None).await;
    assert_eq!(read(&mine), Vec::<String>::new());
    // Their slot length is theirs too.
    let (status, _) = call(
        &h.app,
        "PUT",
        "/settings/slot-minutes",
        Some(json!({ "slot_minutes": 30 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, their_slot) = call(&theirs_app, "GET", "/settings/slot-minutes", None).await;
    assert_eq!(their_slot, json!({ "slot_minutes": 15 }));

    // Still booked, untouched, in its own shop.
    let (_, still) = call(&theirs_app, "GET", &format!("/appointments/{id}"), None).await;
    assert_eq!(still, theirs);
}

/// The gate half, on the same terms as `queue_api.rs`'s: reads ask
/// `ViewPatients`, the book's writes `EditPatients`, the slot length's write
/// `EditSettings` like every other settings write, its read has no row, and
/// no session is refused before any of them. A cashier holds the first two
/// and not the third.
#[tokio::test]
async fn the_book_asks_the_patient_files_permissions_and_the_slot_length_edit_settings() {
    for (method, path, wanted) in [
        ("GET", "/appointments", Some(Permission::ViewPatients)),
        ("POST", "/appointments", Some(Permission::EditPatients)),
        ("GET", "/appointments/{id}", Some(Permission::ViewPatients)),
        (
            "POST",
            "/appointments/{id}/cancel",
            Some(Permission::EditPatients),
        ),
        (
            "POST",
            "/appointments/{id}/move",
            Some(Permission::EditPatients),
        ),
        (
            "PUT",
            "/settings/slot-minutes",
            Some(Permission::EditSettings),
        ),
        ("GET", "/settings/slot-minutes", None),
    ] {
        assert_eq!(
            gate_for(method, path).and_then(|g| g.permission),
            wanted,
            "{method} {path}"
        );
    }

    let h = harness();
    common::sign_in_as(&h.path, SHOP, "cashier", common::CASHIER_SESSION);
    common::sign_in_as(&h.path, SHOP, "manager", common::MANAGER_SESSION);
    let day = day_ahead(1);
    let list = format!("/appointments?day={day}");
    for (method, uri) in [
        ("GET", list.as_str()),
        ("GET", "/settings/slot-minutes"),
        ("POST", "/appointments/nobody/cancel"),
    ] {
        let (status, _) = call_as(&h.app, None, method, uri, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }

    for role in ROLES {
        let session = match role.as_str() {
            "owner" => common::OWNER_SESSION,
            "manager" => common::MANAGER_SESSION,
            _ => common::CASHIER_SESSION,
        };
        let (status, body) = call_as(&h.app, Some(session), "GET", &list, None).await;
        if can(role, Permission::ViewPatients) {
            assert_eq!(status, StatusCode::OK, "{role:?}: {body}");
        } else {
            assert_eq!(status, StatusCode::FORBIDDEN, "{role:?}: {body}");
        }
        // An id nobody made: allowed through the gate, not found after it.
        let (status, body) = call_as(
            &h.app,
            Some(session),
            "POST",
            "/appointments/nobody/cancel",
            None,
        )
        .await;
        if can(role, Permission::EditPatients) {
            assert_eq!(status, StatusCode::NOT_FOUND, "{role:?}: {body}");
        } else {
            assert_eq!(status, StatusCode::FORBIDDEN, "{role:?}: {body}");
            assert_eq!(body["error"]["permission"], "edit_patients");
        }
        // The slot length is a setting: who may not change settings is
        // refused it, whatever they may do in the book. The length in force,
        // so a pass writes nothing.
        let (status, body) = call_as(
            &h.app,
            Some(session),
            "PUT",
            "/settings/slot-minutes",
            Some(json!({ "slot_minutes": 15 })),
        )
        .await;
        if can(role, Permission::EditSettings) {
            assert_eq!(status, StatusCode::OK, "{role:?}: {body}");
        } else {
            assert_eq!(status, StatusCode::FORBIDDEN, "{role:?}: {body}");
            assert_eq!(body["error"]["permission"], "edit_settings");
        }
    }
}
