// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The walk over `gates::ROUTE_GATES` (M4 T2).
//!
//! The permission table in the core is one table a test reads rather than
//! restates; this is the same thing one layer up. The table says which
//! permission each mutating route wants, and this walks it against the
//! router's own route list in both directions, so neither can move without
//! the other.
//!
//! The router's list is read out of `crates/api/src/lib.rs`, because axum's
//! `Router` does not hand its routes back. The same shape as
//! `apps/desktop/src/theme.test.ts`, which greps the screens for a theme
//! branch: a source walk is what there is when the thing being checked is a
//! list a person maintains.
//!
//! What this does not do is check that a route refuses a cashier. That is T3,
//! which is one pass down this table; T2 ships the table, the actor and the
//! mechanism.

use dzpos_api::gates::{gate_for, Gate, ROUTE_GATES};

/// The router, as source. Read at compile time so this test cannot be run
/// against a file that is not the one that shipped.
const ROUTER_SOURCE: &str = include_str!("../src/lib.rs");

/// Every route the router declares, as (method, path).
///
/// `lib.rs` writes them as `.route("/path", get(..).post(..))` or
/// `.route("/path", post(..))`, sometimes over several lines, so the path is
/// taken off the `.route(` line and the methods off everything up to the
/// closing of that call.
fn declared_routes() -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = ROUTER_SOURCE[from..].find(".route(") {
        let open = from + at + ".route(".len();
        // The whole `.route(..)` call, found by counting parentheses from the
        // one that opened it. A span taken up to the next `.route(` instead
        // would swallow the call after it whenever a path is written on its
        // own line, and hand one route's methods to another.
        let Some(close) = closing_paren(ROUTER_SOURCE, open) else {
            break;
        };
        let call = &ROUTER_SOURCE[open..close];
        from = close;

        // The path is the first string literal in the call, whether it is on
        // the same line as `.route(` or on the next.
        let Some(start) = call.find('"') else {
            continue;
        };
        let Some(end) = call[start + 1..].find('"') else {
            continue;
        };
        let path = call[start + 1..start + 1 + end].to_owned();
        if !path.starts_with('/') {
            continue;
        }
        // What is left after the path names the methods the path takes.
        let handlers = &call[start + 1 + end..];
        for (needle, method) in [("get(", "GET"), ("post(", "POST"), ("put(", "PUT")] {
            if handlers.contains(needle) {
                found.push((method.to_owned(), path.clone()));
            }
        }
    }
    found
}

/// The index just past the `)` that closes the `(` this span opened at.
fn closing_paren(source: &str, open: usize) -> Option<usize> {
    let mut depth = 1usize;
    for (offset, c) in source[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// The router declares routes at all, and enough of them that a parser that
/// silently found nothing would fail here rather than passing every
/// assertion below by vacuum.
#[test]
fn the_router_source_is_readable_and_declares_the_routes_it_has() {
    let routes = declared_routes();
    assert!(
        routes.len() > 40,
        "only {} routes were read out of lib.rs; the parser has stopped working",
        routes.len()
    );
    for wanted in [
        ("GET", "/health"),
        ("POST", "/auth/login"),
        ("GET", "/auth/me"),
        ("POST", "/sales"),
        ("PUT", "/products/{id}"),
        ("GET", "/products"),
    ] {
        assert!(
            routes.contains(&(wanted.0.to_owned(), wanted.1.to_owned())),
            "{wanted:?} was not read out of lib.rs"
        );
    }
}

/// Every route that writes names a permission, or names why it wants none.
/// A new POST or PUT with no row in the table fails here, which is the whole
/// point of the table: a route that changes the shop's data without anybody
/// having decided who may is the thing M4 exists to stop.
#[test]
fn every_mutating_route_has_a_row_in_the_table() {
    let missing: Vec<(String, String)> = declared_routes()
        .into_iter()
        .filter(|(method, _)| method == "POST" || method == "PUT")
        .filter(|(method, path)| gate_for(method, path).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "these routes write and no row of gates::ROUTE_GATES says who may: {missing:?}"
    );
}

/// And the other way: a row naming a route that is not there is a ruling
/// applied to nothing, which is how a table quietly stops describing the app.
#[test]
fn every_row_of_the_table_names_a_route_that_is_there() {
    let routes = declared_routes();
    let orphans: Vec<&Gate> = ROUTE_GATES
        .iter()
        .filter(|g| !routes.contains(&(g.method.to_owned(), g.path.to_owned())))
        .collect();
    assert!(
        orphans.is_empty(),
        "these rows name a route the router does not have: {:?}",
        orphans
            .iter()
            .map(|g| (g.method, g.path))
            .collect::<Vec<_>>()
    );
}

/// A read is not in the table, and that is deliberate rather than an
/// oversight: what a cashier may see is T5's business on the screens, and
/// gating a GET is a separate ruling the plan has not taken for most of them.
/// The two reads the carry-ins do name (`GET /stock/recount` and the label
/// routes, both of which a cashier keeps) are open, so no row is wanted.
#[test]
fn the_table_is_about_writes_and_the_five_reads_that_carry_the_lists_out() {
    for gate in ROUTE_GATES {
        assert!(
            matches!(gate.method, "POST" | "PUT" | "GET"),
            "{} {} is a method this table does not carry",
            gate.method,
            gate.path
        );
    }
    // An ordinary read is open: a cashier is already looking at the list.
    for read in ["/products", "/stock/recount", "/dashboard", "/settings"] {
        assert!(
            gate_for("GET", read).is_none(),
            "GET {read} has a row; an ordinary read is not this table's business"
        );
    }
    // The exception the M3 carry-in named in words: a whole list on a USB
    // stick. If one of these loses its row, a cashier walks out with it.
    for read in [
        "/export/products",
        "/export/sales",
        "/export/customers",
        "/export/suppliers",
        "/import/products/template",
    ] {
        assert!(
            gate_for("GET", read).is_some(),
            "GET {read} carries a whole list out and has no row"
        );
    }
}
