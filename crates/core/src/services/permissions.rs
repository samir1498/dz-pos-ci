//! Roles and the permission table (features.md §5, this milestone's
//! subject). `can` is the one place a role is compared to decide what
//! somebody may do; every route, screen and service asks it, or `require`
//! below, rather than comparing a role of its own
//! (`docs/architecture.md` § Transport and auth).
//!
//! `Role` is `models::sql_types::Role`, the type the `users.role` column
//! already stores, re-exported here so a caller of `can` imports one name.
//! This module carried its own copy while the users table was being written
//! on another branch; the two met on 2026-09-11 and the copy went.

use diesel::sqlite::SqliteConnection;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::money::{Bps, Money};
use crate::services::audit;

pub use crate::models::sql_types::Role;

/// The three roles, for a caller that walks them all (the permission table's
/// own tests, and any screen that offers the list). `models::sql_types::Role`
/// is a stored enum and its macro writes no such constant.
pub const ROLES: [Role; 3] = [Role::Owner, Role::Manager, Role::Cashier];

/// What a shopkeeper would call each of the things a role decides
/// (features.md §5, the M4 team plan and its carry-ins). Named for the
/// action, not for a route: several routes share one permission, the way
/// every export and both import routes share `ExportAndImport` (M3 carry-in
/// ruling, 2026-09-10).
///
/// The stock recount is not its own variant. The same ruling reads it onto
/// `CorrectLedger`, "the way a supplier adjustment and a purchase return
/// already do": a twelfth permission that answers the same question as an
/// existing one would be a second statement of a rule already decided, which
/// is exactly what this table exists to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Ring a sale up. The one thing every role may do.
    Sell,
    /// Give a discount above the settings threshold
    /// (`services::settings::discount_threshold_as_of`). At or under the
    /// threshold needs nothing: features.md §5's "give discount above X %"
    /// is the shape of the rule, and a caller checks `discount_needs_permission`
    /// before it asks for this one.
    DiscountAboveThreshold,
    /// Pass a credit sale that the customer's credit limit would otherwise
    /// block. Owner-only until this milestone (M2 carry-in, 2026-09-09); T6
    /// moves the gate to this permission.
    OverrideCreditBlock,
    /// Read what a product cost the shop and what a sale made on it: the
    /// cost and landed-cost columns on products, purchases and the
    /// dashboard.
    SeeCostAndMargin,
    /// Change a card the shop keeps on something it deals with: a product's
    /// fiche (price, name, barcode, category) and a supplier's, closing or
    /// reopening one included, because a body carrying `active: false` closes
    /// a supplier through the same rule the close route uses (M3 carry-in,
    /// 2026-09-10). Named for the card and not for the product because the
    /// same hand keeps both.
    EditFiches,
    /// Change the shop's settings screen, the régime fiscal included.
    EditSettings,
    /// Read the dashboard and the reports it links to.
    SeeReports,
    /// Add, rename, reset a PIN or password on, deactivate or reactivate a
    /// user. The finer rule that the shop's last active owner cannot be
    /// moved off the role or switched off is T0's, enforced on the row, and
    /// not a second permission here.
    ManageUsers,
    /// Send money out of the shop on purpose: a supplier payment, a
    /// purchase order and its receipt, an expense (M3 carry-in,
    /// 2026-09-10).
    CommitMoney,
    /// Correct a ledger the ordinary flow does not: a supplier debt
    /// adjustment, a purchase return or close-short, or a stock recount
    /// that rewrites a cached quantity on hand (M3 carry-in, 2026-09-10);
    /// and undo a document already handed to a customer, which is a
    /// cancellation or an avoir against a facture (M2 carry-in, 2026-09-09).
    /// Undoing a sale and undoing a purchase are the same act on the two
    /// sides of the counter, so they answer to the same permission.
    CorrectLedger,
    /// Walk the shop's data out on a USB stick, or rewrite its prices in
    /// bulk from one: the four exports, the product import template, and
    /// both import routes (M3 carry-in, 2026-09-10). Labels are not this: a
    /// name, a price and a barcode a customer can already read off the
    /// shelf need nobody's permission.
    ExportAndImport,
    /// Sell a line at a price that is not the product's own: the till's
    /// negotiated price (`NewSaleLine.unit_price`). M1 shipped it ungated
    /// because the till had one user; this is the gate that ruling asked for
    /// (M1 carry-in, 2026-09-09), and T3 writes the audit row beside it with
    /// the stored price next to the one used.
    ChangePriceAtTheTill,
    /// Read the audit log: who changed a price, who passed a credit block,
    /// who put a quantity back to what its ledger sums to. The owner's, and
    /// only the owner's, because a log that the people it watches can also
    /// read is not a control (T7 writes the screen "for the owner").
    SeeAuditLog,
}

impl Permission {
    pub const ALL: [Permission; 13] = [
        Permission::Sell,
        Permission::DiscountAboveThreshold,
        Permission::OverrideCreditBlock,
        Permission::SeeCostAndMargin,
        Permission::EditFiches,
        Permission::EditSettings,
        Permission::SeeReports,
        Permission::ManageUsers,
        Permission::CommitMoney,
        Permission::CorrectLedger,
        Permission::ExportAndImport,
        Permission::ChangePriceAtTheTill,
        Permission::SeeAuditLog,
    ];

