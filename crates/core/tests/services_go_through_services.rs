// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! A service reaching past a sibling service into that sibling's repo skips
//! whatever the sibling decides on the way past: the clock it stamps, the
//! rule it enforces, the row it also writes. `stock.rs` did exactly that to
//! read the audit log, and the drift it read back was fine only because
//! nothing is decided about reading it yet.
//!
//! This is a burn-down list, the same idea as `scripts/file-sizes.json`:
//! every reach that exists today is written down, the test fails if a new
//! one appears, and the list gets shorter. It is
//! `architecture-fixes-without-a-domain-split` T4's work, and the test is
//! what makes it work rather than a survey.
//!
//! A source walk, the shape `crates/api/tests/route_gates.rs` and
//! `crates/core/tests/repos_own_the_queries.rs` already use. It keys off the
//! text after `repos::`, which catches a renamed import
//! (`use crate::repos::audit as audit_repo`) because the module's real name
//! is what sits there. Three ways past it are closed below by their own
//! assertion rather than left as a comment: a wildcard import, a service
//! going round `repos` to `crate::schema` and querying the table itself,
//! and a service that becomes a folder the walk does not descend into.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Repos with no service of their own. Reaching one is not skipping a
/// sibling, because there is no sibling: `counters` hands out the next
/// number in a series, `jobs` remembers when a background run last
/// happened, `sale_idempotency` remembers a retry key, and `testdb` only
/// exists under `cfg(test)`. None of them decides anything a caller could
/// route around. If one of them grows a service, it leaves this list and
/// joins the one below.
const NO_SERVICE_OWNS_THEM: [&str; 4] = ["counters", "jobs", "sale_idempotency", "testdb"];

/// What reaches past a sibling today, service by service. Seventeen of
/// them across eleven services. Shrinking, never growing: the fix for a
/// row is to add the missing function to the sibling's service and call
/// that, the way `stock.rs` now reads the audit log through
/// `services::audit::by_action`.
const REACHES_PAST_A_SIBLING: [(&str, &[&str]); 11] = [
    ("avoir", &["documents"]),
    ("cash", &["expenses"]),
    ("customers", &["documents"]),
    ("dashboard", &["debt", "supplier_debt"]),
    ("debt", &["customers", "documents"]),
    ("export", &["categories", "documents"]),
    ("import", &["categories", "products"]),
    ("products", &["categories"]),
    ("purchases", &["products", "supplier_debt"]),
    ("seed", &["categories"]),
    ("supplier_debt", &["purchases", "suppliers"]),
];

/// Every repo named in a file, whether by a `use crate::repos::{a, b as c}`
/// or by a `crate::repos::x::f()` at the call site. Both forms are written
/// in this crate, and a walk that saw only one of them would pass over half
/// the reaches it is looking for.
fn repos_named_in(source: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let stripped = code_of(source);
    let mut rest = stripped.as_str();
    while let Some(at) = rest.find("repos::") {
        rest = &rest[at + "repos::".len()..];
        if let Some(inner) = rest.strip_prefix('{') {
            let close = inner.find('}').unwrap_or(inner.len());
            for item in inner[..close].split(',') {
                if let Some(name) = first_ident(item.trim()) {
                    found.insert(name);
                }
            }
        } else if let Some(name) = first_ident(rest) {
            found.insert(name);
        }
    }
    found
}

