// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// One row per route, and every row says something. A `why` left empty on
/// an open route would be the blank this table exists to refuse.
#[test]
fn every_row_is_filled_in_and_no_route_is_named_twice() {
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for gate in ROUTE_GATES {
        assert!(
            matches!(gate.method, "POST" | "PUT" | "GET"),
            "{} is not a method this table carries",
            gate.method
        );
        // A read in here is one of the reads the module doc names and
        // never one added in passing: an open read is open by having no
        // row at all, not by a row with no permission.
        if gate.method == "GET" {
            assert!(
                gate.permission.is_some(),
                "{} is a read with no permission; leave it out of the table instead",
                gate.path
            );
            assert!(
                gate.path.starts_with("/export/")
                    || gate.path == "/import/products/template"
                    || gate.path == "/audit-log"
                    || gate.path == "/users"
                    || gate.path == "/dashboard"
                    || gate.path == "/dashboard/series"
                    || gate.path == "/purchases"
                    || gate.path == "/purchases/{id}"
                    || gate.path == "/expenses"
                    || gate.path == "/cash"
                    || gate.path == "/suppliers"
                    || gate.path == "/suppliers/{id}/ledger"
                    || gate.path == "/backups"
                    || gate.path == "/support-bundle"
                    || gate.path == "/pairing/devices"
                    || gate.path == "/till/shifts/{id}"
                    || gate.path == "/till/shifts"
                    || gate.path == "/patients"
                    || gate.path == "/patients/{id}",
                "{} is a read this table was not opened for; widen this allow-list deliberately \
                 and say why in ROUTE_GATES's own `why` (M4 T5 review, 2026-09-11: /dashboard, \
                 /dashboard/series, /purchases and /purchases/{{id}} joined the exports, the \
                 import template, the audit log and /users as reads a cashier does not get. \
                 M4 closing review, same day: /expenses, /cash, /suppliers, \
                 /suppliers/{{id}}/ledger and /backups joined them, the money the shop spends \
                 and owes having been readable by a cashier while the pages that sum it were \
                 not. M5 T3: /support-bundle joined them, the zip a shop sends out gated the \
                 same way the backups block beside it is). M6 T4: /pairing/devices joined them, same gate as the QR. \
                 Till shifts, 2026-09-21: /till/shifts/{{id}} joined them, one person's evening being a \
                 report a manager runs the floor off; its sibling /till/shifts/open stayed off the table \
                 because it answers about the caller and names nobody else. Closing sweep, same day: \
                 /till/shifts joined them too, the same report widened from a row to a day's list. \
                 Clinic C3, 2026-09-23: /patients and /patients/{{id}} joined them behind \
                 ViewPatients, a patient's medical identity and notes being no ordinary list.",
                gate.path
            );
        } else {
            // The other half of the same rule, and the half `route_gates.rs`
            // cannot keep: its refusal walk reads `gate.permission` and skips
            // the row when it is `None`, so a write whose permission is taken
            // away is a write nothing probes any more. Every open write is
            // therefore named here, and a write that is not on this list
            // carries a permission or fails.
            assert!(
                gate.permission.is_some()
                    || matches!(
                        (gate.method, gate.path),
                        ("POST", "/auth/login")
                            | ("POST", "/auth/logout")
                            | ("POST", "/auth/first-setup")
                            | ("POST", "/pairing/claim")
                            | ("POST", "/customers/{id}/payments")
                            | ("POST", "/labels/sheet")
                            | ("PUT", "/settings/theme")
                    ),
                "{} {} writes with no permission and is not one of the seven this table was \
                 opened for. Four of those seven are how a credential comes to exist at all \
                 (the two sign-in routes, the first owner, and the phone trading its QR), and \
                 the other three are rulings somebody took in words: taking a customer's \
                 payment over the counter is the debt side of ringing a sale up, a label \
                 prints what a customer can already read off the shelf, and the theme is \
                 cosmetic (M3 carry-in, 2026-09-10). Widen this list the same way, by taking \
                 the decision first and writing the reason in the row's own `why`.",
                gate.method,
                gate.path
            );
        }
        assert!(gate.path.starts_with('/'), "{} is not a path", gate.path);
        assert!(
            gate.why.len() > 20,
            "{} {} says nothing about why",
            gate.method,
            gate.path
        );
        let key = (gate.method, gate.path);
        assert!(!seen.contains(&key), "{key:?} is in the table twice");
        seen.push(key);
    }
}

