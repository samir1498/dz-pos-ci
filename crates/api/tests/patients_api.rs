// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// C3 of `the-first-clinic-module-patients-queue-appointments`: every route
// here exists only with the clinic built in. `just check-clinic-only` is
// what runs this file, with no shop in the build at all; no gate builds it
// beside the shop (`just types` compiles `export_bindings` alone).
#![cfg(feature = "clinic")]

//! The patient file over HTTP: a real temp SQLite file, the real router,
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

fn amina() -> Value {
    json!({
        "first_name": "Amina",
        "last_name": "Benali",
        "sex": "female",
        "date_of_birth": "1990-05-17",
        "phone": "0555 12 34 56",
        "notes": null,
    })
}

fn ids(list: &Value) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn a_patient_created_is_found_by_a_search() {
    let h = harness();
    let (status, made) = call(&h.app, "POST", "/patients", Some(amina())).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["first_name"], "Amina");
    assert_eq!(made["sex"], "female");
    assert_eq!(made["date_of_birth"], "1990-05-17");
    assert_eq!(made["phone"], "0555123456");
    assert_eq!(made["archived_at"], Value::Null);
    let id = made["id"].as_str().unwrap().to_string();

    for q in ["benal", "AMINA", "12%2034"] {
        let (status, found) = call(&h.app, "GET", &format!("/patients?q={q}"), None).await;
        assert_eq!(status, StatusCode::OK, "{found}");
        assert_eq!(ids(&found), std::slice::from_ref(&id), "{q}");
    }
    let (_, none) = call(&h.app, "GET", "/patients?q=haddad", None).await;
    assert_eq!(ids(&none), Vec::<String>::new());

    let (status, one) = call(&h.app, "GET", &format!("/patients/{id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{one}");
    assert_eq!(one, made);
}