/// The source with its line comments cut off, so prose about a repo is not
/// read as a call to one. Three doc comments name a repo today
/// (`audit.rs`, `dashboard.rs`, `purchases.rs`) and every one of them is
/// harmless by coincidence: two name their own file's repo and the third
/// names one on the no-service list. A fourth, in a service whose prose
/// happened to mention a sibling, would turn this test red over a sentence.
///
/// Cut at `//` unless it is the `//` of a `://`, so a URL in a comment does
/// not end the line early. A `//` inside a string literal would cut too
/// much, which loses a reach rather than inventing one; no service holds
/// such a string today, and the wider `crate::schema` check below catches
/// the shape that would matter.
fn code_of(source: &str) -> String {
    source
        .lines()
        .map(|line| match comment_at(line) {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

fn comment_at(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    (0..bytes.len().saturating_sub(1))
        .find(|&i| bytes[i] == b'/' && bytes[i + 1] == b'/' && (i == 0 || bytes[i - 1] != b':'))
}

/// The leading `a_snake_case` word, which is the repo's module name. Stops
/// at the `as` of a rename, at a `::`, and at anything that is not a word.
fn first_ident(text: &str) -> Option<String> {
    let word: String = text
        .chars()
        .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
        .collect();
    if word.is_empty() {
        None
    } else {
        Some(word)
    }
}

#[test]
fn no_service_reaches_a_repo_that_is_not_on_the_list() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services");
    let allowed: BTreeSet<&str> = NO_SERVICE_OWNS_THEM.into_iter().collect();

    let mut found: Vec<(String, Vec<String>)> = Vec::new();
    for entry in fs::read_dir(&dir).expect("crates/core/src/services is readable") {
        let path = entry.expect("a directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let own = path
            .file_stem()
            .and_then(|n| n.to_str())
            .expect("a file name")
            .to_string();
        let source = fs::read_to_string(&path).expect("a service file is readable");
        let reaches: Vec<String> = repos_named_in(&source)
            .into_iter()
            .filter(|repo| *repo != own && !allowed.contains(repo.as_str()))
            .collect();
        if !reaches.is_empty() {
            found.push((own, reaches));
        }
    }
    found.sort();

    let mut pinned: Vec<(String, Vec<String>)> = REACHES_PAST_A_SIBLING
        .iter()
        .map(|(service, repos)| {
            (
                (*service).to_string(),
                repos.iter().map(|r| (*r).to_string()).collect(),
            )
        })
        .collect();
    pinned.sort();

    assert_eq!(
        found, pinned,
        "the list of services reaching past a sibling into its repo has \
         changed. Shorter is the only direction it goes: add the function \
         the sibling's service is missing and call that, then take the row \
         out of REACHES_PAST_A_SIBLING. A longer list means a new reach, and \
         the sibling's rules were skipped on the way."
    );
}

/// The three ways round the walk above, each closed here rather than left
/// as a comment. None of them is in the tree today, which is what makes
/// them cheap to close: a rule written while nothing breaks it is a rule
/// nobody has to argue about.
///
/// One test and not three: they are one claim, that the walk cannot be
/// stepped around, and three names would say the same thing three times.
/// Each assertion carries its own sentence, so a failure still says which.
#[test]
fn the_walk_cannot_be_stepped_around() {
    // Querying `crate::schema` is not reaching past a sibling, it is going
    // round the whole repo layer, and the walk sees nothing because the
    // text `repos::` never appears. The same rule `repos_own_the_queries.rs`
    // holds for raw SQL, for the query builder.
    let schema = services_containing("crate::schema::");
    assert!(
        schema.is_empty(),
        "{schema:?} builds a query against crate::schema instead of calling \
         a repo. The walk above cannot see it, because it never writes \
         `repos::` at all."
    );

    // A wildcard puts every repo in scope under a bare name, and the walk
    // reads the `*` and finds no module to attribute.
    let wildcard = services_containing("repos::*");
    assert!(
        wildcard.is_empty(),
        "{wildcard:?} imports every repo at once. Name the ones it uses, or \
         the walk above cannot tell which."
    );

    // The walk reads one directory and does not descend.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services");
    let folders: Vec<String> = fs::read_dir(&dir)
        .expect("crates/core/src/services is readable")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.is_dir())
        .map(|path| stem(&path))
        .collect();
    assert!(
        folders.is_empty(),
        "{folders:?} is a service split into a folder, and the walk above \
         reads one directory deep. Teach it to descend before landing this."
    );
}

/// Where the services live, and every `.rs` in it. One walk, because two
/// hand-copies of "what counts as a service file" drift the day one of them
/// learns to skip something and the other does not.
fn service_files() -> Vec<std::path::PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services");
    let mut files: Vec<std::path::PathBuf> = fs::read_dir(&dir)
        .expect("crates/core/src/services is readable")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("rs"))
        .collect();
    files.sort();
    files
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|n| n.to_str())
        .expect("a file name")
        .to_string()
}

/// Every service file whose code mentions `marker`, comments excluded.
fn services_containing(marker: &str) -> Vec<String> {
    service_files()
        .into_iter()
        .filter(|path| {
            code_of(&fs::read_to_string(path).expect("a service file is readable")).contains(marker)
        })
        .map(|path| format!("{}.rs", stem(&path)))
        .collect()
}