// A kernel route stands with the feature off (`table.rs`'s own header), so
// this proves the lookup mechanics with one rather than `/sales`, which S7
// (`a-kernel-crate-and-retail-as-the-first-module`) leaves with no row at
// all in a `--no-default-features` build.
#[test]
fn a_route_can_be_looked_up_and_an_unknown_one_is_not_there() {
    let qr = gate_for("POST", "/pairing/qr").expect("POST /pairing/qr has no row");
    assert_eq!(qr.permission, Some(Permission::EditSettings));
    assert!(gate_for("GET", "/pairing/qr").is_none());
    assert!(gate_for("POST", "/nowhere").is_none());
}

/// The carry-in rulings for the kernel's own routes, as the plan wrote
/// them, read back off the table rather than restated in prose. Split from
/// the retail half below (S7) because these hold with the feature off.
#[test]
fn the_kernel_carry_in_rulings_are_what_the_table_says() {
    let wants = |method, path| gate_for(method, path).and_then(|g| g.permission);
    // The theme stays open.
    assert_eq!(wants("PUT", "/settings/theme"), None);
    // The facture layout does not: it changes the paper, not a screen.
    assert_eq!(
        wants("PUT", "/settings/facture-layout"),
        Some(Permission::EditSettings)
    );
    // The language every fiscal paper prints in is the paper too.
    assert_eq!(
        wants("PUT", "/settings/print-lang"),
        Some(Permission::EditSettings)
    );
    // So is which wire the thermal head is sent down.
    assert_eq!(
        wants("PUT", "/settings/thermal-mode"),
        Some(Permission::EditSettings)
    );
}

/// The carry-in rulings for the shop's own routes: every path named below
/// has no row at all with the feature off (S7), so this half stays behind
/// it rather than reading `None` back for the wrong reason.
#[cfg(feature = "retail")]
#[test]
fn the_retail_carry_in_rulings_are_what_the_table_says() {
    let wants = |method, path| gate_for(method, path).and_then(|g| g.permission);
    // The exports and imports, and the labels that are deliberately not
    // with them.
    for (method, path) in [
        ("POST", "/import/products"),
        ("POST", "/import/products/dry-run"),
        ("GET", "/import/products/template"),
        ("GET", "/export/products"),
        ("GET", "/export/sales"),
        ("GET", "/export/customers"),
        ("GET", "/export/suppliers"),
    ] {
        assert_eq!(
            wants(method, path),
            Some(Permission::ExportAndImport),
            "{method} {path}"
        );
    }
    assert_eq!(wants("POST", "/labels/sheet"), None);
    // Money out, and the ledger corrections beside it.
    for path in ["/expenses", "/purchases", "/suppliers/{id}/payments"] {
        assert_eq!(wants("POST", path), Some(Permission::CommitMoney), "{path}");
    }
    for path in [
        "/purchases/{id}/returns",
        "/purchases/{id}/close-short",
        "/suppliers/{id}/adjustments",
        "/stock/recount",
        "/sales/{id}/cancel",
        "/sales/{id}/avoir",
    ] {
        assert_eq!(
            wants("POST", path),
            Some(Permission::CorrectLedger),
            "{path}"
        );
    }
    // The supplier fiche, both ways into closing it.
    assert_eq!(
        wants("POST", "/suppliers/{id}/close"),
        Some(Permission::EditFiches)
    );
    assert_eq!(
        wants("PUT", "/suppliers/{id}"),
        Some(Permission::EditFiches)
    );
}
