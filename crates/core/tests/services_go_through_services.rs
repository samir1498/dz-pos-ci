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
//! is what sits there. The same walk over `services::` says which services
//! import each other, which Rust compiles without a word.
//!
//! Five ways past both walks are closed below by their own assertion rather
//! than left as a comment: either wildcard import, a sibling named through
//! `super::` instead of `crate::services::`, a service going round `repos`
//! to `crate::schema` and querying the table itself, and a service that
//! becomes a folder the walk does not descend into.

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

/// What reaches past a sibling today, service by service. Three reaches
/// across two services; it was seventeen across eleven when the list was
/// written. Shrinking, never growing: the fix for a
/// row is to add the missing function to the sibling's service and call
/// that, the way `stock.rs` now reads the audit log through
/// `services::audit::by_action`.
///
/// The count below is of pairs and the array's length is of rows, and the
/// two are not the same number: a service reaching two repos is one row and
/// two reaches. The assertion compares the rows.
///
/// `purchases` left on 2026-09-21 with no ring to untangle first: the three
/// `repos::supplier_debt` calls saving an order paid at once made were the
/// ledger row, its settlement and the balance either side of it, and
/// `supplier_debt::hand_over` is now the one function that writes that row,
/// for `pay` and for the purchase alike.
///
/// `customers -> documents` and `debt -> documents` left together on
/// 2026-09-21, because one ring stood in front of both: `documents.rs`
/// imported `services::customers` for a single ownership check inside
/// `issue`, and `customers.rs` imports `services::debt`, so either service
/// routing its documents reads through `services::documents` closed a ring
/// the walk below refuses. The check did not move up to the callers, which
/// would have let a fourth caller of `issue` forget it; it became a type.
/// `NewDocument::customer` is a `ProvedCustomer`, whose only constructor is
/// `customers::prove`, so `issue` cannot be handed a customer nobody looked
/// up and no longer needs to look one up itself.
///
/// `debt -> customers` stays: four calls from `customers.rs` into
/// `services::debt`, one of them the ledger write in `debt::append`, so the
/// ring there is thick rather than an accident. Named by function and not by
/// line: the line was written as 129 and had drifted to 157 before anyone
/// looked.
const REACHES_PAST_A_SIBLING: [(&str, &[&str]); 2] = [
    ("debt", &["customers"]),
    ("supplier_debt", &["purchases", "suppliers"]),
];

/// Every repo named in a file, whether by a `use crate::repos::{a, b as c}`
/// or by a `crate::repos::x::f()` at the call site. Both forms are written
/// in this crate, and a walk that saw only one of them would pass over half
/// the reaches it is looking for.
fn repos_named_in(source: &str) -> BTreeSet<String> {
    modules_named_in(source, "repos::")
}

