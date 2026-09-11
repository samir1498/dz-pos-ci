// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The test that matters for M5 T3 (`crates/core/src/services/support_bundle.rs`'s
//! own doc names the same list). A shop seeded with names, prices,
//! identifiers and documents a real one would carry is the fixture; every
//! assertion below either names the exact shape a file inside the zip may
//! take, or asks the seeded shop itself what a leak would look like and
//! looks for it in the zip's raw bytes and in every entry's decompressed
//! text, entry names included. The day somebody adds "just the sales table,
//! it helps with debugging" under a seventh entry name, the exact-set
//! assertion below refuses it before the leak scan even runs.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Nullable, Text};
use dzpos_core::services::{customers, products, seed, shops, suppliers};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

mod common;

const SHOP: i32 = 1;
const OWNER: i32 = 1;
const TOKEN: &str = "test-launch-token";

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).unwrap()
}

/// The day the seeded month of trading ends on. Fixed, never the wall
/// clock, the same reason `crates/core/tests/seed_service.rs`'s own `today`
/// is: a bundle built from a file seeded "today" would read different
/// figures every day this test ran.
fn seed_day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()
}

struct Harness {
    dir: tempfile::TempDir,
    app: axum::Router,
}

impl Harness {
    fn db(&self) -> std::path::PathBuf {
        self.dir.path().join("t.db")
    }
}

/// A shop with a catalogue, fiches and a month of trading behind it,
/// written through the real seeder (`just seed`'s own service) so the names,
/// the identifiers and the amounts are the ones a real shop would show a
/// support bundle it could not diagnose from.
fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    // The seeder sets the owner's first PIN and password
    // (`seed::the_way_in`), and `services::users::set_pin`/`set_password`
    // end every session on the fiche they touch (a credential change must
    // not leave an old session standing). Seeding runs first, against the
    // shop and owner row the first migration already wrote (the same ones
    // `common::sign_in` would otherwise make), and the session this test
    // signs in with is opened only after, so it is never one of the
    // sessions seeding itself just ended.
    {
        let mut conn = dzpos_core::db::open(&path).unwrap();
        seed::run(&mut conn, SHOP, OWNER, seed_day()).unwrap();
    }
    common::sign_in(&path, SHOP);
    let state = dzpos_api::AppState::open(&path, SHOP).unwrap();
    let app = dzpos_api::router(state, &token());
    Harness { dir, app }
}

fn auth(builder: axum::http::request::Builder) -> axum::http::request::Builder {
    builder
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, common::OWNER_SESSION)
}

async fn get_zip(app: &axum::Router, uri: &str) -> (StatusCode, Option<String>, Vec<u8>) {
    let req = auth(Request::builder().method("GET").uri(uri))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let disposition = res
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let bytes = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, disposition, bytes)
}

async fn post_json(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = auth(Request::builder().method("POST").uri(uri))
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

/// Every entry, decompressed, by name.
fn unzip(bytes: &[u8]) -> BTreeMap<String, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).unwrap();
    let mut out = BTreeMap::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().to_string();
        let mut text = String::new();
        file.read_to_string(&mut text)
            .unwrap_or_else(|e| panic!("{name} is not UTF-8 text: {e}"));
        out.insert(name, text);
    }
    out
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

/// Fails the test if `needle` shows up anywhere: in the zip's raw bytes
/// (compressed content and entry names alike, the literal ask), or in any
/// entry's decompressed text, or as an entry name itself. Blank needles are
/// skipped: an unset optional field is not a leak of the empty string.
fn assert_absent(raw: &[u8], files: &BTreeMap<String, String>, label: &str, needle: &str) {
    if needle.trim().is_empty() {
        return;
    }
    assert!(
        !contains_bytes(raw, needle.as_bytes()),
        "{label} ({needle:?}) appears in the support bundle's raw bytes"
    );
    for (name, text) in files {
        assert!(
            !text.contains(needle),
            "{label} ({needle:?}) appears inside {name}"
        );
        assert!(
            !name.contains(needle),
            "{label} ({needle:?}) appears as an entry name ({name})"
        );
    }
}

