// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// C5b of `the-first-clinic-module-patients-queue-appointments`: every route
// here exists only with the clinic built in, so `just check-clinic-only` is
// what runs this file, as it runs `book_api.rs`.
#![cfg(feature = "clinic")]

//! The book tools over HTTP: a real temp SQLite file, the real router, the
//! real session and gate layers. The book refuses the past on the wall
//! clock, so every case books on days counted from tomorrow.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{Datelike, Duration, NaiveDate, Weekday};
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

/// The first `weekday` from tomorrow on.
fn next(weekday: Weekday) -> NaiveDate {
    let mut day = clock::now().date() + Duration::days(1);
    while day.weekday() != weekday {
        day = day.succ_opt().unwrap();
    }
    day
}

async fn book(app: &axum::Router, body: Value) -> (StatusCode, Value) {
    call(app, "POST", "/appointments", Some(body)).await
}

fn assert_refused(status: StatusCode, body: &Value, wanted: StatusCode, field: &str) {
    assert_eq!(status, wanted, "{body}");
    assert_eq!(body["error"]["field"], field, "{body}");
}

/// Sunday to Thursday with a lunch break, Friday and Saturday closed.
fn algerian_week() -> Value {
    let day = json!([
        { "opens": "08:00", "closes": "12:00" },
        { "opens": "14:00", "closes": "18:00" }
    ]);
    json!({ "days": [day, day, day, day, day, [], []] })
}