    /// The stable key the UI translates and the wire carries (T2: 403 with
    /// the `forbidden` code and this name).
    pub const fn as_str(self) -> &'static str {
        match self {
            Permission::Sell => "sell",
            Permission::DiscountAboveThreshold => "discount_above_threshold",
            Permission::OverrideCreditBlock => "override_credit_block",
            Permission::SeeCostAndMargin => "see_cost_and_margin",
            Permission::EditFiches => "edit_fiches",
            Permission::EditSettings => "edit_settings",
            Permission::SeeReports => "see_reports",
            Permission::ManageUsers => "manage_users",
            Permission::CommitMoney => "commit_money",
            Permission::CorrectLedger => "correct_ledger",
            Permission::ExportAndImport => "export_and_import",
            Permission::ChangePriceAtTheTill => "change_price_at_the_till",
            Permission::SeeAuditLog => "see_audit_log",
        }
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The one statement of who may do what. Every place in the codebase that
/// would otherwise compare a role asks this, or `require` below, instead.
///
/// A cashier rings sales up and nothing else on this list. A manager runs the
/// shop and answers like an owner on everything except two: who the staff are,
/// and the log of what the staff did. A log the people it watches can read,
/// and a staff list they can add themselves to, are not controls, and the
/// milestone's own demo line is "the owner sees the audit log of a price
/// change" (review ruling, 2026-09-11; features.md §5 names neither way, so
/// this table is the statement of it). The régime fiscal still sits inside
/// `EditSettings`, which a manager holds; splitting that out is the next
/// question if Samir wants the fiscal setting owner-only.
///
/// The match is on `permission` first and not on the `(role, permission)`
/// pair, so a twelfth `Permission` variant fails to compile here until it is
/// placed, rather than silently defaulting through a wildcard.
pub const fn can(role: Role, permission: Permission) -> bool {
    match permission {
        Permission::Sell => true,
        Permission::DiscountAboveThreshold
        | Permission::OverrideCreditBlock
        | Permission::SeeCostAndMargin
        | Permission::EditFiches
        | Permission::EditSettings
        | Permission::SeeReports
        | Permission::CommitMoney
        | Permission::CorrectLedger
        | Permission::ExportAndImport
        | Permission::ChangePriceAtTheTill => matches!(role, Role::Owner | Role::Manager),
        Permission::ManageUsers | Permission::SeeAuditLog => matches!(role, Role::Owner),
    }
}

/// `can`, turned into the typed refusal a caller raises instead of writing
/// its own `if`. The permission travels back out of the very check that
/// wanted it, so a route or a screen never restates which one it asked for.
pub fn require(role: Role, permission: Permission) -> Result<(), CoreError> {
    if can(role, permission) {
        Ok(())
    } else {
        Err(CoreError::forbidden(permission))
    }
}

/// The row `crates/api/src/session.rs::require` writes when `require` above
/// refuses somebody: a refusal used to leave nothing behind.
/// That is the only caller, and on purpose: it is the one seam every gated
/// request already passes through, so the row is written once and a handler
/// never has to remember to.
///
/// What is not a row here, decided at that one seam and not restated: a 401
/// with no session, because there is no person yet to write one about; and
/// `ungated_write`, a route the permission table forgot rather than a role
/// asking for something it may not, which names no permission to record.
/// Everything this function is actually called on is a route
/// `crates/api/src/gates.rs` deliberately names a permission for, a read as
/// much as a write — `ExportAndImport` on `GET /export/*` is exactly the case
/// the finding was written about, a cashier trying the export route and
/// leaving nothing behind — so the method is carried on the row rather than
/// used to decide whether there is one.
///
/// The API layer calls this rather than building the row itself
/// (`docs/architecture.md` § Transport and auth: an audit row is business,
/// never transport), and it is a single `INSERT` that commits on its own, the
/// way `services::users::settle`'s failure counter does: the refusal that
/// triggers it happens before any handler transaction has opened, so there is
/// nothing here for a later rollback to undo.
pub fn record_refusal(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    permission: Permission,
    method: &str,
    route: &str,
) -> Result<(), CoreError> {
    audit::record(
        conn,
        shop_id,
        actor_id,
        audit::Change {
            action: audit::ACTION_PERMISSION_REFUSED,
            entity: "permission",
            entity_id: None,
            before: None,
            after: Some(
                serde_json::json!({
                    "permission": permission.as_str(),
                    "method": method,
                    "route": route,
                })
                .to_string(),
            ),
        },
    )
}

/// Whether a discount needs `Permission::DiscountAboveThreshold`: strictly
/// more than `threshold` of `base`. "At or below the threshold needs no
/// permission" (this task's brief), so the compare is strict and not
/// `>=`. `base.pct` is the one rounding rule this crate uses for a rate
/// against an amount (half away from zero, once), so this check and a TVA
/// line agree on what "19 % of 1000" is.
pub fn discount_needs_permission(
    base: Money,
    discount: Money,
    threshold: Bps,
) -> Result<bool, CoreError> {
    Ok(discount > base.pct(threshold)?)
}
