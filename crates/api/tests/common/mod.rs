// Every test binary compiles this whole file and calls part of it, so the
// helpers a given binary has no use for are dead code from where it stands.
#![allow(dead_code)]

//! What the tests in this directory need to name a document. These tests
//! issue through the till, which stamps a document with the shop clock, so
//! the year in a series and in a printed number is read rather than written
//! out: a literal `2026` would go red on 1 January and say nothing about the
//! rule it was pinning.

/// This moment's year on the shop's calendar (`services::clock`, UTC+1).
fn shop_year() -> String {
    dzpos_core::services::clock::now().format("%Y").to_string()
}

/// The counter key a document of this kind is numbered in right now:
/// `doc_facture:2026`. The series carries the year it counts in
/// (features.md §4, Numbering).
pub fn series_of(kind: &str) -> String {
    format!("{kind}:{}", shop_year())
}

/// The same year in the number a customer quotes: `FA-2026-000001`.
pub fn printed(prefix: &str, number: i64) -> String {
    format!("{prefix}-{}-{number:06}", shop_year())
}
// Signing the tests in this folder in.
//
// Every route but `/health` and the three auth ones now takes its actor from
// a session (M4 T2), so a test that used to show only the launch token gets a
// 401. Rather than make every `call` in every file take a token, the token is
// one constant this file owns: `sign_in` plants a session row for the shop's
// owner under it and each file's `call` shows it in the header the desktop
// uses.
//
// Planted rather than signed in through `POST /auth/login`, because a real
// sign-in hands back a random token and a shared constant is what lets a
// `call` helper stay the two arguments it already was. `tests/auth_api.rs` is
// where the real sign-in is exercised end to end.

use std::path::Path;

use diesel::prelude::*;

/// The session token every test in this folder signs its calls with. Not a
/// credential of any shop: it only ever reaches a temp file a test made.
pub const OWNER_SESSION: &str = "dz-pos-test-session-for-the-shops-owner";

/// The header the desktop shows a session token in, so a test spells it once.
pub const SESSION_HEADER: &str = "x-dzpos-session";