#[tokio::test]
async fn the_week_is_read_open_written_whole_and_closes_the_book_outside_it() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/settings/working-hours", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "days": null }));

    for bad in [
        json!({ "days": [[{ "opens": "8:00", "closes": "12:00" }], [], [], [], [], [], []] }),
        json!({ "days": [[{ "opens": "08:00", "closes": "24:01" }], [], [], [], [], [], []] }),
        json!({ "days": [[{ "opens": "08:00", "closes": "12:60" }], [], [], [], [], [], []] }),
        json!({ "days": [[], [], [], [], [], [], []] }),
        json!({ "days": [[], [], []] }),
    ] {
        let (status, body) =
            call(&h.app, "PUT", "/settings/working-hours", Some(bad.clone())).await;
        assert_refused(status, &body, StatusCode::UNPROCESSABLE_ENTITY, "days");
    }
    let (status, body) = call(
        &h.app,
        "PUT",
        "/settings/working-hours",
        Some(json!({ "days": [[{ "opens": "22:00", "closes": "24:00" }], [], [], [], [], [], []], "extra": 1 })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");

    let (status, body) = call(
        &h.app,
        "PUT",
        "/settings/working-hours",
        Some(algerian_week()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, algerian_week());
    let (_, read) = call(&h.app, "GET", "/settings/working-hours", None).await;
    assert_eq!(read, algerian_week());

    let benali = patient(&h.app, "Amina", "Benali").await;
    let sunday = next(Weekday::Sun);
    let (status, body) = book(
        &h.app,
        json!({ "patient_id": benali, "starts_at": format!("{sunday} 12:00:00"), "note": null }),
    )
    .await;
    assert_refused(status, &body, StatusCode::UNPROCESSABLE_ENTITY, "starts_at");
    let (status, body) = book(
        &h.app,
        json!({ "patient_id": benali, "starts_at": format!("{} 09:00:00", next(Weekday::Fri)), "note": null }),
    )
    .await;
    assert_refused(status, &body, StatusCode::UNPROCESSABLE_ENTITY, "starts_at");
    let (status, body) = book(
        &h.app,
        json!({ "patient_id": benali, "starts_at": format!("{sunday} 11:45:00"), "note": null }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

#[tokio::test]
async fn a_block_answers_with_its_hits_refuses_bookings_and_is_removed() {
    let h = harness();
    let benali = patient(&h.app, "Amina", "Benali").await;
    let haddad = patient(&h.app, "Karim", "Haddad").await;
    let day = next(Weekday::Sun);
    for (who, at) in [(&benali, "09:00:00"), (&haddad, "10:00:00")] {
        let (status, body) = book(
            &h.app,
            json!({ "patient_id": who, "starts_at": format!("{day} {at}"), "note": null }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
    }
    for (bad, field) in [
        (
            json!({ "starts_at": format!("{day}T09:00:00"), "ends_at": format!("{day} 10:00:00"), "label": null }),
            "starts_at",
        ),
        (
            json!({ "starts_at": format!("{day} 09:00:00"), "ends_at": format!("{day} 10:00"), "label": null }),
            "ends_at",
        ),
        (
            json!({ "starts_at": format!("{day} 09:00:00"), "ends_at": format!("{day} 09:00:00"), "label": null }),
            "ends_at",
        ),
    ] {
        let (status, body) = call(&h.app, "POST", "/absence-blocks", Some(bad)).await;
        assert_refused(status, &body, StatusCode::UNPROCESSABLE_ENTITY, field);
    }
    // 09:15 to 10:15 lands on Haddad; Benali's 09:00-09:15 only touches it.
    let (status, made) = call(
        &h.app,
        "POST",
        "/absence-blocks",
        Some(json!({ "starts_at": format!("{day} 09:15:00"), "ends_at": format!("{day} 10:15:00"), "label": "congrès" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["block"]["label"], "congrès");
    assert_eq!(made["block"]["starts_at"], format!("{day} 09:15:00"));
    let hits = made["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1, "{made}");
    assert_eq!(hits[0]["last_name"], "Haddad");
    assert_eq!(hits[0]["starts_at"], format!("{day} 10:00:00"));

    let (status, body) = book(
        &h.app,
        json!({ "patient_id": benali, "starts_at": format!("{day} 09:30:00"), "note": null }),
    )
    .await;
    assert_refused(status, &body, StatusCode::CONFLICT, "starts_at");
    let (_, list) = call(&h.app, "GET", "/absence-blocks", None).await;
    assert_eq!(list["blocks"].as_array().unwrap().len(), 1, "{list}");

    let id = made["block"]["id"].as_str().unwrap().to_string();
    let (status, gone) = call(
        &h.app,
        "POST",
        &format!("/absence-blocks/{id}/remove"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert_eq!(gone["id"], id.as_str());
    let (status, _) = call(
        &h.app,
        "POST",
        &format!("/absence-blocks/{id}/remove"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, body) = book(
        &h.app,
        json!({ "patient_id": benali, "starts_at": format!("{day} 09:30:00"), "note": null }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

#[tokio::test]
async fn a_visit_type_is_set_up_and_a_booking_that_names_it_takes_its_length() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/settings/visit-types", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "visit_types": [] }));
    let (status, body) = call(
        &h.app,
        "POST",
        "/settings/visit-types",
        Some(json!({ "name": "Contrôle", "minutes": 20 })),
    )
    .await;
    assert_refused(status, &body, StatusCode::UNPROCESSABLE_ENTITY, "minutes");
    let (status, made) = call(
        &h.app,
        "POST",
        "/settings/visit-types",
        Some(json!({ "name": "Première consultation", "minutes": 30 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["minutes"], 30);
    let (status, body) = call(
        &h.app,
        "POST",
        "/settings/visit-types",
        Some(json!({ "name": "première CONSULTATION", "minutes": 45 })),
    )
    .await;
    // The same name in other capitals is the same type.
    assert_refused(status, &body, StatusCode::CONFLICT, "name");
    let id = made["id"].as_str().unwrap().to_string();

    let benali = patient(&h.app, "Amina", "Benali").await;
    let day = next(Weekday::Mon);
    let (status, booked) = book(
        &h.app,
        json!({ "patient_id": benali, "starts_at": format!("{day} 09:00:00"), "note": null, "visit_type_id": id }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{booked}");
    assert_eq!(booked["slot_minutes"], 30);
    let (status, body) = book(
        &h.app,
        json!({ "patient_id": benali, "starts_at": format!("{day} 09:15:00"), "note": null }),
    )
    .await;
    assert_refused(status, &body, StatusCode::CONFLICT, "starts_at");
    let (status, _) = book(
        &h.app,
        json!({ "patient_id": benali, "starts_at": format!("{day} 11:00:00"), "note": null, "visit_type_id": "nobody" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, body) = call(
        &h.app,
        "PUT",
        &format!("/settings/visit-types/{id}"),
        Some(json!({ "name": "Consultation", "minutes": 60 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["minutes"], 60);
    let (_, read) = call(
        &h.app,
        "GET",
        &format!("/appointments/{}", booked["id"].as_str().unwrap()),
        None,
    )
    .await;
    assert_eq!(read["slot_minutes"], 30);
    let (status, gone) = call(
        &h.app,
        "POST",
        &format!("/settings/visit-types/{id}/remove"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert_eq!(gone["name"], "Consultation");
    let (_, list) = call(&h.app, "GET", "/settings/visit-types", None).await;
    assert_eq!(list, json!({ "visit_types": [] }));
}

#[tokio::test]
async fn the_next_free_slot_crosses_the_weekend_and_takes_an_offset_or_a_length() {
    let h = harness();
    let (status, body) = call(
        &h.app,
        "PUT",
        "/settings/working-hours",
        Some(algerian_week()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let friday = next(Weekday::Fri);
    let sunday = friday + Duration::days(2);
    let (status, found) = call(
        &h.app,
        "GET",
        &format!("/appointments/next-free?from={friday}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{found}");
    assert_eq!(
        found,
        json!({
            "first_day": friday.to_string(),
            "last_day": (friday + Duration::days(59)).to_string(),
            "slot_minutes": 15,
            "starts_at": format!("{sunday} 08:00:00"),
        })
    );
    // Asked by length instead of a visit type: the answer searched 45.
    let (status, found) = call(
        &h.app,
        "GET",
        &format!("/appointments/next-free?from={friday}&minutes=45"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{found}");
    assert_eq!(
        (&found["slot_minutes"], &found["starts_at"]),
        (&json!(45), &json!(format!("{sunday} 08:00:00")))
    );
    // Fifteen days after that Friday is a Saturday; the Sunday after it.
    let (_, found) = call(
        &h.app,
        "GET",
        &format!("/appointments/next-free?from={friday}&offset_days=15"),
        None,
    )
    .await;
    assert_eq!(
        found["first_day"],
        (friday + Duration::days(15)).to_string()
    );
    assert_eq!(
        found["starts_at"],
        format!("{} 08:00:00", friday + Duration::days(16))
    );
    for bad in [
        "/appointments/next-free?from=2026-9-1",
        "/appointments/next-free?offset_days=-1",
        "/appointments/next-free?offset_days=400",
        "/appointments/next-free?day=2026-09-01",
        "/appointments/next-free?minutes=half",
        "/appointments/next-free?minutes=0",
        "/appointments/next-free?minutes=241",
        "/appointments/next-free?minutes=30&visit_type_id=any",
    ] {
        let (status, body) = call(&h.app, "GET", bad, None).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}: {body}");
    }
    let (status, _) = call(
        &h.app,
        "GET",
        "/appointments/next-free?visit_type_id=nobody",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_appointment_is_marked_missed_before_or_after_its_start_twice_changes_nothing_and_a_cancel_clears_the_mark(
) {
    use diesel::RunQueryDsl;
    let h = harness();
    let benali = patient(&h.app, "Amina", "Benali").await;
    let tomorrow = clock::now().date() + Duration::days(1);
    let (_, future) = book(
        &h.app,
        json!({ "patient_id": benali, "starts_at": format!("{tomorrow} 09:00:00"), "note": null }),
    )
    .await;
    assert_eq!(future["no_show_at"], Value::Null);
    let id = future["id"].as_str().unwrap();
    let (status, marked) = call(&h.app, "POST", &format!("/appointments/{id}/no-show"), None).await;
    assert_eq!(status, StatusCode::OK, "{marked}");
    assert!(marked["no_show_at"].is_string(), "{marked}");

    // The book refuses the past, so yesterday's row is written past it.
    let yesterday = clock::now().date() - Duration::days(1);
    let past = "0199a0c1-0000-7000-8000-0000000e0002";
    let mut conn = dzpos_core::db::open(&h.path).unwrap();
    diesel::sql_query(format!(
        "INSERT INTO appointments (id, shop_id, patient_id, starts_at, slot_minutes, \
         created_at, updated_at) VALUES ('{past}', 1, '{benali}', '{yesterday} 09:00:00', 15, \
         '2026-09-01 08:00:00', '2026-09-01 08:00:00')"
    ))
    .execute(&mut conn)
    .unwrap();
    let (status, marked) = call(
        &h.app,
        "POST",
        &format!("/appointments/{past}/no-show"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{marked}");
    assert!(marked["no_show_at"].is_string(), "{marked}");
    // Pressed twice, the mark stands with its first stamp.
    let (status, again) = call(
        &h.app,
        "POST",
        &format!("/appointments/{past}/no-show"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(again["no_show_at"], marked["no_show_at"]);
    let (status, cleared) = call(
        &h.app,
        "POST",
        &format!("/appointments/{past}/no-show/clear"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{cleared}");
    assert_eq!(cleared["no_show_at"], Value::Null);
    let (status, again) = call(
        &h.app,
        "POST",
        &format!("/appointments/{past}/no-show/clear"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(again, cleared);

    // The future one, still marked, is cancelled and loses its mark.
    let (status, cancelled) =
        call(&h.app, "POST", &format!("/appointments/{id}/cancel"), None).await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert!(cancelled["cancelled_at"].is_string(), "{cancelled}");
    assert_eq!(cancelled["no_show_at"], Value::Null);
}

#[tokio::test]
async fn the_day_list_holds_the_book_and_the_walk_ins_of_one_day() {
    let h = harness();
    let benali = patient(&h.app, "Amina", "Benali").await;
    let haddad = patient(&h.app, "Karim", "Haddad").await;
    let tomorrow = clock::now().date() + Duration::days(1);
    for (who, at) in [(&haddad, "10:00:00"), (&benali, "09:00:00")] {
        let (status, body) = book(
            &h.app,
            json!({ "patient_id": who, "starts_at": format!("{tomorrow} {at}"), "note": null }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
    }
    let (status, body) = call(
        &h.app,
        "POST",
        "/queue",
        Some(json!({ "patient_id": haddad })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let (status, list) = call(&h.app, "GET", &format!("/day-list?day={tomorrow}"), None).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(list["day"], tomorrow.to_string());
    let names = |key: &str| -> Vec<String> {
        list[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["last_name"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(names("appointments"), ["Benali", "Haddad"]);
    assert!(names("walk_ins").is_empty());
    let today = clock::now().date();
    let (_, list) = call(&h.app, "GET", &format!("/day-list?day={today}"), None).await;
    assert_eq!(list["walk_ins"][0]["last_name"], "Haddad", "{list}");
    assert_eq!(list["appointments"], json!([]));
    for bad in [
        "/day-list",
        "/day-list?day=2026-9-1",
        "/day-list?week=2026-09-01",
    ] {
        let (status, body) = call(&h.app, "GET", bad, None).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}: {body}");
    }
}

/// Each tool's routes against the permission table, and each role against
/// the writes over HTTP.
#[tokio::test]
async fn the_book_tools_ask_edit_settings_for_the_setup_and_the_patient_files_for_the_desk() {
    for (method, path, wanted) in [
        (
            "PUT",
            "/settings/working-hours",
            Some(Permission::EditSettings),
        ),
        ("GET", "/settings/working-hours", None),
        ("POST", "/absence-blocks", Some(Permission::EditPatients)),
        (
            "POST",
            "/absence-blocks/{id}/remove",
            Some(Permission::EditPatients),
        ),
        ("GET", "/absence-blocks", None),
        (
            "POST",
            "/settings/visit-types",
            Some(Permission::EditSettings),
        ),
        (
            "PUT",
            "/settings/visit-types/{id}",
            Some(Permission::EditSettings),
        ),
        (
            "POST",
            "/settings/visit-types/{id}/remove",
            Some(Permission::EditSettings),
        ),
        ("GET", "/settings/visit-types", None),
        (
            "GET",
            "/appointments/next-free",
            Some(Permission::ViewPatients),
        ),
        (
            "POST",
            "/appointments/{id}/no-show",
            Some(Permission::EditPatients),
        ),
        (
            "POST",
            "/appointments/{id}/no-show/clear",
            Some(Permission::EditPatients),
        ),
        ("GET", "/day-list", Some(Permission::ViewPatients)),
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
    let (status, _) = call_as(&h.app, None, "GET", "/settings/working-hours", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    for role in ROLES {
        let session = match role.as_str() {
            "owner" => common::OWNER_SESSION,
            "manager" => common::MANAGER_SESSION,
            _ => common::CASHIER_SESSION,
        };
        let (status, body) = call_as(
            &h.app,
            Some(session),
            "PUT",
            "/settings/working-hours",
            Some(algerian_week()),
        )
        .await;
        if can(role, Permission::EditSettings) {
            assert_eq!(status, StatusCode::OK, "{role:?}: {body}");
        } else {
            assert_eq!(status, StatusCode::FORBIDDEN, "{role:?}: {body}");
            assert_eq!(body["error"]["permission"], "edit_settings");
        }
        let (status, _) = call_as(
            &h.app,
            Some(session),
            "GET",
            "/settings/working-hours",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{role:?}");
        // The open reads: no name in them.
        for read in ["/absence-blocks", "/settings/visit-types"] {
            let (status, body) = call_as(&h.app, Some(session), "GET", read, None).await;
            assert_eq!(status, StatusCode::OK, "{role:?} {read}: {body}");
        }
        // The reads of the book: the day list and the next free slot.
        let day_list = format!("/day-list?day={}", clock::now().date());
        for read in [day_list.as_str(), "/appointments/next-free"] {
            let (status, body) = call_as(&h.app, Some(session), "GET", read, None).await;
            if can(role, Permission::ViewPatients) {
                assert_eq!(status, StatusCode::OK, "{role:?} {read}: {body}");
            } else {
                assert_eq!(status, StatusCode::FORBIDDEN, "{role:?} {read}: {body}");
                assert_eq!(body["error"]["permission"], "view_patients");
            }
        }
        // The desk's writes: an id nobody made passes the gate and is not
        // found, or is refused at it.
        for (write, wanted, name) in [
            (
                "/absence-blocks/nobody/remove",
                Permission::EditPatients,
                "edit_patients",
            ),
            (
                "/appointments/nobody/no-show",
                Permission::EditPatients,
                "edit_patients",
            ),
            (
                "/settings/visit-types/nobody/remove",
                Permission::EditSettings,
                "edit_settings",
            ),
        ] {
            let (status, body) = call_as(&h.app, Some(session), "POST", write, None).await;
            if can(role, wanted) {
                assert_eq!(status, StatusCode::NOT_FOUND, "{role:?} {write}: {body}");
            } else {
                assert_eq!(status, StatusCode::FORBIDDEN, "{role:?} {write}: {body}");
                assert_eq!(body["error"]["permission"], name);
            }
        }
    }
}