/// Digits, dashes and underscores only: whatever exact shape diesel gives a
/// migration version, a name or a sentence never passes this.
fn is_version_stamp(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|c| c.is_ascii_digit() || c == '-' || c == '_')
}

/// `table(col TYPE, col TYPE, ...)`: a lower-case table name, then
/// parenthesised `name TYPE` pairs. Never anything that looks like a row of
/// data, which is what this shape is checked for.
fn is_table_shape_line(line: &str) -> bool {
    let Some(open) = line.find('(') else {
        return false;
    };
    if !line.ends_with(')') {
        return false;
    }
    let table = &line[..open];
    if table.is_empty() || !table.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
        return false;
    }
    let inside = &line[open + 1..line.len() - 1];
    // A `col name` pair may itself carry parentheses (SQLite's own
    // `VARCHAR(50)`), so the split is on the first space of each pair, not
    // on every space, and the two-part check above already only found the
    // outer parentheses because `find` takes the first `(` in the whole
    // line.
    split_columns(inside).into_iter().all(|col| {
        let Some((name, ty)) = col.split_once(' ') else {
            return false;
        };
        !name.is_empty()
            && name.chars().all(|c| c.is_ascii_lowercase() || c == '_')
            && !ty.is_empty()
            && ty
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '(' || c == ')')
    })
}

/// `inside` split on `", "`, except a `", "` inside a type's own
/// parentheses (`VARCHAR(50)`) does not start a new column.
fn split_columns(inside: &str) -> Vec<&str> {
    let mut cols = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let bytes = inside.as_bytes();
    let mut i = 0;
    while i < inside.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth == 0 && inside[i..].starts_with(", ") => {
                cols.push(&inside[start..i]);
                start = i + 2;
            }
            _ => {}
        }
        i += 1;
    }
    cols.push(&inside[start..]);
    cols
}

#[derive(QueryableByName)]
struct UserRow {
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    pin_hash: String,
    #[diesel(sql_type = Nullable<Text>)]
    password_hash: Option<String>,
}

#[derive(QueryableByName)]
struct DocumentRow {
    #[diesel(sql_type = Nullable<Text>)]
    buyer_name: Option<String>,
    #[diesel(sql_type = BigInt)]
    total_ttc_centimes: i64,
    #[diesel(sql_type = BigInt)]
    net_to_pay_centimes: i64,
}

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = BigInt)]
    n: i64,
}

fn count(conn: &mut SqliteConnection, table: &str) -> i64 {
    let row: Count = diesel::sql_query(format!("SELECT COUNT(*) AS n FROM {table}"))
        .get_result(conn)
        .unwrap();
    row.n
}

/// Every amount's decimal spelling, dot and comma both: `format!("{}.{:02}")`
/// of the centimes is what `services::export`'s own doc says a spreadsheet
/// cell gets, and it is what a leaked total would look like as text, not
/// the raw centimes (`6000` collides with a byte count on sight).
fn money_spellings(centimes: i64) -> [String; 2] {
    let whole = centimes / 100;
    let cents = centimes % 100;
    [format!("{whole}.{cents:02}"), format!("{whole},{cents:02}")]
}

