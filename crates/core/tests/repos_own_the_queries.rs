// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Repos are the only place a query for data lives (architecture.md, the
//! layers section). Three services talk to SQLite directly anyway, none of
//! them about a shop's data, and this walk is what keeps the list at three.
//!
//! A source walk, the same shape as `crates/api/tests/route_gates.rs` and
//! `apps/desktop/src/theme.test.ts`: what is being checked is a rule a
//! person keeps, and there is nothing at runtime to ask.
//!
//! Two markers, because diesel offers two doors and they are not the same
//! door. `sql_query` asks a question and hands rows back. `batch_execute`
//! returns nothing and will run any statement at all, a `BEGIN` or an
//! `UPDATE` alike, so watching only the first would leave a service free to
//! write across every shop's rows unscoped and never trip this test. Both
//! are walked, each against its own list and its own reason.

use std::fs;
use std::path::Path;

/// Services that ask SQL a question. Both ask about the file rather than
/// anything in it: `backup.rs` reads `PRAGMA integrity_check` and the
/// migrations table, `support_bundle.rs` counts rows off `sqlite_master`
/// and `PRAGMA table_info` without knowing what any of them mean. Neither
/// has an aggregate to own the query or a `shop_id` to scope it by.
const ASKS_A_QUESTION: [&str; 2] = ["backup.rs", "support_bundle.rs"];

/// Services that hand SQLite a statement to run. `backup.rs` for `VACUUM
/// INTO` and `PRAGMA query_only`, neither of which diesel's query builder
/// can express; `pairing.rs` for `BEGIN IMMEDIATE`, because diesel's
/// `transaction()` cannot ask for the write lock up front and two phones
/// scanning one QR at the same moment need it to.
///
/// This is the list that matters most. A statement here can write, and it
/// writes outside every repo and every `shop_id` filter.
const RUNS_A_STATEMENT: [&str; 2] = ["backup.rs", "pairing.rs"];

/// A service split into a folder, which both walks in this file would read
/// past: they read one directory deep and keep only a `.rs`, so
/// `services/foo/mod.rs` is a service no rule here applies to. The sibling
/// gate in `services_go_through_services.rs` refuses the same shape for the
/// same reason, and this one matters more: what escapes here is a statement
/// that writes outside every repo and every `shop_id` filter.
fn service_folders() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services");
    let mut found: Vec<String> = fs::read_dir(&dir)
        .expect("crates/core/src/services is readable")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.is_dir())
        .map(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("a folder")
                .to_string()
        })
        .collect();
    found.sort();
    found
}

/// Every file in `src/services` whose source mentions `marker`.
fn services_mentioning(marker: &str) -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services");
    let mut found: Vec<String> = fs::read_dir(&dir)
        .expect("crates/core/src/services is readable")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("rs"))
        .filter(|path| {
            fs::read_to_string(path)
                .expect("a service file is readable")
                .contains(marker)
        })
        .map(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .expect("a file name")
                .to_string()
        })
        .collect();
    found.sort();
    found
}

fn sorted(names: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = names.iter().map(|s| (*s).to_string()).collect();
    out.sort();
    out
}

/// Both lists at once, because a service that moved from one door to the
/// other should read as one failure and not as two unrelated ones.
///
/// A file named in a list and missing from `src/services` fails here too,
/// without a test of its own: a renamed file stops turning up in the walk,
/// so the two sides stop matching.
#[test]
fn no_service_outside_the_lists_talks_to_sqlite_itself() {
    assert_eq!(
        services_mentioning("sql_query"),
        sorted(&ASKS_A_QUESTION),
        "a service outside {ASKS_A_QUESTION:?} runs its own query. Either it \
         belongs to an aggregate, and then it belongs in that aggregate's \
         repo, or it asks about the database file itself, and then \
         docs/architecture.md says so and this list grows by one."
    );

    let folders = service_folders();
    assert!(
        folders.is_empty(),
        "{folders:?} is a service split into a folder, and both walks in \
         this file read one directory deep. Descending is not enough on \
         its own: `services_mentioning` names what it found by the bare \
         file name, so every folder's `mod.rs` collapses to one key and \
         the failure cannot say which folder it means. Teach them to \
         descend and to keep the path before landing this, or a service's \
         raw SQL is a rule nothing here applies."
    );

    assert_eq!(
        services_mentioning("batch_execute"),
        sorted(&RUNS_A_STATEMENT),
        "a service outside {RUNS_A_STATEMENT:?} hands SQLite a statement. \
         `batch_execute` runs anything, an UPDATE as readily as a BEGIN, and \
         outside a repo there is no `shop_id` on it. Say in \
         docs/architecture.md what this one runs and why the query builder \
         cannot express it."
    );
}