#[tokio::test]
async fn an_archived_patient_is_hidden_from_the_search() {
    let h = harness();
    let (_, made) = call(&h.app, "POST", "/patients", Some(amina())).await;
    let id = made["id"].as_str().unwrap().to_string();

    let (status, archived) = call(&h.app, "POST", &format!("/patients/{id}/archive"), None).await;
    assert_eq!(status, StatusCode::OK, "{archived}");
    assert!(archived["archived_at"].is_string(), "{archived}");

    let (_, live) = call(&h.app, "GET", "/patients?q=benali", None).await;
    assert_eq!(ids(&live), Vec::<String>::new());
    let (_, everyone) = call(&h.app, "GET", "/patients", None).await;
    assert_eq!(ids(&everyone), Vec::<String>::new());
    let (_, with_archived) = call(&h.app, "GET", "/patients?q=benali&archived=true", None).await;
    assert_eq!(ids(&with_archived), std::slice::from_ref(&id));

    // A second archive is a conflict, not a second stamp.
    let (status, again) = call(&h.app, "POST", &format!("/patients/{id}/archive"), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{again}");
    assert_eq!(again["error"]["code"], "conflict");
    assert_eq!(again["error"]["field"], "archived_at");
}

#[tokio::test]
async fn another_shops_patient_is_invisible() {
    let h = harness();
    let theirs_app = common::signed_in_router(&h.path, 2, &token());
    let (status, theirs) = call(&theirs_app, "POST", "/patients", Some(amina())).await;
    assert_eq!(status, StatusCode::CREATED, "{theirs}");
    let id = theirs["id"].as_str().unwrap().to_string();

    let (_, list) = call(&h.app, "GET", "/patients?q=benali&archived=true", None).await;
    assert_eq!(ids(&list), Vec::<String>::new());

    for (method, uri, body) in [
        ("GET", format!("/patients/{id}"), None),
        ("PUT", format!("/patients/{id}"), Some(amina())),
        ("POST", format!("/patients/{id}/archive"), None),
    ] {
        let (status, body) = call(&h.app, method, &uri, body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}: {body}");
        assert_eq!(body["error"]["code"], "not_found", "{method} {uri}");
    }
    // Still there, untouched, in its own shop.
    let (status, still) = call(&theirs_app, "GET", &format!("/patients/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(still, theirs);
}

#[tokio::test]
async fn two_ids_made_in_a_row_differ_and_sort_by_creation() {
    let h = harness();
    let (_, first) = call(&h.app, "POST", "/patients", Some(amina())).await;
    // Same names on purpose: the list's name order ties, and only the id
    // decides which comes first.
    let (_, second) = call(&h.app, "POST", "/patients", Some(amina())).await;
    let (a, b) = (
        first["id"].as_str().unwrap().to_string(),
        second["id"].as_str().unwrap().to_string(),
    );
    assert_ne!(a, b);
    assert!(a < b, "{a} was made first and sorts after {b}");
    let (_, list) = call(&h.app, "GET", "/patients", None).await;
    assert_eq!(ids(&list), [a, b]);
}

#[tokio::test]
async fn an_update_rewrites_the_file_and_a_bad_field_is_refused_by_name() {
    let h = harness();
    let (_, made) = call(&h.app, "POST", "/patients", Some(amina())).await;
    let id = made["id"].as_str().unwrap().to_string();

    let mut changed = amina();
    changed["last_name"] = json!("Benali-Saidi");
    changed["phone"] = Value::Null;
    let (status, after) = call(&h.app, "PUT", &format!("/patients/{id}"), Some(changed)).await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after["last_name"], "Benali-Saidi");
    assert_eq!(after["phone"], Value::Null);
    assert_eq!(after["created_at"], made["created_at"]);

    for (field, value) in [
        ("date_of_birth", json!("1990-5-17")),
        ("date_of_birth", json!("2999-01-01")),
        ("first_name", json!("  ")),
        ("phone", json!("12")),
    ] {
        let mut bad = amina();
        bad[field] = value.clone();
        let (status, body) = call(&h.app, "POST", "/patients", Some(bad)).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{field} {value}: {body}"
        );
        assert_eq!(body["error"]["code"], "validation", "{field} {value}");
        assert_eq!(body["error"]["field"], field, "{field} {value}");
    }
    let mut unknown = amina();
    unknown["blood_group"] = json!("O+");
    let (status, _) = call(&h.app, "POST", "/patients", Some(unknown)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Only the one good file was made.
    let (_, list) = call(&h.app, "GET", "/patients?archived=true", None).await;
    assert_eq!(ids(&list), [id]);
}

/// The gate half. Every patient route has a row naming one of the clinic's
/// two permissions, reads `ViewPatients` and writes `EditPatients`, and a
/// call with no session is refused before any of them.
///
/// Why there is no 403 case here: `services::permissions::can` gives both
/// permissions to every role (a cabinet's cashier-level user is its
/// receptionist, C3's ruling), so no signed-in role exists that the gate
/// could refuse. The loop below asks `can` rather than assuming that, so the
/// day one role loses either permission it asserts that role's 403 here,
/// and `route_gates.rs`'s walk asserts it on every row as well.
#[tokio::test]
async fn every_patient_route_is_gated_on_the_clinics_two_permissions() {
    for (method, path, wanted) in [
        ("GET", "/patients", Permission::ViewPatients),
        ("GET", "/patients/{id}", Permission::ViewPatients),
        ("POST", "/patients", Permission::EditPatients),
        ("PUT", "/patients/{id}", Permission::EditPatients),
        ("POST", "/patients/{id}/archive", Permission::EditPatients),
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

    let (status, body) = call_as(&h.app, None, "GET", "/patients", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    let (status, _) = call_as(&h.app, None, "POST", "/patients", Some(amina())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    for role in ROLES {
        let session = match role.as_str() {
            "owner" => common::OWNER_SESSION,
            "manager" => common::MANAGER_SESSION,
            _ => common::CASHIER_SESSION,
        };
        let (status, body) =
            call_as(&h.app, Some(session), "POST", "/patients", Some(amina())).await;
        if can(role, Permission::EditPatients) {
            assert_eq!(status, StatusCode::CREATED, "{role:?}: {body}");
        } else {
            assert_eq!(status, StatusCode::FORBIDDEN, "{role:?}: {body}");
            assert_eq!(body["error"]["permission"], "edit_patients");
        }
        let (status, body) = call_as(&h.app, Some(session), "GET", "/patients", None).await;
        if can(role, Permission::ViewPatients) {
            assert_eq!(status, StatusCode::OK, "{role:?}: {body}");
        } else {
            assert_eq!(status, StatusCode::FORBIDDEN, "{role:?}: {body}");
            assert_eq!(body["error"]["permission"], "view_patients");
        }
    }
}