/// Every module named after `prefix` in a file, by a `use …{a, b}`, a
/// `use …a::{T, U}` or a `…a::f()` at the call site. One walk for the repos
/// and the services, because two hand-copies of it drift the day one learns
/// something the other does not: the nested brace below was read wrong by
/// both until 2026-09-20.
fn modules_named_in(source: &str, prefix: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let stripped = code_of(source);
    let mut rest = stripped.as_str();
    while let Some(at) = rest.find(prefix) {
        rest = &rest[at + prefix.len()..];
        if let Some(inner) = rest.strip_prefix('{') {
            for item in top_level_items(inner) {
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

/// The comma-separated items of a brace list, cut at the brace that closes
/// it rather than at the first `}` seen. `use crate::repos::{a::{X}, b}`
/// loses `b` to the naive read, and losing an item loses a reach.
fn top_level_items(inner: &str) -> Vec<&str> {
    let mut depth = 0usize;
    let mut items = Vec::new();
    let mut start = 0usize;
    for (at, c) in inner.char_indices() {
        match c {
            '{' => depth += 1,
            '}' if depth == 0 => {
                items.push(&inner[start..at]);
                return items;
            }
            '}' => depth -= 1,
            ',' if depth == 0 => {
                items.push(&inner[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    items.push(&inner[start..]);
    items
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

/// Every sibling service named in a file, by a `use crate::services::{a, b}`,
/// a `use crate::services::a::{T, U}` or a `crate::services::a::f()` at the
/// call site. The same walk as `repos_named_in` over a different prefix: one
/// text, `services::`, and whatever module name sits after it.
fn services_named_in(source: &str) -> BTreeSet<String> {
    modules_named_in(source, "services::")
}

/// The rings still standing, the same burn-down shape as the reach list
/// above. Each one is written from its alphabetically first service, which
/// is also the only start the walk below reports it from, so the same ring
/// always prints the same string.
///
/// All three are the same corner: a user is written with an audit row, the
/// audit row names a user, a session belongs to a user, and a preference is
/// read while writing one. Untangling that is its own task, on a plan that
/// can say what the shared kernel there is. The two the architecture review
/// named, `avoir -> documents` and `proforma -> sales`, are gone.
const RINGS_STILL_OPEN: [&str; 3] = [
    "audit -> users -> audit",
    "audit -> users -> sessions -> preferences -> audit",
    "sessions -> users -> sessions",
];

/// Two services that import each other cannot be read, moved or tested
/// apart: whichever one you open first assumes the other. Rust compiles an
/// intra-crate cycle without a word, so nothing said so while `documents`
/// and `avoir` imported each other and `sales` and `proforma` did the same.
///
/// The fix in both cases was a third module: the shared kernel comes out
/// underneath, or the operation that needs both goes above, and the edge
/// that remains points one way. This test is what keeps the edge from being
/// put back, and it looks for a ring of any length rather than only the
/// pair, because a three-hop ring is the same problem with one more file to
/// read before you find it.
#[test]
fn no_service_imports_a_sibling_that_imports_it_back() {
    let edges: Vec<(String, BTreeSet<String>)> = service_files()
        .into_iter()
        .map(|path| {
            let own = stem(&path);
            let source = fs::read_to_string(&path).expect("a service file is readable");
            let mut named = services_named_in(&source);
            named.remove(&own);
            (own, named)
        })
        .collect();

    // Every ring is enumerated from its alphabetically first service, and
    // the walk from that service never steps to one that sorts before it.
    // So each ring is found once, written one way, and found at all: a walk
    // that remembered which services it had already finished with would
    // miss the second ring through a shared service, which is how the
    // four-hop one here disappeared the moment `audit` was given a second
    // way back to itself.
    let mut rings: Vec<String> = Vec::new();
    for (start, _) in &edges {
        let mut path = vec![start.clone()];
        rings_through(start, start, &mut path, &edges, &mut rings);
    }
    rings.sort();
    rings.dedup();

    let pinned: Vec<String> = RINGS_STILL_OPEN.iter().map(|r| (*r).to_string()).collect();
    assert_eq!(
        rings, pinned,
        "the list of services that import each other has changed. Shorter is \
         the only direction it goes: put what both need in a module \
         underneath them, or lift the one operation that needs both above \
         them, then take the row out of RINGS_STILL_OPEN. A longer list \
         means neither of two services can now be read or moved without the \
         other."
    );
}

/// Every ring that closes back on `start`, walking forward from `node` and
/// never visiting a service that sorts before `start`. That one rule is
/// what makes the enumeration terminate, report each ring once, and write
/// it from the same end every time.
fn rings_through(
    start: &str,
    node: &str,
    path: &mut Vec<String>,
    edges: &[(String, BTreeSet<String>)],
    found: &mut Vec<String>,
) {
    let Some((_, out)) = edges.iter().find(|(name, _)| name == node) else {
        return;
    };
    for next in out {
        if next.as_str() < start {
            continue;
        }
        if next == start {
            let mut ring = path.clone();
            ring.push(start.to_string());
            found.push(ring.join(" -> "));
            continue;
        }
        if path.iter().any(|n| n == next) || !edges.iter().any(|(name, _)| name == next) {
            continue;
        }
        path.push(next.clone());
        rings_through(start, next, path, edges, found);
        path.pop();
    }
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

    // The same wildcard, aimed at the siblings rather than the repos:
    // `use crate::services::*` would put every service in scope under a
    // bare name and the ring walk would see no module to attribute.
    let all_siblings = services_containing("services::*");
    assert!(
        all_siblings.is_empty(),
        "{all_siblings:?} imports every service at once. Name the ones it \
         uses, or the ring walk above cannot tell which."
    );

    // `super::` is the other spelling of a sibling: every service file's
    // parent is `crate::services`, so `use super::sales` reaches one
    // without the text `services::` the walk keys off. Ten files write
    // `super::` today and every one of them is inside its own
    // `#[cfg(test)]` module, reaching back into itself, which is why this
    // asserts on the name after it rather than on the word.
    let siblings: BTreeSet<String> = service_files().iter().map(|path| stem(path)).collect();
    let by_super: Vec<String> = service_files()
        .into_iter()
        .filter(|path| {
            let own = stem(path);
            let source = code_of(&fs::read_to_string(path).expect("a service file is readable"));
            source.match_indices("super::").any(|(at, _)| {
                first_ident(&source[at + "super::".len()..])
                    .is_some_and(|name| name != own && siblings.contains(&name))
            })
        })
        .map(|path| format!("{}.rs", stem(&path)))
        .collect();
    assert!(
        by_super.is_empty(),
        "{by_super:?} names a sibling service through `super::`, which the \
         ring walk above does not read. Write `crate::services::` so the \
         edge is counted."
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

/// The one hole the compiler cannot close in the shape that took
/// `customers -> documents` and `debt -> documents` off the list above.
///
/// `NewDocument::customer` is a `ProvedCustomer`, so no caller of
/// `documents::issue` can hand it a customer id nobody looked up: that much
/// the type says, at the call, in the compiler's own words. What the type
/// cannot say is which module may make one. `ProvedCustomer` lives in
/// `models::customer` because `services::documents` has to name it without
/// importing `services::customers`, and Rust's `pub(in path)` only narrows
/// to an ancestor, so the constructor is `pub(crate)` and any module in the
/// crate could call it, a repo or a model as readily as a service.
///
/// This is what says only one does, and it reads the whole of
/// `crates/core/src` for that reason rather than the services beside it.
/// `customers::prove` reads the fiche under the shop filter first; a second
/// caller of `proved` would be a second meaning of the word proved, which is
/// the thing the shape exists to avoid.
///
/// It looks for `proved(` and not for `ProvedCustomer::proved`, because
/// `use crate::models::customer::ProvedCustomer as P;` and then `P::proved`
/// spells the call without the type's name appearing anywhere in the file.
/// The two names the assertion allows are the declaration and the one call.
#[test]
fn only_the_customers_service_makes_a_proved_customer() {
    let makers: Vec<String> = core_source_files()
        .into_iter()
        .filter(|path| {
            code_of(&fs::read_to_string(path).expect("a source file is readable"))
                .contains("proved(")
        })
        .map(|path| under_src(&path))
        .collect();
    assert_eq!(
        makers,
        vec![
            "models/customer.rs".to_string(),
            "services/customers.rs".to_string()
        ],
        "`proved` is the constructor of the proof `documents::issue` takes \
         instead of a customer id. `models/customer.rs` declares it and \
         `services::customers::prove` is the only thing that may call it, \
         because it is the only thing that reads the fiche under this shop's \
         filter first. Call `customers::prove`."
    );
}

/// Every `.rs` under `crates/core/src`, however deep. The walks above read
/// one directory each, because the rule they hold is about a service. This
/// one is about a `pub(crate)` constructor, which every module in the crate
/// can reach, so it has to see every module.
fn core_source_files() -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut folders = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).expect("a folder under crates/core/src is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                folders.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// A path as this file names one: `services/customers.rs`, so a failure says
/// which module it means and not just which file name.
fn under_src(path: &Path) -> String {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    path.strip_prefix(&src)
        .unwrap_or(path)
        .to_str()
        .unwrap_or("a path")
        .to_string()
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
