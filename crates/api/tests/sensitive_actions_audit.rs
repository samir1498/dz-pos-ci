// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! A permission refusal, an export and a restore each used to leave
//! nothing in the audit log. `tests/audit_log.rs` already proves the read
//! side of the log against a row `services::products::update` writes; this
//! file drives the three writes this finding is about and proves the same
//! thing about each of them.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
#[cfg(feature = "retail")]
use serde_json::json;
use serde_json::Value;
use tower::ServiceExt;

mod common;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).unwrap()
}

struct Harness {
    // Held for its `Drop`, not read: the temp file it names has to outlive
    // `app`. Only the retail-gated tests below read `h.dir.path()` itself, so
    // a no-shop build never reads the field by name.
    #[cfg_attr(not(feature = "retail"), allow(dead_code))]
    dir: tempfile::TempDir,
    app: axum::Router,
}

fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    Harness {
        dir,
        app: dzpos_api::router(state, &token()),
    }
}

/// `session` is `None` for a call that carries no session at all: only the
/// launch token, the way a stranger with the desktop's token but no sign-in
/// would call.
async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    session: Option<&str>,
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

async fn owner(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    call(app, method, uri, body, Some(common::OWNER_SESSION)).await
}

async fn audit_rows(app: &axum::Router) -> Vec<Value> {
    let (status, page) = owner(app, "GET", "/audit-log", None).await;
    assert_eq!(status, StatusCode::OK, "{page}");
    page["rows"].as_array().cloned().unwrap_or_default()
}

// S7: only the two retail-gated tests below call this.
#[cfg(feature = "retail")]
fn product(name: &str) -> Value {
    json!({
        "name": name,
        "category_id": 1,
        "unit": "piece",
        "cost_centimes": 820,
        "selling_centimes": 920
    })
}

/// The finding's own example: a cashier tries the export route, is refused,
/// and used to leave nothing an owner reading the log could see. `GET
/// /export/products` is deliberately a read and not a write: `gates/`
/// still names `ExportAndImport` for it, so the refusal is exactly as
/// deliberate as one on a write, which is the whole reason a read is not
/// excluded here.
// S7 of `a-kernel-crate-and-retail-as-the-first-module`: `/export/products`
// is retail-only (`routes::mod.rs`).
#[cfg(feature = "retail")]
#[tokio::test]
async fn a_permission_refusal_writes_a_row_naming_who_the_permission_and_the_route() {
    let h = harness();
    let path = h.dir.path().join("t.db");
    common::sign_in_as(&path, SHOP, "cashier", common::CASHIER_SESSION);

    let (status, refused) = call(
        &h.app,
        "GET",
        "/export/products?lang=fr",
        None,
        Some(common::CASHIER_SESSION),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(refused["error"]["code"], "forbidden");

    let rows = audit_rows(&h.app).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row["action"], "permission.refused");
    assert_eq!(row["entity"], "permission");
    assert_eq!(row["user_name"], "Test cashier", "{row}");

    let after: Value = serde_json::from_str(row["after"].as_str().unwrap()).unwrap();
    assert_eq!(after["permission"], "export_and_import");
    assert_eq!(after["route"], "/export/products");
    assert_eq!(after["method"], "GET");
}

/// A refused route is not the only kind of refusal: a request with no
/// session at all is answered `session_required` before anybody is known,
/// and there is nobody to write a row about. `crates/api/src/session.rs`'s
/// own doc is the decision; this is the negative half of the test above,
/// proving the log is not filled with rows about nobody.
// S7: `/export/products` is retail-only (`routes::mod.rs`).
#[cfg(feature = "retail")]
#[tokio::test]
async fn a_request_with_no_session_writes_no_row() {
    let h = harness();

    let (status, refused) = call(&h.app, "GET", "/export/products?lang=fr", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
    assert_eq!(refused["error"]["code"], "session_required");

    assert_eq!(audit_rows(&h.app).await, Vec::<Value>::new());
}

/// The support bundle is a file on its way out of the shop, so the gate
/// table and the architecture page both promise it leaves a row behind the
/// same way an export does. Until this test the promise rested on a
/// constant being defined and called, with nothing reading it back.
#[tokio::test]
async fn a_support_bundle_writes_a_row_naming_who_asked_for_it() {
    let h = harness();

    let (status, _) = owner(&h.app, "GET", "/support-bundle", None).await;
    assert_eq!(status, StatusCode::OK);

    let rows = audit_rows(&h.app).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row["action"], "support_bundle.download");
    assert_eq!(row["entity"], "support_bundle");
    assert_eq!(row["user_name"], "Propriétaire", "{row}");
}

/// An export writes a row naming which one and how many rows walked out with
/// it, so an owner reading the log afterwards sees what left the shop even
/// when nothing was ever refused.
// S7: `/products` and `/export/products` are both retail-only
// (`routes::mod.rs`).
#[cfg(feature = "retail")]
#[tokio::test]
async fn an_export_writes_a_row_naming_which_one_and_how_many_rows() {
    let h = harness();
    owner(&h.app, "POST", "/products", Some(product("Semoule 10kg"))).await;
    owner(&h.app, "POST", "/products", Some(product("Huile Elio 5L"))).await;

    let (status, _) = owner(&h.app, "GET", "/export/products?lang=fr", None).await;
    assert_eq!(status, StatusCode::OK);

    let rows = audit_rows(&h.app).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row["action"], "export.download");
    assert_eq!(row["entity"], "export");

    let after: Value = serde_json::from_str(row["after"].as_str().unwrap()).unwrap();
    assert_eq!(after["which"], "products");
    assert_eq!(after["rows"], 2);
}

/// A restore rewrites every row the shop has, and used to leave nothing of
/// its own behind. Taking a backup, by contrast, is deliberately not logged
/// (`services::backup::create`'s own doc says why): this test pins both
/// halves of that decision in one place so neither drifts without the other
/// being noticed.
// S7: the restore's own audit row carries `BackupCounts` (`after["products"]`
// below), retail-only (`Restored::summary` is `()` with the feature off,
// `crates/api/src/lib.rs`), and the fixture data is written through
// `/products`.
#[cfg(feature = "retail")]
#[tokio::test]
async fn a_backup_writes_nothing_but_a_restore_writes_a_row() {
    let h = harness();
    owner(&h.app, "POST", "/products", Some(product("Semoule 10kg"))).await;

    let (status, made) = owner(&h.app, "POST", "/backups", None).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    let name = made["name"].as_str().unwrap().to_string();
    assert_eq!(
        audit_rows(&h.app).await,
        Vec::<Value>::new(),
        "taking a backup wrote a row despite the decision that it should not"
    );

    let (status, back) = owner(&h.app, "POST", &format!("/backups/{name}/restore"), None).await;
    assert_eq!(status, StatusCode::OK, "{back}");

    let rows = audit_rows(&h.app).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row["action"], "backup.restore");
    assert_eq!(row["entity"], "backup");
    assert_eq!(row["user_name"], "Propriétaire", "{row}");

    let after: Value = serde_json::from_str(row["after"].as_str().unwrap()).unwrap();
    assert_eq!(after["restored_from"], name);
    assert_eq!(after["products"], 1);
}
