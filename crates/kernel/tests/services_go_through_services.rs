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
//! `crates/kernel/tests/repos_own_the_queries.rs` already use. It keys off the
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
use std::path::{Path, PathBuf};

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
    let allowed: BTreeSet<&str> = NO_SERVICE_OWNS_THEM.into_iter().collect();

    let mut found: Vec<(String, Vec<String>)> = Vec::new();
    for path in service_files() {
        let own = stem(&path);
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

/// The rings still standing: none, since 2026-09-21. It stays as an exact
/// list rather than becoming an `is_empty` so a failure still prints the ring
/// it found, written from its alphabetically first service, which is also the
/// only start the walk below reports it from.
///
/// The count went five, three, zero. The architecture review named two by
/// reading four files, `avoir -> documents` and `proforma -> sales`, and both
/// went in T3. A walk over every service found three more, all in the same
/// corner: a user is written with an audit row, the audit row names a user, a
/// session belongs to a user, and a preference is read while writing one.
/// T11 cut two edges and all three fell.
///
/// `audit -> users` was one call, `users::list`, for the names the log page
/// prints beside a `user_id`; it is `services::user_names` now, beside
/// `role_of` in `services/mod.rs`, which is above both and draws no edge
/// between them. `users -> sessions` was the three calls that end a person's
/// sessions when their credential changes or their fiche is switched off; the
/// `services::end_sessions_of` beside it holds them. The edge that stays is
/// the one the sign-in rule needs, `sessions -> users`, because believing a
/// PIN, counting a wrong try and locking a fiche out are settled in `users`
/// and a session is opened after that. Both are moves: the same function is
/// called with the same arguments at the same point in the same transaction,
/// so no audit row changed when it is written or what it says.
const RINGS_STILL_OPEN: [&str; 0] = [];

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
        "the list of services that import each other has changed, and it is \
         empty: no service in this crate imports a sibling that imports it \
         back, and nothing joins the list. Put what both need in a module \
         underneath them, or lift the one operation that needs both above \
         them — `services::role_of`, `services::user_names` and \
         `services::end_sessions_of` in `services/mod.rs` are what that \
         looks like. A ring here means neither of two services can now be \
         read or moved without the other."
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

    // The walk reads each services directory one level deep and does not
    // descend.
    let mut folders: Vec<String> = Vec::new();
    for dir in service_dirs() {
        folders.extend(
            fs::read_dir(&dir)
                .unwrap_or_else(|e| panic!("{} is readable: {e}", dir.display()))
                .map(|entry| entry.expect("a directory entry").path())
                .filter(|path| path.is_dir())
                .map(|path| stem(&path)),
        );
    }
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
/// `crates/retail/src` for that reason rather than the services beside it.
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
    let makers: Vec<String> = retail_source_files()
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

/// Every `.rs` under `crates/retail/src`, however deep. The walks above read
/// one directory each, because the rule they hold is about a service. This
/// one is about a `pub(crate)` constructor, which every module in that crate
/// can reach, so it has to see every module.
fn retail_source_files() -> Vec<std::path::PathBuf> {
    // `ProvedCustomer` and its `pub(crate)` constructor are both in
    // `dzpos-retail`: `pub(crate)` only reaches as far as that crate, and
    // this file lives in `crates/kernel/tests` (S6 of
    // `a-kernel-crate-and-retail-as-the-first-module`) rather than beside
    // the crate it reads, since the rule it proves is one the boundary walk
    // holds regardless of which crate's tests carry it.
    let mut found = Vec::new();
    let mut folders = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("../retail/src")];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).expect("a folder under crates/retail/src is readable") {
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
/// which module it means and not just which file name. Always with `/`: the
/// literals this file compares against are written that way, and on Windows
/// the OS hands back `models\customer.rs`, which is how the mirror's Windows
/// job failed `only_the_customers_service_makes_a_proved_customer` on
/// 2026-09-21 while Linux passed it.
fn under_src(path: &Path) -> String {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../retail/src");
    path.strip_prefix(&src)
        .unwrap_or(path)
        .to_str()
        .unwrap_or("a path")
        .replace('\\', "/")
}

/// Where the services live: two crates since the kernel crate split (S3 of
/// `a-kernel-crate-and-retail-as-the-first-module`). Cargo already refuses a
/// dependency cycle between crates, so a kernel-to-retail ring cannot exist
/// any more; what these two directories still let this file check is a ring
/// *within* one crate's services, which the compiler does not refuse on its
/// own.
fn service_dirs() -> Vec<std::path::PathBuf> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    vec![
        base.join("kernel/src/services"),
        base.join("retail/src/services"),
    ]
}

