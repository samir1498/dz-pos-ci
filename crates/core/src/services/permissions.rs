//! Roles and the permission table (features.md §5, this milestone's
//! subject). `can` is the one place a role is compared to decide what
//! somebody may do; every route, screen and service asks it, or `require`
//! below, rather than comparing a role of its own
//! (`docs/architecture.md` § Transport and auth).
//!
//! `Role` duplicates `models::sql_types::Role`, which M4 T0 is writing on its
//! own branch at the same time (`m4/t0-users`, migration `2026-09-11-000011`)
//! for the `users.role` column. T1 must not depend on the users table, so
//! this is its own type until the branches meet: same three variants, same
//! stored strings, same shape (`as_str`, `parse`, `Display`). The merge is
//! expected to delete this one and `pub use` T0's from `models::sql_types`
//! instead; nothing here is stored anywhere, so nothing else moves.

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::money::{Bps, Money};

/// What a user is allowed to be. See the module doc for why this is not
/// `models::sql_types::Role`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Manager,
    Cashier,
}

impl Role {
    pub const ALL: [Role; 3] = [Role::Owner, Role::Manager, Role::Cashier];

    pub const fn as_str(self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Manager => "manager",
            Role::Cashier => "cashier",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "owner" => Some(Role::Owner),
            "manager" => Some(Role::Manager),
            "cashier" => Some(Role::Cashier),
            _ => None,
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a shopkeeper would call each of the things a role decides
/// (features.md §5, the M4 team plan and its carry-ins). Named for the
/// action, not for a route: several routes share one permission, the way
/// every export and both import routes share `ExportAndImport` (M3 carry-in
/// ruling, 2026-09-10).
///
/// The stock recount is not its own variant. The same ruling reads it onto
/// `CorrectLedger`, "the way a supplier adjustment and a purchase return
/// already do" — a twelfth permission that answers the same question as an
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
    /// Change a product's fiche: price, name, barcode, category.
    EditProducts,
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
    /// that rewrites a cached quantity on hand (M3 carry-in, 2026-09-10).
    CorrectLedger,
    /// Walk the shop's data out on a USB stick, or rewrite its prices in
    /// bulk from one: the four exports, the product import template, and
    /// both import routes (M3 carry-in, 2026-09-10). Labels are not this: a
    /// name, a price and a barcode a customer can already read off the
    /// shelf need nobody's permission.
    ExportAndImport,
}

impl Permission {
    pub const ALL: [Permission; 11] = [
        Permission::Sell,
        Permission::DiscountAboveThreshold,
        Permission::OverrideCreditBlock,
        Permission::SeeCostAndMargin,
        Permission::EditProducts,
        Permission::EditSettings,
        Permission::SeeReports,
        Permission::ManageUsers,
        Permission::CommitMoney,
        Permission::CorrectLedger,
        Permission::ExportAndImport,
    ];

    /// The stable key the UI translates and the wire carries (T2: 403 with
    /// the `forbidden` code and this name).
    pub const fn as_str(self) -> &'static str {
        match self {
            Permission::Sell => "sell",
            Permission::DiscountAboveThreshold => "discount_above_threshold",
            Permission::OverrideCreditBlock => "override_credit_block",
            Permission::SeeCostAndMargin => "see_cost_and_margin",
            Permission::EditProducts => "edit_products",
            Permission::EditSettings => "edit_settings",
            Permission::SeeReports => "see_reports",
            Permission::ManageUsers => "manage_users",
            Permission::CommitMoney => "commit_money",
            Permission::CorrectLedger => "correct_ledger",
            Permission::ExportAndImport => "export_and_import",
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
/// A cashier rings sales up and nothing else on this list; a manager and an
/// owner answer alike on every permission today. Nothing read for this
/// milestone (features.md §5, the M4 team plan and its M1-M3 carry-ins)
/// draws a line between owner and manager: T7's audit log screen is written
/// "for the owner" and T8 refuses only "a cashier" on user management, so a
/// split between the two, if the review wants one, is a later ruling and not
/// a permission this table already holds.
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
        | Permission::EditProducts
        | Permission::EditSettings
        | Permission::SeeReports
        | Permission::ManageUsers
        | Permission::CommitMoney
        | Permission::CorrectLedger
        | Permission::ExportAndImport => matches!(role, Role::Owner | Role::Manager),
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
