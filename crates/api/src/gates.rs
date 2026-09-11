//! One table naming the permission each route that needs one wants (M4 T2).
//!
//! The permission enum is one table so a role is never compared twice
//! (`services::permissions`); this is the same idea one layer up, so a route
//! never decides for itself which permission it is about. `tests/route_gates.rs`
//! walks it against the router's own `.route(` lines, in both directions: a
//! mutating route with no row here fails, and a row naming a route that is not
//! there fails.
//!
//! **Five reads are in it.** The table is otherwise about writes, because a
//! read of a list a cashier is already looking at needs no permission. The
//! four exports and the import template are the exception the M3 carry-in
//! named in words: an export is the whole customer list, the whole supplier
//! list and every sale the shop ever rang up, walking out on a USB stick.
//! They carry the same permission as the import that reads the template back.
//!
//! **What T2 ships and what T3 does.** T2 is the mechanism, the table's shape
//! and the actor. What T2 deliberately does not do is apply it: no handler
//! calls `permissions::require` yet, and a cashier is refused by nothing. T3
//! is one pass down this table, and the session middleware can look a row up
//! by `axum::extract::MatchedPath` rather than each handler naming its own
//! permission a second time.
//!
//! **Where the rows come from.** Some are rulings the plan already took: the
//! M1, M2 and M3 carry-ins in `context/plans/20260908-m4-team.md` name the
//! permission for the till, the stock screens, the settings block, the
//! imports and exports and the price change at the till, and those are
//! decisions rather than suggestions, so they are written out here rather
//! than invented again later. The rest are read off the same rulings by the
//! block a screen sits in, and T2 took them: the customer fiches and their
//! payments and adjustments, the backup and restore routes, the purchase
//! cancellation, and the sale routes past the first. Each says in `why` what
//! it was read off. T3 is where they are argued with, and a row changed there
//! is a row changed here, not a second opinion in a handler.

use dzpos_core::services::permissions::Permission;

/// One route, and the permission it wants. `permission: None` is a mutating
/// route that is deliberately open to every signed-in role, and `why` is the
/// ruling that says so: a blank in this table has to be a decision somebody
/// took, not a row nobody filled in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gate {
    /// Upper case, as the router spells it: `POST`, `PUT`, and `GET` for the
    /// five reads the module doc names.
    pub method: &'static str,
    /// The path exactly as `crates/api/src/lib.rs` writes it, `{id}`
    /// placeholders and all, so the walking test can match the two by string.
    pub path: &'static str,
    pub permission: Option<Permission>,
    pub why: &'static str,
}