/// Every `.rs` in the two service directories. One walk, because two
/// hand-copies of "what counts as a service file" drift the day one of them
/// learns to skip something and the other does not.
fn service_files() -> Vec<std::path::PathBuf> {
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for dir in service_dirs() {
        files.extend(
            fs::read_dir(&dir)
                .unwrap_or_else(|e| panic!("{} is readable: {e}", dir.display()))
                .map(|entry| entry.expect("a directory entry").path())
                .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("rs")),
        );
    }
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

/// Every `.rs` under `crates/kernel/src`, however deep (whole-loop review,
/// 2026-09-24): a hand list here, `KERNEL_FILES`, used to name nineteen of
/// them, chosen the day the crate split for the reasons still worth keeping
/// below, and every file since added to the crate — `schema.rs` among
/// them, `build_info.rs`, `db.rs`, `lang.rs`, `log.rs`, five of `models/`'s
/// six files, two of `print/`'s three, and the whole of `repos/` — was
/// never read at all. A shop word landing in any of those was invisible to
/// this test by construction. The walk below reads every one instead, the
/// same shape as `retail_source_files` above for the crate beside this one;
/// what stays a choice is only which words are allowed to be there
/// (`SHOP_WORDS_ALLOWED`), never which files are looked at.
///
/// The eleven services, the five money files and the crate's one error enum
/// carry no shop meaning: no shop's own trade picks a different `users.rs`,
/// `error.rs` or `money/totals.rs`, the way every trade would still need a
/// facture reader picking a different `sales.rs`.
///
/// The print engine is mostly gone from the crate outright, since the
/// kernel crate split (S3 of `a-kernel-crate-and-retail-as-the-first-module`):
/// `mod.rs` and `escpos.rs` each name `models::document::Document` on a code
/// line (`mod.rs`'s own `pub fn number(doc: &Document)`), and `raster.rs`
/// in turn reaches into `print/ticket.rs` for `Align`, `Item` and `WIDTH`,
/// so neither of the two can live in a crate that depends on nothing; they
/// moved to `dzpos-retail` whole rather than only their shop half, a bigger
/// move than the S3 list was written for and reported as a deviation in
/// that commit rather than folded in quietly. `layout.rs` and `thermal.rs`
/// name no model at all and hold `FactureLayout` and `ThermalMode`, which
/// `services::preferences` (a kernel service) stores, so they stayed in
/// `dzpos_kernel::print`, and `print/strings.rs` beside them: its only
/// import is `crate::lang::Lang`, no shop type on a code line, and its row
/// below is what pins the fourteen shop words S4 split out.
fn kernel_source_files() -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut folders = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("../kernel/src")];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).expect("a folder under crates/kernel/src is readable") {
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

/// The word the shared kernel is not to name in its own code, chosen from
/// `docs/features.md`'s domain nouns and given rather than derived: this
/// walk cannot decide on its own what a shop word is, only whether one from
/// this list appears.
///
/// `product` (singular) is on this list beside `products` (plural), on a
/// second thought: `money/totals.rs`'s discount proration names its own
/// local `let product = i128::from(discount.as_centimes()) * …` for the
/// arithmetic product of two factors, nothing about a shop's stock, and
/// that hit is a false one. It is kept rather than tuned away by dropping
/// the word, because the word is what also catches `error.rs`'s
/// `UnpricedReversal { product_id: i32, … }`, a real hit, and a
/// `ProductRef` or a `Product` named in any *other* kernel file would be
/// exactly this kind of real hit too. `money/totals.rs`'s row below
/// carries `product` with the arithmetic reason spelled out beside it, so
/// the one false alarm this list knows about is accepted by name rather
/// than hidden by narrowing the list past the point where it still works.
///
/// `cash_refund` is the one two-word entry, matched when `cash` and
/// `refund` both appear somewhere in one identifier's own split, not only
/// when they land next to each other (see `shop_words_named_in`):
/// `cash_partial_refund` is as much a cash refund as `cash_refund` is, and
/// a pair check that only looked at neighbours would miss it. `cash` alone
/// is a payment method every kernel row can carry
/// (`models::sql_types::PaymentMethod::Cash`), and a list that fired on
/// the bare word would be wrong about a file that never mentions a
/// refund; the pair still has to be read off the same identifier; `cash`
/// in one binding and `refund` in an unrelated one three lines down does
/// not trip it.
///
/// `till`, `sell`, `price`, `discount` and `margin` joined the list on a
/// second pass, once `services/permissions.rs`'s own vocabulary was read
/// rather than guessed at: `Sell`, `ChangePriceAtTheTill`,
/// `OpenAndCloseTill`, `CloseAnotherPersonsTill`, `OverrideCreditBlock`,
/// `DiscountAboveThreshold` and `SeeCostAndMargin` are seven of its fifteen
/// permissions, and five of those seven split into a word now on this
/// list. `credit`, `cost` and `fiche` are deliberately still off it, though
/// `OverrideCreditBlock` and `SeeCostAndMargin` also carry them: a clinic's
/// permission table would keep a credit account, a cost line and a fiche
/// too, so none of the three marks a shop the way `till`, `sell`, a line
/// `price` or a `discount` and the `margin` a sale makes do.
///
/// Not on this list, though a shop word by any reading: `facture`,
/// `ticket`, `party`, `pricing`'s own near-miss `unpriced`. This walk
/// catches what the list names and nothing past it.
const SHOP_WORDS: [&str; 29] = [
    "product",
    "products",
    "sale",
    "sales",
    "document",
    "documents",
    "supplier",
    "suppliers",
    "stock",
    "purchase",
    "purchases",
    "customer",
    "customers",
    "avoir",
    "proforma",
    "pricing",
    "debt",
    "expense",
    "expenses",
    "category",
    "categories",
    "shift",
    "shifts",
    "barcode",
    "till",
    "sell",
    "price",
    "discount",
    "margin",
];

/// The shop already lived inside the kernel, in as many as eight of its
/// nineteen files (twelve of twenty-one before the kernel crate split of S3
/// of `a-kernel-crate-and-retail-as-the-first-module` moved `print/mod.rs`
/// and `print/escpos.rs` to `dzpos-retail` whole, taking their two rows with
/// them). Eight carried a word on the first pass, before `till`, `sell`,
/// `price`, `discount` and `margin` joined `SHOP_WORDS` above; seven of
/// those eight were already named by the
/// `whether-dinar-becomes-a-core-and-modules` research
/// (`context/research/module-shape/06-fork-per-trade.md:84-100`), which
/// walked `services::`, `repos::` and `models::` import paths into files a
/// split would delete, and `services/audit.rs` was the eighth: its
/// `ACTION_*` rows are string-tag constant names, never an import path, so
/// that survey had nothing to see. The second pass added four more, namely
/// `services/permissions.rs`, `services/settings.rs`, `money/mod.rs` and
/// `money/totals.rs`, and grew two of the first eight, `services/audit.rs`
/// and `print/strings.rs`, a word further. Each row below carries a
/// one-line reason of its own.
///
/// `money/mod.rs` and `money/totals.rs` stay by decision, not by default:
/// Samir, 2026-09-22, ruled the arithmetic is shared for every trade, so
/// their two rows exist to keep that decision visible rather than silent,
/// and each row's own comment carries the ruling.
///
/// A third pass put `product` back on `SHOP_WORDS`, no file joining or
/// leaving the list at the time: `error.rs`'s row (since removed, see
/// below) gained `product`, a real hit off `UnpricedReversal`'s own
/// `product_id` field, and `money/totals.rs`'s row gained it too, the one
/// false hit this list accepts by name rather than by narrowing the word
/// away (see `SHOP_WORDS`'s own doc comment).
///
/// S4 of `a-kernel-crate-and-retail-as-the-first-module` took two files off
/// this list entirely, the same milestone and for the same shape of reason
/// each time: the shop-named half of what the file carried moved to
/// `dzpos-retail`, and the row that pinned it here has nothing left to
/// catch.
///
/// `error.rs` left first: the eight variants that carried `barcode`, `debt`,
/// `document` and `product` (`DuplicateBarcode`, `PaymentAboveDebt`,
/// `CreditLimit`, `PartyIds`, `Unstamped`, `UnpricedReversal`, plus `Render`
/// and `Workbook`, which named no shop word but pinned `askama` and
/// `rust_xlsxwriter` on the kernel's own `Cargo.toml` for two variants only
/// retail's print and export code ever raised) left for
/// `dzpos_retail::error::RetailError`, which wraps `CoreError` rather than
/// repeating it. `crates/api/src/error.rs` maps both enums now, to the same
/// status, code and body every one of the eight always had.
///
/// `print/strings.rs` left second, in the same task: the twenty-four `Key`
/// variants that carried `avoir`, `barcode`, `customer`, `customers`,
/// `debt`, `discount`, `document`, `price`, `products`, `proforma`, `sale`,
/// `sales`, `stock` and `suppliers` (`Avoir`, `Proforma`, `ProformaNotice`,
/// `AvoirOnFacture`, `AvoirAmount`, `UnitPriceHt`, `UnitPrice`,
/// `AvoirInWords`, `ProformaInWords`, `ThisDocument`, `TotalDebt`,
/// `Document`, `KindSale`, `KindAvoir`, `InFavourOfCustomer`, `DebtSlip`,
/// `DebtInWords`, `SheetProducts`, `SheetSales`, `SheetCustomers`,
/// `SheetSuppliers`, `TemplateBarcodeNote`, `TemplateStockNote`, and
/// `Discount`) left for `dzpos_retail::print::strings::ShopKey`, which sits
/// beside a re-export of the kernel's own `Key` and `text` rather than
/// wrapping them, so every call site this crate already had for a shared
/// word (`Key::Total`, `Key::Tva`, and the rest) kept reading exactly as it
/// did; only the twenty-four call sites for a shop word were touched, to
/// `ShopKey::X` and `shop_text`. `SheetAllowedValues`, `TemplateUnits` and
/// `TemplateRates` stayed: no word on `SHOP_WORDS` sits in any of the three,
/// even though all three are read only from `dzpos_retail::services::
/// import`, and moving them anyway would have been tidying past the rule
/// this walk actually enforces.
///
/// Each row is the *set* of words that file names today, not a count: a
/// listed file that stops naming a word its row still carries would leave a
/// stale reason sitting above with nothing to catch it, so `assert_eq!`
/// below is exact both ways, the way `no_service_reaches_a_repo_that_is_not_on_the_list`
/// is for `REACHES_PAST_A_SIBLING`.
const SHOP_WORDS_ALLOWED: [(&str, &[&str]); 4] = [
    // Every `diesel::table!` name, hand-written to match the migrations
    // (schema.rs's own header), one file for the one SQLite file a shop
    // keeps: 21 of its 31 blocks are shop tables (`categories` opens it).
    // That is not the kernel choosing to know about the shop, it is what a
    // single migration list for both halves requires structurally, moved
    // here from a hand-written exemption to the file list itself (the
    // walk above used to skip this file rather than name what it finds)
    // when this test started reading every kernel file instead of
    // nineteen of them (whole-loop review, 2026-09-24).
    (
        "schema.rs",
        &[
            "avoir",
            "barcode",
            "categories",
            "category",
            "customer",
            "customers",
            "debt",
            "discount",
            "document",
            "documents",
            "expense",
            "expenses",
            "price",
            "product",
            "products",
            "purchase",
            "purchases",
            "sale",
            "shifts",
            "stock",
            "supplier",
            "suppliers",
        ],
    ),
    // The MoneyError variants a line discount or a unit price can raise
    // (NegativeUnitPrice, NegativeDiscount, LineDiscountAboveLine,
    // GlobalDiscountAboveTotal). Samir, 2026-09-22: the money arithmetic is
    // shared for every trade a shop rings up; a trade that never discounts
    // simply does not call this part of it, the same reason every trade
    // still shares the code that adds two prices together.
    ("money/mod.rs", &["discount", "price"]),
    // discount/price as money/mod.rs above (unit_price, line_discount,
    // global_discount, spread_discount); product is the one accepted
    // false hit this list knows about, its own local `let product = …`
    // for the arithmetic product of two factors in the discount
    // proration, no shop concept at all (see SHOP_WORDS). Samir,
    // 2026-09-22: the money arithmetic is shared for every trade a shop
    // rings up; a trade that never discounts simply does not call this
    // part of it.
    ("money/totals.rs", &["discount", "price", "product"]),
    // Five of the fifteen Permission variants split into a listed word:
    // Sell, OpenAndCloseTill/CloseAnotherPersonsTill/ChangePriceAtTheTill
    // (till, twice over), ChangePriceAtTheTill (price again),
    // DiscountAboveThreshold, SeeCostAndMargin. Samir ruled twice, on
    // 2026-09-21 and again on 2026-09-22 confirming it: the permission list
    // stays one list for the whole product, so the ten shop variants stay in
    // the kernel by that decision, not because nobody looked.
    (
        "services/permissions.rs",
        &["discount", "margin", "price", "sell", "till"],
    ),
];

/// `code_of`'s comment strip, with a kernel file's own `#[cfg(test)]`
/// module cut first and every string literal's content dropped after. Every
/// kernel file this walk reads has at most one `#[cfg(test)] mod tests {`,
/// opened once and running to the file's end (checked 2026-09-24, against
/// every file `kernel_source_files` now reads, not only the nineteen the
/// walk used to), so cutting at the first occurrence is exact rather than a
/// guess: a fixture a test builds for itself, such as a throwaway
/// `let products = 3;`, is not the kernel naming the shop, it is a test
/// naming what it is testing against.
///
/// A string is walked a byte at a time with its own escape handled
/// (`\"` does not close it), because `services/permissions.rs`'s
/// `Permission::Sell => "sell",` line pairs a variant name that already
/// spells the word with a string literal spelling it again, and counting
/// the string's content as code would put that file's row here for the
/// wrong reason: its real reason is the variant name beside it, which
/// survives the strip because it is not inside quotes. No raw string
/// (`r"…"`, `r#"…"#`) and no block comment (`/*…*/`) appears in any file
/// this walk reads (checked 2026-09-24 the same way, same wider set), so
/// neither is handled; a file that grew one would need this taught to read
/// it before its row above could be trusted again.
fn shop_facing_code_of(source: &str) -> String {
    let before_test_module = match source.find("#[cfg(test)]") {
        Some(at) => &source[..at],
        None => source,
    };
    let without_comments = code_of(before_test_module);
    let mut kept = String::new();
    let mut in_string = false;
    let mut chars = without_comments.chars();
    while let Some(c) = chars.next() {
        if in_string {
            if c == '\\' {
                chars.next();
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        if c == '"' {
            in_string = true;
            continue;
        }
        kept.push(c);
    }
    kept
}

/// The lowercase words inside one identifier, split at each underscore and
/// at each lowercase-to-uppercase step: `DuplicateBarcode` gives
/// `duplicate` and `barcode`; `ACTION_STOCK_DRIFT` gives `action`, `stock`
/// and `drift`; `product_id` gives `product` and `id`. This reads Rust's
/// own casing conventions, `SCREAMING_SNAKE_CASE` constants, `PascalCase`
/// types and variants, `snake_case` everything else, and not a general
/// English tokenizer, which this crate has no need of anywhere else.
fn words_of(ident: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut prev_lower = false;
    for c in ident.chars() {
        if c == '_' {
            if !current.is_empty() {
                words.push(current.to_lowercase());
                current.clear();
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower && !current.is_empty() {
            words.push(current.to_lowercase());
            current.clear();
        }
        prev_lower = c.is_lowercase();
        current.push(c);
    }
    if !current.is_empty() {
        words.push(current.to_lowercase());
    }
    words
}

/// Every maximal run of `[A-Za-z0-9_]` in `source`: wide enough to also
/// walk over a keyword or a type name, which costs nothing here because
/// neither is ever a `SHOP_WORDS` entry.
fn identifiers_of(source: &str) -> Vec<String> {
    let mut idents = Vec::new();
    let mut current = String::new();
    for c in source.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            current.push(c);
        } else if !current.is_empty() {
            idents.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        idents.push(current);
    }
    idents
}

/// Every `SHOP_WORDS` entry found as one whole word of an identifier in
/// `source`'s shipped code, comments, strings and the file's own test
/// module all cut first by `shop_facing_code_of`. Matched as a whole split
/// word and never a substring, so `PaymentMethod`'s `Cash` variant, split
/// to `["cash"]` alone, never trips `cash_refund` by itself: the pair is
/// checked on the same identifier's whole split, immediately below,
/// wherever the two land in it, so `cash_partial_refund` counts exactly as
/// `cash_refund` does. `cash` in one identifier and `refund` in an
/// unrelated one elsewhere in the same file does not: the check reads one
/// identifier's own `words` at a time and starts over for the next.
fn shop_words_named_in(source: &str) -> BTreeSet<String> {
    let code = shop_facing_code_of(source);
    let mut found = BTreeSet::new();
    for ident in identifiers_of(&code) {
        let words = words_of(&ident);
        for word in &words {
            if SHOP_WORDS.contains(&word.as_str()) {
                found.insert(word.clone());
            }
        }
        if words.iter().any(|w| w == "cash") && words.iter().any(|w| w == "refund") {
            found.insert("cash_refund".to_string());
        }
    }
    found
}

/// The rule this task draws as a rule and not a refactor: the shared
/// kernel does not name the shop. `SHOP_WORDS_ALLOWED` is where it does
/// today, and the assertion below is exact in both directions, the way the
/// reach list earlier in this file is: a new file joining the walk with a
/// shop word in it, or a listed file naming a word beyond its row, fails
/// the same as a listed file losing a word its row still claims. Only the
/// last of those three ever makes the file shorter, and shorter is the
/// only direction this list is meant to go.
#[test]
fn the_shared_kernel_does_not_name_the_shop() {
    // `crates/kernel/src` since the kernel crate split (S3 of
    // `a-kernel-crate-and-retail-as-the-first-module`).
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../kernel/src");
    let mut found: Vec<(String, Vec<String>)> = Vec::new();
    for path in kernel_source_files() {
        let rel = path
            .strip_prefix(&src)
            .unwrap_or(&path)
            .to_str()
            .unwrap_or("a path")
            .replace('\\', "/");
        let source = fs::read_to_string(&path).expect("a kernel file is readable");
        let words: Vec<String> = shop_words_named_in(&source).into_iter().collect();
        if !words.is_empty() {
            found.push((rel, words));
        }
    }
    found.sort();

    let mut pinned: Vec<(String, Vec<String>)> = SHOP_WORDS_ALLOWED
        .iter()
        .map(|(file, words)| {
            (
                (*file).to_string(),
                words.iter().map(|w| (*w).to_string()).collect(),
            )
        })
        .collect();
    pinned.sort();

    assert_eq!(
        found, pinned,
        "the shared kernel names the shop in a file, or with a word, this \
         list does not carry. A new file means a shop concept moved into a \
         file every trade would share; a listed file with a word beyond \
         its row means that file grew a new shop concept. Add the missing \
         kernel function so the caller stops spelling the concept itself, \
         and take the word back out; or, if the kernel genuinely needs to \
         carry it, add the word to the file's row and say why above. A \
         word dropped from a listed file and not from SHOP_WORDS_ALLOWED \
         fails too, on purpose: the list only shrinks, and a row nobody \
         shortens when the code no longer earns it is a reason nobody \
         re-reads."
    );
}

/// The mutation proof `services_containing` and its family lean on
/// implicitly, made explicit: the scanner sees a planted shop word in each
/// of Rust's three casings, does not see one hidden in a comment or a
/// string, and the `cash_refund` pair fires wherever the two land in one
/// identifier and never across two. Eight claims and one test, because
/// they are one claim, that the walk works on a string built here rather
/// than only on the files it happens to be pointed at today, and eight
/// names would say the same thing eight times.
#[test]
fn the_word_walk_sees_a_planted_shop_word() {
    let planted = "fn f() {\n    let products_total = 1;\n}\n";
    assert_eq!(
        shop_words_named_in(planted),
        BTreeSet::from(["products".to_string()]),
        "a bare identifier carrying a shop word must be seen"
    );

    let commented = "// products\nfn f() {}\n";
    assert!(
        shop_words_named_in(commented).is_empty(),
        "a shop word inside a comment must not be seen"
    );

    let stringed = "fn f() {\n    let s = \"products\";\n}\n";
    assert!(
        shop_words_named_in(stringed).is_empty(),
        "a shop word only inside a string literal must not be seen"
    );

    let paired = "fn f() {\n    let cash_refund_amount = 1;\n}\n";
    assert!(
        shop_words_named_in(paired).contains("cash_refund"),
        "the cash/refund pair must be seen when the two sit next to each \
         other in one identifier"
    );

    let apart_in_one = "fn f() {\n    let cash_partial_refund = 1;\n}\n";
    assert!(
        shop_words_named_in(apart_in_one).contains("cash_refund"),
        "the cash/refund pair must be seen when the two sit in the same \
         identifier with a word between them, not only when adjacent"
    );

    let unpaired = "fn f() {\n    let cash = 1;\n    let refund = 2;\n}\n";
    assert!(
        !shop_words_named_in(unpaired).contains("cash_refund"),
        "cash and refund named apart, in two identifiers, must not trip \
         the pair"
    );

    let screaming_snake = "const ACTION_PRODUCTS: &str = \"products.count\";\n";
    assert_eq!(
        shop_words_named_in(screaming_snake),
        BTreeSet::from(["products".to_string()]),
        "a SCREAMING_SNAKE_CASE constant carrying a shop word must be seen"
    );

    let pascal_case = "struct ProductsTotal {\n    total: i64,\n}\n";
    assert_eq!(
        shop_words_named_in(pascal_case),
        BTreeSet::from(["products".to_string()]),
        "a PascalCase type name carrying a shop word must be seen"
    );
}
