// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Handlers translate, they do not decide (`routes/mod.rs`). A permission is
//! a row in `gates.rs` that answers for a whole route, and one handler is
//! outside that because a row cannot say "this answer, without one field".
//!
//! `products.rs::redact_cost` blanks what a product cost the shop for a role
//! without `SeeCostAndMargin`. `GET /products` cannot be gated as a route at
//! all: the till needs the catalogue to ring a sale up, cashier included.
//!
//! A source walk, the same shape as `route_gates.rs` beside it. What is
//! being checked is a rule a person keeps, and a second redaction would mean
//! a second place to read before trusting what a role sees.

use std::fs;
use std::path::Path;

/// The handler files that may ask `can` at all, and why.
///
/// `products.rs` decides: it strips two fields out of an answer nobody can
/// be refused. `auth.rs` does not: `held_by` walks `Permission::ALL` to tell
/// a screen which buttons to grey out, refusing nothing and blanking
/// nothing, and the answer it builds is the one the gates would give anyway.
const MAY_ASK_CAN: [&str; 2] = ["auth.rs", "products.rs"];

/// A file named in `MAY_ASK_CAN` and missing from `src/routes` fails here
/// too, without a test of its own: a renamed file stops turning up in the
/// walk, so the two sides stop matching.
#[test]
fn no_handler_outside_the_two_routes_mod_names_reads_a_permission() {
    let routes = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes");
    let mut found: Vec<String> = Vec::new();

    for entry in fs::read_dir(&routes).expect("crates/api/src/routes is readable") {
        let path = entry.expect("a directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("a file name")
            .to_string();
        if name == "mod.rs" {
            continue;
        }
        let source = fs::read_to_string(&path).expect("a route file is readable");
        if source.contains("Permission::") {
            found.push(name);
        }
    }
    found.sort();

    let mut allowed: Vec<String> = MAY_ASK_CAN.iter().map(|s| s.to_string()).collect();
    allowed.sort();

    assert_eq!(
        found, allowed,
        "a handler outside {allowed:?} reads a permission. A route a role may \
         not call at all is a row in gates.rs; a field a role may not see \
         inside an answer everyone may call is the exception routes/mod.rs \
         describes, and taking it a second time means saying so there first."
    );
}
