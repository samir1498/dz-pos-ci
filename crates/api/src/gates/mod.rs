//! One table naming the permission each route that needs one wants (M4 T2).
//!
//! The permission enum is one table so a role is never compared twice
//! (`services::permissions`); this is the same idea one layer up, so a route
//! never decides for itself which permission it is about. `tests/route_gates.rs`
//! walks it against the router's own `.route(` lines, in both directions: a
//! mutating route with no row here fails, and a row naming a route that is not
//! there fails.
//!
//! **Nineteen reads are in it.** The table is otherwise about writes, because a
//! read of a list a cashier is already looking at needs no permission. The
//! four exports and the import template are the exception the M3 carry-in
//! named in words: an export is the whole customer list, the whole supplier
//! list and every sale the shop ever rang up, walking out on a USB stick.
//! They carry the same permission as the import that reads the template back.
//! The audit log (M4 T7) and `GET /users` (M4 T8) are the owner's alone,
//! which `services::permissions::can`'s own doc puts with the owner and not
//! with the ordinary lists a cashier is already looking at.
//!
//! Four more joined them on M4 T5's own review (2026-09-11), the day the
//! screens were being hidden from a cashier and the review found the reads
//! behind them were not: `GET /purchases` and `GET /purchases/{id}` are
//! nothing but what the shop pays for its stock (`col_extra_costs`,
//! `col_unit_cost`, `col_landed_cost`), so `Permission::SeeCostAndMargin`
//! gates the whole route rather than a column a handler would have to strip
//! out of a `PurchaseDto` the way `routes/products.rs::redact_cost` strips
//! `ProductDto`'s. `GET /dashboard` and `GET /dashboard/series` are the
//! screen `Permission::SeeReports`'s own doc names ("read the dashboard and
//! the reports it links to"), and nothing a cashier's ordinary work reads:
//! unlike `GET /products`, no till or sale flow calls either. `GET /products`
//! stayed off this table for the reason that the till needs it, and is
//! redacted at the field instead (`ProductDto.cost_centimes` and
//! `.wholesale_centimes`, both `Option`, filled only for a caller who holds
//! `SeeCostAndMargin`); see that DTO's own doc comment.
//!
//! Five more joined on the milestone's closing review (2026-09-11), which
//! found the reads carrying the shop's money were still open after the sweep
//! meant to close them: `GET /backups` sits with the two settings writes
//! beside it, gated the same way. `GET /expenses` and `GET /cash` are what
//! the dashboard's own expense and cash figures come from, read directly
//! rather than through the chart a cashier cannot open, so the same
//! `SeeReports` refusal applies. `GET /suppliers` and
//! `GET /suppliers/{id}/ledger` sit with `GET /purchases` for the same
//! reason: the buying side of the shop, gated whole because no till flow
//! reads either.
//!
//! One more joined on the till shifts (2026-09-21): `GET /till/shifts/{id}`
//! is one person's evening, the cash they took over its window beside what
//! the shop expected them to be holding, which is a report a manager runs
//! the floor off — `Permission::SeeReports`, and not `SeeAuditLog`, which is
//! the owner's alone. Its sibling `GET /till/shifts/open` deliberately has
//! no row: it answers about the caller and takes no parameter naming anybody
//! else, so there is nothing on it a role could be refused, and a cashier
//! who cannot read their own open drawer cannot be shown the expected figure
//! before they count it.
//!
//! **What a row cannot say.** A gate answers for a whole route before the
//! handler runs, so it can only ask about the caller and the path, never
//! about the row the caller named. Where a rule turns on the row, the table
//! carries the coarse half and the service carries the fine one: `POST
//! /sales` is `Sell` here and `services::sales` asks for
//! `DiscountAboveThreshold` and `ChangePriceAtTheTill` on the baskets that
//! need them, and `POST /till/shifts/{id}/close` is `OpenAndCloseTill` here
//! while `services::shifts::close` asks for `CloseAnotherPersonsTill` when
//! the closer is not the opener. A fine permission never has a row of its
//! own, so reading this table alone does not tell you everything a role may
//! be refused for; `services::permissions::Permission`'s own docs do.
//!
//! **Where it is applied.** T2 shipped the mechanism, the table's shape and
//! the actor. T3 applied it: `crates/api/src/session.rs::require` looks the
//! matched route up in this table by `axum::extract::MatchedPath` and calls
//! `permissions::require` before the handler runs, once per request rather
//! than once per handler, so no handler names its own permission a second
//! time.
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
    /// reads the module doc names.
    pub method: &'static str,
    /// The path exactly as `crates/api/src/router.rs` writes it, `{id}`
    /// placeholders and all, so the walking test can match the two by string.
    pub path: &'static str,
    pub permission: Option<Permission>,
    pub why: &'static str,
}

mod table;

pub use table::ROUTE_GATES;

/// The row for one route, if the table has one.
pub fn gate_for(method: &str, path: &str) -> Option<&'static Gate> {
    ROUTE_GATES
        .iter()
        .find(|g| g.method == method && g.path == path)
}

#[cfg(test)]
mod tests;