#[tokio::test]
async fn the_bundle_holds_exactly_six_files_shaped_the_way_they_should_be_and_none_of_the_shops_own_data(
) {
    let h = harness();

    // A real backup on disk, so `counts.txt`'s own backup fields are
    // exercised against something other than zero, and the copy's name
    // (which carries the shop file's own stem) is something to scan for.
    let (backup_status, backup) = post_json(&h.app, "/backups").await;
    assert_eq!(backup_status, StatusCode::CREATED, "{backup}");
    let backup_name = backup["name"].as_str().unwrap().to_string();
    let backup_taken_at = backup["taken_at"].as_str().unwrap().to_string();

    let size_before = std::fs::metadata(h.db()).unwrap().len();
    let (status, disposition, raw) = get_zip(&h.app, "/support-bundle").await;
    let size_after = std::fs::metadata(h.db()).unwrap().len();
    assert_eq!(status, StatusCode::OK);
    let disposition = disposition.unwrap();
    assert!(disposition.starts_with("attachment;"), "{disposition}");
    assert!(disposition.ends_with(".zip\""), "{disposition}");
    assert!(
        raw.starts_with(&[0x50, 0x4B, 0x03, 0x04]),
        "the body does not start like a zip"
    );

    let files = unzip(&raw);
    let names: BTreeSet<&str> = files.keys().map(String::as_str).collect();
    // Spelled out here rather than read from `ENTRIES`: taking the list from
    // the constant the writer itself walks would pass for a seventh file as
    // readily as for six, and the promise this test holds is the list, not
    // the agreement between two readings of it.
    let expected: BTreeSet<&str> = BTreeSet::from([
        "README.txt",
        "log.txt",
        "migrations.txt",
        "schema.txt",
        "counts.txt",
        "system.txt",
    ]);
    assert_eq!(
        names, expected,
        "the archive does not carry exactly the six named files"
    );

    let mut conn = dzpos_core::db::open(h.db()).unwrap();

    // README.txt: the build's own header line, verbatim, first.
    let header = dzpos_core::build_info::header_line(&dzpos_core::build_info::BUILD_INFO);
    let readme = &files["README.txt"];
    assert_eq!(readme.lines().next(), Some(header.as_str()), "{readme}");

    // log.txt: the same session log `AppState::open` just headed.
    let log = &files["log.txt"];
    assert!(log.contains(&header), "{log}");
    assert!(log.contains("session started"), "{log}");

    // migrations.txt: "applied:", the file's own applied versions, then
    // "shipped:", this build's own, and nothing else on any line.
    let applied = dzpos_core::db::applied_versions(&mut conn).unwrap();
    let shipped = dzpos_core::db::embedded_versions().unwrap();
    assert!(!applied.is_empty());
    assert!(!shipped.is_empty());
    let migrations = &files["migrations.txt"];
    let mut lines = migrations.lines();
    assert_eq!(lines.next(), Some("applied:"));
    for v in &applied {
        assert_eq!(lines.next(), Some(v.as_str()));
    }
    assert_eq!(lines.next(), Some("shipped:"));
    for v in &shipped {
        assert_eq!(lines.next(), Some(v.as_str()));
    }
    assert_eq!(lines.next(), None, "{migrations}");
    for line in migrations.lines() {
        assert!(
            line == "applied:" || line == "shipped:" || is_version_stamp(line),
            "{line:?} in migrations.txt is not a version stamp"
        );
    }

    // schema.txt: one shape line per table, every table named and no row.
    let schema = &files["schema.txt"];
    for table in ["products", "customers", "suppliers", "documents", "users"] {
        assert!(
            schema.lines().any(|l| l.starts_with(&format!("{table}("))),
            "{table} is missing from schema.txt: {schema}"
        );
    }
    for line in schema.lines() {
        assert!(
            is_table_shape_line(line),
            "{line:?} in schema.txt is not table(col TYPE, ...)"
        );
    }

    // counts.txt: the shop's real counts, in order, plus a shop file size
    // that only grew between the two stats taken around the call (the
    // session's own idle-time write lands before the handler's own read).
    let expected_products = count(&mut conn, "products");
    let expected_customers = count(&mut conn, "customers");
    let expected_documents = count(&mut conn, "documents");
    assert_eq!(expected_products, 60);
    assert_eq!(expected_customers, 12);
    assert!(expected_documents > 0);

    let counts = &files["counts.txt"];
    let mut fields: BTreeMap<&str, &str> = BTreeMap::new();
    for line in counts.lines() {
        let (key, value) = line.split_once(": ").expect("{line} is not key: value");
        fields.insert(key, value);
    }
    assert_eq!(fields.len(), 6, "{counts}");
    assert_eq!(fields["products"], expected_products.to_string());
    assert_eq!(fields["customers"], expected_customers.to_string());
    assert_eq!(fields["documents"], expected_documents.to_string());
    assert_eq!(fields["backups"], "1");
    assert_eq!(fields["backups_newest"], backup_taken_at);
    let reported_size: u64 = fields["shop_file_bytes"].parse().unwrap();
    assert!(
        reported_size >= size_before && reported_size <= size_after,
        "{reported_size} is outside [{size_before}, {size_after}]"
    );

    // system.txt: exactly three lines, in order, and the OS this test ran
    // on named honestly.
    let system = &files["system.txt"];
    let sys_lines: Vec<&str> = system.lines().collect();
    assert_eq!(sys_lines.len(), 3, "{system}");
    assert!(sys_lines[0].starts_with("os: "), "{system}");
    assert!(sys_lines[1].starts_with("language: "), "{system}");
    assert!(sys_lines[2].starts_with("timezone: "), "{system}");
    assert_eq!(sys_lines[0], format!("os: {}", std::env::consts::OS));

    // The leak scan. Every seeded name, identifier, hash and credential the
    // shop's own file holds, asked of the shop itself rather than
    // hardcoded, then looked for in the bundle's raw bytes and in every
    // entry's decompressed text.
    let mut forbidden: Vec<(&str, String)> = vec![
        ("shop name", seed::SHOP_NAME.to_string()),
        // The seeded PIN is not here and the password is. Neither reaches the
        // shop file in the clear, so what guards both is the hash below; four
        // digits, though, collide with any byte count the bundle happens to
        // print, which would fail this test for a leak that did not happen.
        ("owner password", seed::OWNER_PASSWORD.to_string()),
        ("launch token", TOKEN.to_string()),
        ("session token", common::OWNER_SESSION.to_string()),
        ("backup file name", backup_name),
        ("shop file's own folder", h.dir.path().display().to_string()),
    ];

    let shop = shops::get(&mut conn, SHOP).unwrap();
    for (label, value) in [
        ("shop rc", shop.rc),
        ("shop nif", shop.nif),
        ("shop nis", shop.nis),
        ("shop ai", shop.ai),
        ("shop address", shop.address),
        ("shop phone", shop.phone),
    ] {
        if let Some(v) = value {
            forbidden.push((label, v));
        }
    }

    for c in customers::list(&mut conn, SHOP, None).unwrap() {
        forbidden.push(("customer name", c.name));
        for (label, value) in [
            ("customer phone", c.phone),
            ("customer address", c.address),
            ("customer rc", c.rc),
            ("customer nif", c.nif),
            ("customer nis", c.nis),
            ("customer ai", c.ai),
        ] {
            if let Some(v) = value {
                forbidden.push((label, v));
            }
        }
    }

    for s in suppliers::list(&mut conn, SHOP, None).unwrap() {
        forbidden.push(("supplier name", s.name));
        for (label, value) in [
            ("supplier phone", s.phone),
            ("supplier address", s.address),
            ("supplier rc", s.rc),
            ("supplier nif", s.nif),
            ("supplier nis", s.nis),
            ("supplier ai", s.ai),
        ] {
            if let Some(v) = value {
                forbidden.push((label, v));
            }
        }
    }

    for p in products::list(&mut conn, SHOP).unwrap() {
        forbidden.push(("product name", p.name));
    }

    let users: Vec<UserRow> =
        diesel::sql_query("SELECT name, pin_hash, password_hash FROM users WHERE shop_id = ?")
            .bind::<diesel::sql_types::Integer, _>(SHOP)
            .load(&mut conn)
            .unwrap();
    for u in users {
        forbidden.push(("user name", u.name));
        forbidden.push(("user pin hash", u.pin_hash));
        if let Some(h) = u.password_hash {
            forbidden.push(("user password hash", h));
        }
    }

    let documents: Vec<DocumentRow> = diesel::sql_query(
        "SELECT buyer_name, total_ttc_centimes, net_to_pay_centimes FROM documents WHERE shop_id = ?",
    )
    .bind::<diesel::sql_types::Integer, _>(SHOP)
    .load(&mut conn)
    .unwrap();
    assert!(!documents.is_empty());
    for d in documents {
        if let Some(name) = d.buyer_name {
            forbidden.push(("document buyer name", name));
        }
        for centimes in [d.total_ttc_centimes, d.net_to_pay_centimes] {
            for spelling in money_spellings(centimes) {
                forbidden.push(("document amount", spelling));
            }
        }
    }

    for (label, needle) in &forbidden {
        assert_absent(&raw, &files, label, needle);
    }
}