/// Opens a session for `shop`'s owner under `OWNER_SESSION`.
///
/// The shop and an owner are made if the file has none: a test that opens a
/// router on a second shop id to prove rule 3 wants that call to reach the
/// data layer and be answered 404, not to be turned away at the session gate
/// for a reason the test is not about.
pub fn sign_in(db: &Path, shop: i32) {
    let mut conn = dzpos_core::db::open(db).expect("the test's shop file will not open");

    diesel::sql_query("INSERT OR IGNORE INTO shops (id, name) VALUES (?, 'Test')")
        .bind::<diesel::sql_types::Integer, _>(shop)
        .execute(&mut conn)
        .expect("the test shop could not be made");

    #[derive(QueryableByName)]
    struct Id {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        id: i32,
    }
    let owner: Option<Id> = diesel::sql_query(
        "SELECT id FROM users WHERE shop_id = ? AND role = 'owner' AND active = 1 \
         ORDER BY id LIMIT 1",
    )
    .bind::<diesel::sql_types::Integer, _>(shop)
    .get_result(&mut conn)
    .optional()
    .expect("the users table would not answer");
    let owner = match owner {
        Some(row) => row.id,
        None => {
            let made: Id = diesel::sql_query(
                "INSERT INTO users (shop_id, name, role) VALUES (?, 'Propriétaire', 'owner') \
                 RETURNING id",
            )
            .bind::<diesel::sql_types::Integer, _>(shop)
            .get_result(&mut conn)
            .expect("the test owner could not be made");
            made.id
        }
    };

    diesel::sql_query(
        "INSERT OR REPLACE INTO sessions (shop_id, user_id, token_hash, created_at, last_seen_at) \
         VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind::<diesel::sql_types::Integer, _>(shop)
    .bind::<diesel::sql_types::Integer, _>(owner)
    .bind::<diesel::sql_types::Text, _>(dzpos_core::services::sessions::token_digest(OWNER_SESSION))
    .execute(&mut conn)
    .expect("the test session could not be opened");
}

/// The session token `tests/route_gates.rs` signs a cashier's calls with,
/// for the walk that proves a gated route refuses one (M4 T3).
pub const CASHIER_SESSION: &str = "dz-pos-test-session-for-a-cashier";
/// The same, for a manager, who the table's rows all currently answer for
/// like an owner.
pub const MANAGER_SESSION: &str = "dz-pos-test-session-for-a-manager";

/// Opens a session for one user of `role` ("cashier" or "manager") in
/// `shop`, under `token`. Same shape as `sign_in`, generalised to the role
/// the gate walk needs beside the owner every other file in this folder
/// signs in as; `sign_in` is left alone so the fifteen files already calling
/// it keep the owner they always got.
pub fn sign_in_as(db: &Path, shop: i32, role: &str, token: &str) {
    let mut conn = dzpos_core::db::open(db).expect("the test's shop file will not open");

    diesel::sql_query("INSERT OR IGNORE INTO shops (id, name) VALUES (?, 'Test')")
        .bind::<diesel::sql_types::Integer, _>(shop)
        .execute(&mut conn)
        .expect("the test shop could not be made");

    #[derive(QueryableByName)]
    struct Id {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        id: i32,
    }
    let user: Option<Id> = diesel::sql_query(
        "SELECT id FROM users WHERE shop_id = ? AND role = ? AND active = 1 \
         ORDER BY id LIMIT 1",
    )
    .bind::<diesel::sql_types::Integer, _>(shop)
    .bind::<diesel::sql_types::Text, _>(role)
    .get_result(&mut conn)
    .optional()
    .expect("the users table would not answer");
    let user_id = match user {
        Some(row) => row.id,
        None => {
            let made: Id = diesel::sql_query(
                "INSERT INTO users (shop_id, name, role) VALUES (?, ?, ?) RETURNING id",
            )
            .bind::<diesel::sql_types::Integer, _>(shop)
            .bind::<diesel::sql_types::Text, _>(format!("Test {role}"))
            .bind::<diesel::sql_types::Text, _>(role)
            .get_result(&mut conn)
            .expect("the test user could not be made");
            made.id
        }
    };

    diesel::sql_query(
        "INSERT OR REPLACE INTO sessions (shop_id, user_id, token_hash, created_at, last_seen_at) \
         VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind::<diesel::sql_types::Integer, _>(shop)
    .bind::<diesel::sql_types::Integer, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(dzpos_core::services::sessions::token_digest(token))
    .execute(&mut conn)
    .expect("the test session could not be opened");
}

/// A router on `db` answering for `shop`, with that shop's owner already
/// signed in under `OWNER_SESSION`. What a test uses to prove rule 3: a
/// second shop's router should answer 404 on this shop's rows, and it has to
/// get past the session gate to say so.
pub fn signed_in_router(db: &Path, shop: i32, token: &dzpos_api::LaunchToken) -> axum::Router {
    sign_in(db, shop);
    let state = dzpos_api::AppState::open(db, shop).expect("the test's shop file will not open");
    dzpos_api::router(state, token)
}

/// A second cashier's session, and the shop's floor manager's.
///
/// `sign_in_as` above takes the *first* active user of a role, so calling it
/// twice for "cashier" hands back the same person twice. A test about one
/// person reaching for another person's drawer needs two of the same role, so
/// this one always makes a fresh fiche under the name it is given and returns
/// the id it made.
pub const SECOND_CASHIER_SESSION: &str = "dz-pos-test-session-for-a-second-cashier";

/// Makes a user of `role` named `name` in `shop`, opens a session for them
/// under `token`, and answers their id.
pub fn sign_in_new(db: &Path, shop: i32, role: &str, name: &str, token: &str) -> i32 {
    let mut conn = dzpos_core::db::open(db).expect("the test's shop file will not open");

    diesel::sql_query("INSERT OR IGNORE INTO shops (id, name) VALUES (?, 'Test')")
        .bind::<diesel::sql_types::Integer, _>(shop)
        .execute(&mut conn)
        .expect("the test shop could not be made");

    #[derive(QueryableByName)]
    struct Id {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        id: i32,
    }
    let made: Id =
        diesel::sql_query("INSERT INTO users (shop_id, name, role) VALUES (?, ?, ?) RETURNING id")
            .bind::<diesel::sql_types::Integer, _>(shop)
            .bind::<diesel::sql_types::Text, _>(name)
            .bind::<diesel::sql_types::Text, _>(role)
            .get_result(&mut conn)
            .expect("the test user could not be made");

    diesel::sql_query(
        "INSERT OR REPLACE INTO sessions (shop_id, user_id, token_hash, created_at, last_seen_at) \
         VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind::<diesel::sql_types::Integer, _>(shop)
    .bind::<diesel::sql_types::Integer, _>(made.id)
    .bind::<diesel::sql_types::Text, _>(dzpos_core::services::sessions::token_digest(token))
    .execute(&mut conn)
    .expect("the test session could not be opened");
    made.id
}
