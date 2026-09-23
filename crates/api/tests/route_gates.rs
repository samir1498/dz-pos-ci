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
//! The router's list is read out of `crates/api/src/router.rs`, because axum's
//! `Router` does not hand its routes back. The same shape as
//! `apps/desktop/src/theme.test.ts`, which greps the screens for a theme
//! branch: a source walk is what there is when the thing being checked is a
//! list a person maintains.
//!
//! T2 shipped the table, the actor and the mechanism with nothing applied.
//! `a_cashier_is_refused_and_a_manager_is_not_on_every_gated_route` below is
//! T3's pass down the table over real HTTP: it walks `ROUTE_GATES` the same
//! way the two tests above do, so a row added to the table is a case added
//! here rather than a sixty-sixth test written out by hand.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use dzpos_api::gates::{gate_for, Gate, ROUTE_GATES};
#[cfg(feature = "retail")]
use dzpos_core::services::permissions::Permission;
use dzpos_core::services::permissions::{can, Role};

mod common;

const SHOP: i32 = 1;
const TOKEN: &str = "test-launch-token";

fn token() -> dzpos_api::LaunchToken {
    dzpos_api::LaunchToken::from_secret(TOKEN).expect("a fixed test secret makes a token")
}

/// `gate.path` as the router wrote it (`/customers/{id}`), with every
/// `{placeholder}` segment stood in for by a value that will match the
/// route: the gate runs before the handler even looks at whether `1` names
/// anything, so what is there does not matter, only that the path matches.
fn concrete_path(template: &str) -> String {
    template
        .split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                "1"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

async fn call(app: &axum::Router, method: &str, uri: &str, session: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"))
        .header(common::SESSION_HEADER, session)
        .body(Body::empty())
        .expect("a GET or an empty body always builds a request");
    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("the router did not answer");
    let status = res.status();
    let bytes = res
        .into_body()
        .collect()
        .await
        .expect("the body did not read")
        .to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

/// The walk itself: every row of `ROUTE_GATES` that names a permission is
/// asked twice, once as a cashier and once as a manager, on the same shop.
/// What each is owed is read off `services::permissions::can` rather than
/// assumed, because one row (`POST /sales`) names `Permission::Sell`
/// precisely because every role holds it (the row's own `why`: "the one
/// thing every role may do"), so a table-driven test has to ask the same
/// table the gate itself asks rather than hard-code "a cashier is always
/// refused". For every row `can` refuses a cashier, and today that is every
/// row but `/sales`: a cashier gets 403 `forbidden` naming the row's own
/// permission. For every row `can` allows a role, that role never sees a
/// 403 here, nor a 401 (a 401 would mean `sign_in_as` silently failed to
/// seat that role, which would otherwise pass this branch by vacuum), though
/// it may see any other status the handler itself decides on the empty body
/// this test sends. Neither side sends a body: the gate
/// runs in `session::require`, before any handler reads one, so an empty
/// body is as good as a correct one for proving the gate fired first.
#[tokio::test]
async fn a_cashier_is_refused_and_a_manager_is_not_on_every_gated_route() {
    let dir = tempfile::tempdir().expect("no temp dir for the test's shop file");
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    common::sign_in_as(&path, SHOP, "cashier", common::CASHIER_SESSION);
    common::sign_in_as(&path, SHOP, "manager", common::MANAGER_SESSION);
    let state = dzpos_api::AppState::open(&path, SHOP).expect("the test's shop file will not open");
    let app = dzpos_api::router(state, &token());

    for gate in ROUTE_GATES {
        let Some(permission) = gate.permission else {
            continue;
        };
        let uri = concrete_path(gate.path);

        for (role, session) in [
            (Role::Cashier, common::CASHIER_SESSION),
            (Role::Manager, common::MANAGER_SESSION),
        ] {
            let (status, body) = call(&app, gate.method, &uri, session).await;
            if can(role, permission) {
                assert_ne!(
                    status,
                    StatusCode::FORBIDDEN,
                    "{} {} refused a {role:?}, who holds {permission}: {body}",
                    gate.method,
                    gate.path
                );
                // A missing or unresolved session also answers something
                // other than FORBIDDEN, which would let this branch pass
                // vacuously if `sign_in_as` ever failed to plant the role's
                // session row. A held permission is proved by getting past
                // the gate, not merely by not being refused by it.
                assert_ne!(
                    status,
                    StatusCode::UNAUTHORIZED,
                    "{} {} would not even sign a {role:?} in, who holds {permission}: {body}",
                    gate.method,
                    gate.path
                );
            } else {
                assert_eq!(
                    status,
                    StatusCode::FORBIDDEN,
                    "{} {} let a {role:?} through, who does not hold {permission}: {body}",
                    gate.method,
                    gate.path
                );
                assert_eq!(
                    body["error"]["code"].as_str(),
                    Some("forbidden"),
                    "{} {} refused a {role:?} for a different reason: {body}",
                    gate.method,
                    gate.path
                );
                assert_eq!(
                    body["error"]["permission"].as_str(),
                    Some(permission.as_str()),
                    "{} {} named a different permission than its own row: {body}",
                    gate.method,
                    gate.path
                );
            }
        }
    }
}

/// The router, as source. Read at compile time so this test cannot be run
/// against a file that is not the one that shipped.
const ROUTER_SOURCE: &str = include_str!("../src/router.rs");

/// The span of router.rs's one `#[cfg(feature = "retail")]` block, the same
/// one `the_shops_own_gate_rows_and_routes_carry_the_retail_feature` counts.
/// `declared_routes` needs it too (S7 of
/// `a-kernel-crate-and-retail-as-the-first-module`): a text walk knows
/// nothing about `#[cfg]`, so a `--no-default-features` build has to be told
/// by hand which span of the file its router does not actually have.
fn retail_block_span() -> (usize, usize) {
    let start_marker = "#[cfg(feature = \"retail\")]\n    let guarded = guarded";
    let start = ROUTER_SOURCE
        .find(start_marker)
        .expect("router.rs no longer gates a block of routes on the retail feature");
    let end_marker =
        "\n    let guarded = guarded\n        // Every route above takes its actor from the session";
    let end = ROUTER_SOURCE[start..]
        .find(end_marker)
        .map(|i| start + i)
        .expect("the shop's own routes no longer end where the session layer begins");
    (start, end)
}

/// The span of router.rs's `#[cfg(feature = "clinic")]` block (C3 of
/// `the-first-clinic-module-patients-queue-appointments`), the same idea as
/// `retail_block_span` above for the other module. It sits before the shop's
/// block and is one statement, so it ends at its own `;`: no `.route(` call
/// carries one inside it.
fn clinic_block_span() -> (usize, usize) {
    let start_marker = "#[cfg(feature = \"clinic\")]\n    let guarded = guarded";
    let start = ROUTER_SOURCE
        .find(start_marker)
        .expect("router.rs no longer gates a block of routes on the clinic feature");
    let end = ROUTER_SOURCE[start..]
        .find(';')
        .map(|i| start + i)
        .expect("the clinic's block of routes never ends");
    (start, end)
}

/// Every route the router declares, as (method, path). With a module's
/// feature off, that module's block is skipped: the router this test drives
/// really does not have those routes.
///
/// `router.rs` writes them as `.route("/path", get(..).post(..))` or
/// `.route("/path", post(..))`, sometimes over several lines, so the path is
/// taken off the `.route(` line and the methods off everything up to the
/// closing of that call.
fn declared_routes() -> Vec<(String, String)> {
    let (retail_start, retail_end) = retail_block_span();
    let (clinic_start, clinic_end) = clinic_block_span();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = ROUTER_SOURCE[from..].find(".route(") {
        let open = from + at + ".route(".len();
        if !cfg!(feature = "retail") && open >= retail_start && open < retail_end {
            from = open;
            continue;
        }
        if !cfg!(feature = "clinic") && open >= clinic_start && open < clinic_end {
            from = open;
            continue;
        }
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
    // S7 of `a-kernel-crate-and-retail-as-the-first-module`: with the
    // feature off, `declared_routes` also drops the sixty shop routes off
    // this count, so the floor a no-shop build clears is the kernel's own
    // rather than the two together.
    let floor = if cfg!(feature = "retail") { 40 } else { 20 };
    assert!(
        routes.len() > floor,
        "only {} routes were read out of router.rs; the parser has stopped working",
        routes.len()
    );
    let mut wanted = vec![
        ("GET", "/health"),
        ("POST", "/auth/login"),
        ("GET", "/auth/me"),
    ];
    if cfg!(feature = "retail") {
        wanted.extend([
            ("POST", "/sales"),
            ("PUT", "/products/{id}"),
            ("GET", "/products"),
        ]);
    }
    for wanted in wanted {
        assert!(
            routes.contains(&(wanted.0.to_owned(), wanted.1.to_owned())),
            "{wanted:?} was not read out of router.rs"
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
/// `/products` stays open on purpose even after T5: the till reads it to
/// ring a sale up, so cost is redacted at the field
/// (`routes/products.rs::redact_cost`) rather than the route refused.
/// `/stock/recount` and `/settings` are the same kind of ordinary read a
/// cashier is already looking at.
///
/// The closing review on 2026-09-11 moved five more out of "open" and into
/// the table, and they are the ones this comment's reasoning had missed:
/// `/expenses` and `/cash` hand back what the shop spends and what is in the
/// drawer, `/suppliers` and one supplier's ledger hand back what it owes for
/// goods, and `/backups` lists copies of the whole file. Each was open while
/// the screen that sums it was refused, which is the wrong way round.
///
/// Retail-only (S7 of `a-kernel-crate-and-retail-as-the-first-module`):
/// every whole-list-out read this walks but `/audit-log` and `/backups` is
/// a shop route, and neither of those two loses its row with the feature
/// off.
#[cfg(feature = "retail")]
#[test]
fn the_table_is_about_writes_and_the_reads_that_carry_lists_or_reports_out() {
    for gate in ROUTE_GATES {
        assert!(
            matches!(gate.method, "POST" | "PUT" | "GET"),
            "{} {} is a method this table does not carry",
            gate.method,
            gate.path
        );
    }
    // An ordinary read is open: a cashier is already looking at the list.
    for read in ["/products", "/stock/recount", "/settings"] {
        assert!(
            gate_for("GET", read).is_none(),
            "GET {read} has a row; an ordinary read is not this table's business"
        );
    }
    // The exception the M3 carry-in named in words, a whole list on a USB
    // stick, and the five the closing review added: money out, the cash
    // position, what the shop owes its suppliers, and the backups. If one of
    // these loses its row, a cashier reads it.
    for read in [
        "/export/products",
        "/export/sales",
        "/export/customers",
        "/export/suppliers",
        "/import/products/template",
        "/expenses",
        "/cash",
        "/suppliers",
        "/suppliers/{id}/ledger",
        "/backups",
    ] {
        assert!(
            gate_for("GET", read).is_some(),
            "GET {read} carries a whole list out and has no row"
        );
    }
    // And the audit log: not a whole list out the door, but the record of
    // every sensitive thing the staff have done, the owner's alone (M4 T7).
    assert_eq!(
        gate_for("GET", "/audit-log").and_then(|g| g.permission),
        Some(Permission::SeeAuditLog)
    );
    // Four more, from the M4 T5 review (2026-09-11): the dashboard and its
    // chart are the reports `SeeReports` names, and the purchase list and
    // its detail are nothing but what the shop pays its suppliers, so both
    // pairs are gated whole rather than redacted at a field.
    for read in ["/dashboard", "/dashboard/series"] {
        assert_eq!(
            gate_for("GET", read).and_then(|g| g.permission),
            Some(Permission::SeeReports),
            "GET {read} should carry SeeReports after the M4 T5 review"
        );
    }
    for read in ["/purchases", "/purchases/{id}"] {
        assert_eq!(
            gate_for("GET", read).and_then(|g| g.permission),
            Some(Permission::SeeCostAndMargin),
            "GET {read} should carry SeeCostAndMargin after the M4 T5 review"
        );
    }
    // The till's two reads, which the plan's ruling 10 split on purpose
    // (2026-09-21). One shift by id is one person's evening beside what the
    // shop expected them to hold, a report and therefore `SeeReports`; a
    // cashier reading their own open drawer takes no parameter naming
    // anybody, so it carries no row and stays open. Both halves are pinned:
    // a row appearing on `/open` would quietly refuse a cashier the figure
    // they are about to count against, and a row lost off `/{id}` would let
    // one read every colleague's.
    assert_eq!(
        gate_for("GET", "/till/shifts/{id}").and_then(|g| g.permission),
        Some(Permission::SeeReports),
        "GET /till/shifts/{{id}} should carry SeeReports"
    );
    // The list T7 adds, gated the same way as one shift by id above (2026-09-21).
    assert_eq!(
        gate_for("GET", "/till/shifts").and_then(|g| g.permission),
        Some(Permission::SeeReports),
        "GET /till/shifts should carry SeeReports"
    );
    assert!(
        gate_for("GET", "/till/shifts/open").is_none(),
        "GET /till/shifts/open answers about the caller alone and is not this table's business"
    );
}

/// The gate's own default when the walk above is not looking.
///
/// Every write the router answers today has a row, and the two walks are
/// what keep that true. They read `router.rs` as text, so a route that arrives
/// through a helper, a nested router or a `route_layer` is invisible to
/// them. What holds then is the gate itself: a write on a route no
/// permission has been decided for is refused, because the honest answer to
/// "who may do this" is nobody. A read on the same route is the ordinary
/// case and goes through.
#[tokio::test]
async fn a_write_on_a_route_the_table_does_not_name_is_refused() {
    let dir = tempfile::tempdir().expect("no temp dir for the test's shop file");
    let path = dir.path().join("t.db");
    common::sign_in(&path, SHOP);
    let state = dzpos_api::AppState::open(&path, SHOP).expect("the test's shop file will not open");

    const UNKNOWN: &str = "/a-route-nobody-decided-on";
    assert!(
        gate_for("POST", UNKNOWN).is_none() && gate_for("GET", UNKNOWN).is_none(),
        "{UNKNOWN} is supposed to be a path the table has never heard of"
    );

    // The same session layer the real router puts in front of every guarded
    // route, over a route the table does not name.
    let app = axum::Router::new()
        .route(
            UNKNOWN,
            axum::routing::post(|| async { "ok" }).get(|| async { "ok" }),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            dzpos_api::session::require,
        ))
        .with_state(state);

    // The shop's owner, who holds every permission there is, and is still
    // refused: this is not a role decision.
    let (status, body) = call(&app, "POST", UNKNOWN, common::OWNER_SESSION).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(body["error"]["code"], "ungated_write", "{body}");

    let (status, body) = call(&app, "GET", UNKNOWN, common::OWNER_SESSION).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// S5 of `a-kernel-crate-and-retail-as-the-first-module`: the shop's own
/// rows and routes sit behind `#[cfg(feature = "retail")]` in these same
/// two files (this file's own header says why: a table assembled at
/// startup, or a second router, is the runtime registry Samir ruled out).
/// Red against the file this plan found: no row and no route carried the
/// attribute at all, so both counts below were zero and this failed.
#[test]
fn the_shops_own_gate_rows_and_routes_carry_the_retail_feature() {
    let gates_source = include_str!("../src/gates/table.rs");
    let tagged_gates = gates_source
        .matches("    #[cfg(feature = \"retail\")]\n    Gate {")
        .count();
    assert_eq!(
        tagged_gates, 43,
        "43 of ROUTE_GATES' own rows are the shop's; the rest stand with the feature off"
    );

    // The retail block is the one `#[cfg(feature = "retail")]` that guards a
    // `let guarded = guarded` continuing the chain, not the one on the
    // `DefaultBodyLimit` import beside it; it ends where the session layer
    // that closes every guarded route, kernel or shop, begins.
    let (start, end) = retail_block_span();
    let retail_routes = ROUTER_SOURCE[start..end].matches(".route(").count();
    assert_eq!(
        retail_routes, 60,
        "60 of router.rs' own .route(...) calls are the shop's; the rest stand with the feature off"
    );
}

/// C3 of `the-first-clinic-module-patients-queue-appointments`: the clinic's
/// rows and routes carry the clinic feature, the way the shop's carry
/// theirs, and neither module's count moved the other's (the retail test
/// above still reads 43 and 60).
#[test]
fn the_clinics_own_gate_rows_and_routes_carry_the_clinic_feature() {
    let gates_source = include_str!("../src/gates/table.rs");
    let tagged_gates = gates_source
        .matches("    #[cfg(feature = \"clinic\")]\n    Gate {")
        .count();
    assert_eq!(
        tagged_gates, 5,
        "5 of ROUTE_GATES' own rows are the clinic's: list, create, read, update, archive"
    );
    // The rows are there exactly when the clinic is built in.
    let clinic_rows = ROUTE_GATES
        .iter()
        .filter(|g| g.path.starts_with("/patients"))
        .count();
    assert_eq!(clinic_rows, if cfg!(feature = "clinic") { 5 } else { 0 });

    let (start, end) = clinic_block_span();
    let block = &ROUTER_SOURCE[start..end];
    assert_eq!(
        block.matches(".route(").count(),
        3,
        "3 of router.rs' own .route(...) calls are the clinic's"
    );
    // Nothing but the patient file in the block, and no patient route outside it.
    assert!(!ROUTER_SOURCE[..start].contains("\"/patients"));
    assert!(!ROUTER_SOURCE[end..].contains("\"/patients"));
    // And the two blocks do not overlap.
    let (retail_start, _) = retail_block_span();
    assert!(end < retail_start, "the clinic block runs into the shop's");
}