/// Every mutating route this API answers, and the five reads that carry the
/// shop's lists out of it, with what each will want.
///
/// Sorted by path, which is how `lib.rs` lists its routes, so the two read
/// side by side.
pub const ROUTE_GATES: &[Gate] = &[
    Gate {
        method: "POST",
        path: "/auth/login",
        permission: None,
        why: "signing in is how a role comes to exist; gating it on one would be a lock with its key inside",
    },
    Gate {
        method: "POST",
        path: "/auth/logout",
        permission: None,
        why: "anybody who is signed in may stop being, and a sign-out gated on a role is a screen nobody can leave",
    },
    Gate {
        method: "POST",
        path: "/backups",
        permission: Some(Permission::EditSettings),
        why: "a backup is taken from the settings screen and is the shop's whole file; the block it sits in is the settings one",
    },
    Gate {
        method: "POST",
        path: "/backups/{name}/restore",
        permission: Some(Permission::EditSettings),
        why: "a restore replaces every row the shop has; it belongs with the settings it is reached from",
    },
    Gate {
        method: "POST",
        path: "/customers",
        permission: Some(Permission::EditFiches),
        why: "a customer fiche is a card the shop keeps on somebody it deals with",
    },
    Gate {
        method: "PUT",
        path: "/customers/{id}",
        permission: Some(Permission::EditFiches),
        why: "the same fiche, and a body carrying a credit limit is the fiche too",
    },
    Gate {
        method: "POST",
        path: "/customers/{id}/payments",
        permission: None,
        why: "taking money a customer owes over the counter is a till job; it is the debt side of ringing a sale up",
    },
    Gate {
        method: "POST",
        path: "/customers/{id}/adjustments",
        permission: Some(Permission::CorrectLedger),
        why: "correcting what a customer owes is a ledger the ordinary flow does not write (M2 carry-in)",
    },
    Gate {
        method: "POST",
        path: "/expenses",
        permission: Some(Permission::CommitMoney),
        why: "money out of the drawer, the same way a supplier payment is (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "POST",
        path: "/import/products/dry-run",
        permission: Some(Permission::ExportAndImport),
        why: "an import rewrites prices in bulk from a USB stick (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "POST",
        path: "/import/products",
        permission: Some(Permission::ExportAndImport),
        why: "the same workbook as the dry run, and this is the call that actually rewrites the prices",
    },
    Gate {
        method: "GET",
        path: "/import/products/template",
        permission: Some(Permission::ExportAndImport),
        why: "the template carries the shop's own products and their prices, so it walks out the same list an export does (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "GET",
        path: "/export/customers",
        permission: Some(Permission::ExportAndImport),
        why: "the whole customer list on a USB stick, names and debts included (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "GET",
        path: "/export/products",
        permission: Some(Permission::ExportAndImport),
        why: "the whole catalogue with its cost prices (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "GET",
        path: "/export/sales",
        permission: Some(Permission::ExportAndImport),
        why: "every sale the shop ever rang up (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "GET",
        path: "/export/suppliers",
        permission: Some(Permission::ExportAndImport),
        why: "the whole supplier list and what is owed to each (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "POST",
        path: "/labels/sheet",
        permission: None,
        why: "a label prints a name, a price and a barcode a customer can already read off the shelf, and a cashier relabelling a shelf is who needs it (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "POST",
        path: "/products",
        permission: Some(Permission::EditFiches),
        why: "a product's fiche: price, name, barcode, category",
    },
    Gate {
        method: "PUT",
        path: "/products/{id}",
        permission: Some(Permission::EditFiches),
        why: "the same fiche: a price, a name, a barcode or a category changed on a product the shop already stocks",
    },
    Gate {
        method: "POST",
        path: "/purchases",
        permission: Some(Permission::CommitMoney),
        why: "an order commits the shop's money (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "POST",
        path: "/purchases/{id}/receipts",
        permission: Some(Permission::CommitMoney),
        why: "a receipt is what turns the order into what the shop owes (M3 carry-in)",
    },
    Gate {
        method: "POST",
        path: "/purchases/{id}/returns",
        permission: Some(Permission::CorrectLedger),
        why: "a return lowers what the shop owes (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "POST",
        path: "/purchases/{id}/cancel",
        permission: Some(Permission::CorrectLedger),
        why: "undoing an order is the purchase side of undoing a document",
    },
    Gate {
        method: "POST",
        path: "/purchases/{id}/close-short",
        permission: Some(Permission::CorrectLedger),
        why: "a write-off lowers what the shop owes (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "POST",
        path: "/sales",
        permission: Some(Permission::Sell),
        why: "ringing a sale up, the one thing every role may do; named rather than left blank because it is a permission and not an absence of one",
    },
    Gate {
        method: "POST",
        path: "/sales/{id}/avoir",
        permission: Some(Permission::CorrectLedger),
        why: "an avoir undoes a document already handed to a customer (M2 carry-in, 2026-09-09)",
    },
    Gate {
        method: "POST",
        path: "/sales/{id}/cancel",
        permission: Some(Permission::CorrectLedger),
        why: "the same, and this one voids the document outright (M2 carry-in, 2026-09-09)",
    },
    Gate {
        method: "PUT",
        path: "/settings/store",
        permission: Some(Permission::EditSettings),
        why: "the seller block every ticket and every facture prints, and the identifiers a facture is refused without",
    },
    Gate {
        method: "POST",
        path: "/settings/regime",
        permission: Some(Permission::EditSettings),
        why: "the régime fiscal sits inside EditSettings, which a manager holds; splitting it out is the next question if Samir wants it owner-only (services::permissions says so too)",
    },
    Gate {
        method: "PUT",
        path: "/settings/theme",
        permission: None,
        why: "cosmetic, not money and not stock, so it stays open to everybody unless a shop asks otherwise (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "POST",
        path: "/stock/recount",
        permission: Some(Permission::CorrectLedger),
        why: "a recount rewrites a cached quantity on hand from the ledger (M3 carry-in, 2026-09-10; routes/stock.rs carried the TODO)",
    },
    Gate {
        method: "POST",
        path: "/suppliers",
        permission: Some(Permission::EditFiches),
        why: "a supplier's fiche is a card the shop keeps, kept by the same hand as a product's",
    },
    Gate {
        method: "PUT",
        path: "/suppliers/{id}",
        permission: Some(Permission::EditFiches),
        why: "the same fiche, and the closing branch of it: a body carrying `active: false` closes a fiche through the same rule the close route uses (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "POST",
        path: "/suppliers/{id}/close",
        permission: Some(Permission::EditFiches),
        why: "closing a fiche, named beside the PUT that does it the other way (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "POST",
        path: "/suppliers/{id}/payments",
        permission: Some(Permission::CommitMoney),
        why: "money out to a supplier (M3 carry-in, 2026-09-10)",
    },
    Gate {
        method: "POST",
        path: "/suppliers/{id}/adjustments",
        permission: Some(Permission::CorrectLedger),
        why: "correcting what the shop owes a supplier (M3 carry-in, 2026-09-10)",
    },
];

/// The row for one route, if the table has one.
pub fn gate_for(method: &str, path: &str) -> Option<&'static Gate> {
    ROUTE_GATES
        .iter()
        .find(|g| g.method == method && g.path == path)
}

#[cfg(test)]
mod tests {
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
            // A read in here is one of the five the module doc names and
            // never a sixth added in passing: an open read is open by having
            // no row at all, not by a row with no permission.
            if gate.method == "GET" {
                assert!(
                    gate.permission.is_some(),
                    "{} is a read with no permission; leave it out of the table instead",
                    gate.path
                );
                assert!(
                    gate.path.starts_with("/export/") || gate.path == "/import/products/template",
                    "{} is a read this table was not opened for",
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

    #[test]
    fn a_route_can_be_looked_up_and_an_unknown_one_is_not_there() {
        let sale = gate_for("POST", "/sales").expect("POST /sales has no row");
        assert_eq!(sale.permission, Some(Permission::Sell));
        assert!(gate_for("GET", "/sales").is_none());
        assert!(gate_for("POST", "/nowhere").is_none());
    }

    /// The carry-in rulings, as the plan wrote them, read back off the table
    /// rather than restated in prose. If one of these ever changes it changes
    /// in the plan first and here second, and this is what notices.
    #[test]
    fn the_carry_in_rulings_are_what_the_table_says() {
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
        // The theme stays open.
        assert_eq!(wants("PUT", "/settings/theme"), None);
    }
}
