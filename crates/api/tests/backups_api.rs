// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The backup routes and the restore that swaps the live file underneath a
//! running server. In-process router, real temp SQLite file, real copies on
//! disk: a restore that only passed against a mock would be worthless.

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
    dir: tempfile::TempDir,
    app: axum::Router,
}

impl Harness {
    fn db(&self) -> std::path::PathBuf {
        self.dir.path().join("t.db")
    }

    /// The copy taken just before a restore, kept beside the shop file and
    /// never counted among the thirty.
    fn safety_copies(&self) -> Vec<std::path::PathBuf> {
        let mut found: Vec<std::path::PathBuf> = std::fs::read_dir(self.dir.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    // The suffix matters: opening one of these copies leaves
                    // its own `-wal` and `-shm` beside it, and those are not
                    // copies.
                    .is_some_and(|n| {
                        n.starts_with("t.db.before-restore-") && n.ends_with(".sqlite")
                    })
            })
            .collect();
        found.sort();
        found
    }
}

/// Anything left beside the shop file under the name a restore stages its
/// copy at. Matched by suffix rather than by the one name, so a second
/// staging name would be caught too.
fn staged_copies(h: &Harness) -> Vec<std::path::PathBuf> {
    let mut found: Vec<std::path::PathBuf> = std::fs::read_dir(h.dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".restoring.tmp"))
        })
        .collect();
    found.sort();
    found
}

/// Everything in the backup folder, sorted, so two readings compare.
fn backup_files(h: &Harness) -> Vec<std::path::PathBuf> {
    let dir = h.dir.path().join("backups");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut found: Vec<std::path::PathBuf> =
        entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    found.sort();
    found
}

fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    Harness {
        dir,
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

fn product(name: &str) -> Value {
    json!({
        "name": name,
        "category_id": 1,
        "unit": "piece",
        "cost_centimes": 820,
        "selling_centimes": 920
    })
}

async fn product_names(app: &axum::Router) -> Vec<String> {
    let (status, body) = call(app, "GET", "/products", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body.as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn a_fresh_shop_has_no_copies_and_a_post_makes_one() {
    let h = harness();
    let (status, body) = call(&h.app, "GET", "/backups", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!({ "backups": [], "safety_copies": [] }));

    let (status, made) = call(&h.app, "POST", "/backups", None).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    let name = made["name"].as_str().unwrap().to_string();
    assert!(
        regex_lite_matches(&name),
        "{name} is not dzpos-YYYYMMDD-HHMMSS.sqlite"
    );
    assert!(made["bytes"].as_i64().unwrap() > 0);
    assert_eq!(made["taken_at"].as_str().unwrap().len(), 19, "{made}");

    // The folder is beside the shop file, made on demand.
    let on_disk = h.dir.path().join("backups").join(&name);
    assert!(on_disk.is_file(), "{on_disk:?} was not written");

    let (_, listed) = call(&h.app, "GET", "/backups", None).await;
    assert_eq!(listed["backups"].as_array().unwrap().len(), 1);
    assert_eq!(listed["backups"][0], made);
    assert_eq!(listed["safety_copies"], json!([]), "nothing was restored");
}

#[tokio::test]
async fn a_restore_puts_the_file_back_and_keeps_what_was_there_in_a_safety_copy() {
    let h = harness();
    let (status, _) = call(&h.app, "POST", "/products", Some(product("Semoule 10kg"))).await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, made) = call(&h.app, "POST", "/backups", None).await;
    let name = made["name"].as_str().unwrap().to_string();

    // Added after the copy: the restore must lose it, and the safety copy
    // must be the only place it survives.
    let (status, _) = call(&h.app, "POST", "/products", Some(product("Huile Elio 5L"))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(product_names(&h.app).await.len(), 2);

    let (status, back) = call(&h.app, "POST", &format!("/backups/{name}/restore"), None).await;
    assert_eq!(status, StatusCode::OK, "{back}");
    assert_eq!(back["restored_from"], json!(name));
    assert_eq!(back["products"], json!(1));
    // The documents table exists since migration 2 and holds nothing yet.
    assert_eq!(back["documents"], json!(0));
    // The copy of what is being replaced is named to the caller: it is the
    // only record of it, and nothing deletes it.
    let safety_name = back["safety_copy"].as_str().unwrap().to_string();
    assert!(
        h.dir.path().join(&safety_name).is_file(),
        "{safety_name} was named but not written"
    );

    // The same running server answers from the file it just swapped in.
    assert_eq!(product_names(&h.app).await, vec!["Semoule 10kg"]);

    let safety = h.safety_copies();
    assert_eq!(safety.len(), 1, "{safety:?}");
    let mut kept = dzpos_core::db::open(&safety[0]).unwrap();
    let names: Vec<String> = dzpos_core::services::products::list(&mut kept, SHOP)
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    // The service lists by name, so the pair reads alphabetically; what
    // matters is that both are in the copy taken just before the swap.
    assert_eq!(names, vec!["Huile Elio 5L", "Semoule 10kg"]);

    // The safety copy sits beside the shop file, not among the thirty, and
    // the screen is told about it under its own name.
    let (_, listed) = call(&h.app, "GET", "/backups", None).await;
    assert_eq!(listed["backups"].as_array().unwrap().len(), 1);
    assert_eq!(listed["safety_copies"].as_array().unwrap().len(), 1);
    assert_eq!(listed["safety_copies"][0]["name"], json!(safety_name));
    assert!(listed["safety_copies"][0]["bytes"].as_i64().unwrap() > 0);

    // A copy taken after it leaves it where it is: pruning counts the daily
    // folder, which the safety copy is not in.
    let (status, _) = call(&h.app, "POST", "/backups", None).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(h.safety_copies().len(), 1);
    assert!(h.dir.path().join(&safety_name).is_file());

    // The staged copy is gone: the rename moved it, it was not left beside
    // the shop file. (The `-wal` beside the shop file now is the restored
    // file's own, written when db::open put it back into WAL mode; the old
    // one going is what the product list above already proves.)
    assert!(!h.dir.path().join("t.db.restoring.tmp").exists());
    assert!(h.db().is_file());
}

#[tokio::test]
async fn a_name_that_is_not_one_this_app_wrote_is_refused_and_changes_nothing() {
    let h = harness();
    call(&h.app, "POST", "/products", Some(product("Semoule 10kg"))).await;
    let (_, made) = call(&h.app, "POST", "/backups", None).await;
    let good = made["name"].as_str().unwrap().to_string();

    for name in [
        "..%2F..%2Fetc%2Fpasswd",
        "..%2Fbackups%2Fdzpos-20260908-093000.sqlite",
        "%2Fetc%2Fpasswd",
        "dzpos-20260908-093000.sqlite.bak",
        "t.db",
        "dzpos-20261308-093000.sqlite",
    ] {
        let (status, body) = call(&h.app, "POST", &format!("/backups/{name}/restore"), None).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{name} -> {body}");
        assert_eq!(body["error"]["code"], "validation", "{name} -> {body}");
        assert!(
            !body["error"]["message"]
                .as_str()
                .unwrap()
                .contains(h.dir.path().to_str().unwrap()),
            "a path reached the wire: {body}"
        );
    }

    // A well-formed name with no copy behind it is refused the same way.
    let (status, body) = call(
        &h.app,
        "POST",
        "/backups/dzpos-20200101-000000.sqlite/restore",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");

    // Nothing was restored, nothing was copied aside.
    assert_eq!(product_names(&h.app).await, vec!["Semoule 10kg"]);
    assert!(h.safety_copies().is_empty());
    let (_, listed) = call(&h.app, "GET", "/backups", None).await;
    assert_eq!(listed["backups"][0]["name"], json!(good));
    assert_eq!(listed["safety_copies"], json!([]));
}

#[tokio::test]
async fn a_copy_a_newer_version_wrote_is_refused_and_the_live_file_is_untouched() {
    use diesel::prelude::*;

    let h = harness();
    call(&h.app, "POST", "/products", Some(product("Semoule 10kg"))).await;
    let (_, made) = call(&h.app, "POST", "/backups", None).await;
    let name = made["name"].as_str().unwrap().to_string();

    let copy_path = h.dir.path().join("backups").join(&name);
    let mut copy = dzpos_core::db::open(&copy_path).unwrap();
    diesel::sql_query("INSERT INTO __diesel_schema_migrations (version) VALUES ('29990101000000')")
        .execute(&mut copy)
        .unwrap();
    drop(copy);

    call(&h.app, "POST", "/products", Some(product("Huile Elio 5L"))).await;
    let (status, body) = call(&h.app, "POST", &format!("/backups/{name}/restore"), None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"]["code"], "validation");

    // The refusal happened before anything was moved.
    assert_eq!(product_names(&h.app).await.len(), 2);
    assert!(h.safety_copies().is_empty());
}

#[tokio::test]
async fn the_backup_routes_need_the_token_and_refuse_other_methods() {
    let h = harness();
    for (method, uri) in [
        ("GET", "/backups"),
        ("POST", "/backups"),
        ("POST", "/backups/dzpos-20260908-093000.sqlite/restore"),
    ] {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let res = h.app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{method} {uri}");
    }
    for (method, uri) in [
        ("PUT", "/backups"),
        ("DELETE", "/backups"),
        ("GET", "/backups/dzpos-20260908-093000.sqlite/restore"),
    ] {
        let (status, body) = call(&h.app, method, uri, None).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} {uri}");
        assert_eq!(body["error"]["code"], "method_not_allowed");
    }
}

/// `dzpos-` + 8 digits + `-` + 6 digits + `.sqlite`, without pulling a regex
/// crate into the API's dev dependencies for one assertion.
fn regex_lite_matches(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("dzpos-") else {
        return false;
    };
    let Some(stamp) = rest.strip_suffix(".sqlite") else {
        return false;
    };
    let Some((day, time)) = stamp.split_once('-') else {
        return false;
    };
    day.len() == 8
        && time.len() == 6
        && day.chars().all(|c| c.is_ascii_digit())
        && time.chars().all(|c| c.is_ascii_digit())
}

/// The shop file is swapped in but cannot be reopened. The connection slot
/// is then empty, and an empty slot must refuse every caller: an in-memory
/// stand-in would be a working, empty database, and the next backup would
/// copy *that* over a real one and prune a good copy to make room.
#[tokio::test]
async fn a_shop_file_that_cannot_be_reopened_leaves_the_server_refusing_every_query() {
    use diesel::prelude::*;

    let h = harness();
    call(&h.app, "POST", "/products", Some(product("Semoule 10kg"))).await;
    let (_, made) = call(&h.app, "POST", "/backups", None).await;
    let name = made["name"].as_str().unwrap().to_string();
    let copy_path = h.dir.path().join("backups").join(&name);

    // A copy that passes every check `verify` makes and that `db::open`
    // still cannot open: its migration bookkeeping is gone, so opening it
    // replays the first migration onto tables that are already there.
    let mut copy = dzpos_core::db::open(&copy_path).unwrap();
    diesel::sql_query("DELETE FROM __diesel_schema_migrations")
        .execute(&mut copy)
        .unwrap();
    drop(copy);

    let (status, body) = call(&h.app, "POST", &format!("/backups/{name}/restore"), None).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    // Not "storage": the file on disk is fine and the one thing that helps
    // is relaunching, so the screen is told that and not something vaguer.
    assert_eq!(body["error"]["code"], "restart_needed");

    let before: Vec<std::path::PathBuf> = backup_files(&h);
    assert_eq!(before.len(), 1, "{before:?}");

    // Every route that needs the shop file now refuses, and goes on
    // refusing: nothing here reopens it behind the caller's back.
    for _ in 0..2 {
        let (status, body) = call(&h.app, "GET", "/products", None).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
        assert_eq!(body["error"]["code"], "restart_needed");
    }

    // The one that would have done damage: a backup taken off an empty
    // stand-in, with the prune that follows it evicting a real copy.
    let (status, body) = call(&h.app, "POST", "/backups", None).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(body["error"]["code"], "restart_needed");
    assert_eq!(
        backup_files(&h),
        before,
        "a copy was written, or an older one pruned, off a shop file that is not open"
    );

    // Listing reads the folder, not the shop file, so the owner can still
    // see what there is to restore from.
    let (status, listed) = call(&h.app, "GET", "/backups", None).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(listed["backups"].as_array().unwrap().len(), 1);
}

/// The copy is in place and a sidecar of the file it replaced could not be
/// removed. That old `-wal` belongs to a database that is gone, and SQLite
/// would replay it into the file that took its name, so nothing may reopen
/// the shop file until a person has looked at it.
#[cfg(unix)]
#[tokio::test]
async fn a_sidecar_left_behind_after_the_rename_stops_the_file_being_reopened() {
    let h = harness();
    call(&h.app, "POST", "/products", Some(product("Semoule 10kg"))).await;
    let (_, made) = call(&h.app, "POST", "/backups", None).await;
    let name = made["name"].as_str().unwrap().to_string();
    call(&h.app, "POST", "/products", Some(product("Huile Elio 5L"))).await;

    // A name the removal cannot delete, standing where the old file's `-shm`
    // sits. The connection already has its own handle open, so the shop file
    // goes on working until the restore closes it.
    let shm = h.dir.path().join("t.db-shm");
    let _ = std::fs::remove_file(&shm);
    std::fs::create_dir(&shm).unwrap();
    std::fs::write(shm.join("in the way"), b"x").unwrap();

    let (status, body) = call(&h.app, "POST", &format!("/backups/{name}/restore"), None).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(body["error"]["code"], "restart_needed");

    // Nothing reopened it behind the caller.
    let (status, body) = call(&h.app, "GET", "/products", None).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(body["error"]["code"], "restart_needed");

    // The rename did happen, so what is on disk is the copy, whole and not
    // written to since.
    std::fs::remove_dir_all(&shm).unwrap();
    let mut restored = dzpos_core::db::open(h.db()).unwrap();
    let names: Vec<String> = dzpos_core::services::products::list(&mut restored, SHOP)
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(
        names,
        vec!["Semoule 10kg"],
        "the copy is not what is on disk"
    );
}

/// Step 5 is the checkpoint, and a checkpoint SQLite refuses to take is
/// reported on a row it answers happily, not as an error. If that row is
/// dropped, the restore walks on and renames the copy over a shop file whose
/// write-ahead log still holds committed pages, and the log is then replayed
/// into the copy that took its name. So a busy checkpoint has to stop the
/// restore before the rename: the connection stays open, the shop file stays
/// where it is, and the till goes on working.
#[tokio::test]
async fn a_restore_stops_before_the_rename_when_the_log_cannot_be_folded_back() {
    use diesel::connection::SimpleConnection;

    let h = harness();
    call(&h.app, "POST", "/products", Some(product("Semoule 10kg"))).await;
    let (_, made) = call(&h.app, "POST", "/backups", None).await;
    let name = made["name"].as_str().unwrap().to_string();
    call(&h.app, "POST", "/products", Some(product("Huile Elio 5L"))).await;

    // A second till on the same file, mid-read. Nothing here is unusual: it
    // is what a second window, or a copy being verified, looks like from
    // SQLite's side.
    let mut reader = dzpos_core::db::open(h.db()).unwrap();
    reader.batch_execute("BEGIN").unwrap();
    dzpos_core::services::products::list(&mut reader, SHOP).unwrap();

    let (status, body) = call(&h.app, "POST", &format!("/backups/{name}/restore"), None).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(body["error"]["code"], "storage");

    // The connection was never closed, so the till answers as it did. (The
    // service lists by name, so the pair reads alphabetically.)
    assert_eq!(
        product_names(&h.app).await,
        vec!["Huile Elio 5L", "Semoule 10kg"],
        "the shop file was replaced by a restore that should not have got there"
    );

    // The copy staged for the rename that never came is gone. Left there it
    // would be a whole database beside the shop file that no screen lists
    // and no prune counts.
    assert_eq!(
        staged_copies(&h),
        Vec::<std::path::PathBuf>::new(),
        "a restore that was refused left its staged copy behind"
    );
    reader.batch_execute("COMMIT").unwrap();
}

/// The rename at step 7 does not happen and the file it would have replaced
/// cannot be opened either. Nothing was put back and this process no longer
/// serves the shop file, which is one answer, not two: `storage` alone would
/// send the screen looking for a full disk while every route 500s.
///
/// A folder standing where the shop file was is the only way to refuse the
/// rename from outside the code: the staged copy is written beside the shop
/// file, so a folder that refuses the rename would refuse the copy too, and
/// the permission and cross-device failures need root. The reopen that does
/// succeed is covered where it can be: `reopen_original` in `lib.rs`. Here
/// the file that still opens is the safety copy, which is what the owner has
/// left.
#[cfg(unix)]
#[tokio::test]
async fn a_rename_that_cannot_happen_answers_that_nothing_was_restored() {
    let h = harness();
    call(&h.app, "POST", "/products", Some(product("Semoule 10kg"))).await;
    let (_, made) = call(&h.app, "POST", "/backups", None).await;
    let name = made["name"].as_str().unwrap().to_string();

    // The live connection keeps its handle on the unlinked file, so the
    // safety copy and the checkpoint still run; only the rename onto the
    // name has nowhere to land.
    std::fs::remove_file(h.db()).unwrap();
    std::fs::create_dir(h.db()).unwrap();

    let (status, body) = call(&h.app, "POST", &format!("/backups/{name}/restore"), None).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(body["error"]["code"], "restore_failed_restart_needed");

    // The slot is empty and stays empty: nothing reopens a file that is not
    // there.
    let (status, body) = call(&h.app, "GET", "/products", None).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(body["error"]["code"], "restart_needed");

    assert!(
        h.db().is_dir(),
        "something wrote over the name the rename was refused on"
    );
    assert!(
        !h.dir.path().join("t.db.restoring.tmp").exists(),
        "the staged copy outlived the restore that was refused"
    );

    // What the owner has left is the copy taken on the way in, and it opens
    // and holds what the shop held.
    let copies = h.safety_copies();
    assert_eq!(copies.len(), 1, "{copies:?}");
    let mut safety = dzpos_core::db::open(&copies[0]).unwrap();
    let names: Vec<String> = dzpos_core::services::products::list(&mut safety, SHOP)
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, vec!["Semoule 10kg"]);
}
