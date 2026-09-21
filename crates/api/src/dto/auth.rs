//! Signing in, the session that comes back, roles and permissions.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// What a sign-in sends. Two shapes and not one struct with four optional
/// fields: the till's PIN pad picks a row off the list and sends an id, the
/// office screen asks for a name and a password, and a body carrying a name
/// beside a PIN is a caller that has not decided which it is doing. Untagged,
/// so the wire stays the two plain objects a screen would send anyway.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "LoginDto.ts")]
#[serde(untagged)]
// Both shapes at once is a caller that has not decided which door it is at,
// refused as malformed rather than tried as the first shape that fits.
#[serde(deny_unknown_fields)]
pub enum LoginDto {
    /// The till. `user_id` and never a name: a PIN pad has the list in front
    /// of it, and a name typed at a keypad would be a way to ask the shop
    /// whether somebody works there.
    Pin { user_id: i32, pin: String },
    /// Everywhere else.
    Password { name: String, password: String },
}

/// Who is signed in, as every screen reads it: the person, their role, and
/// what that role may do.
///
/// The permission list travels because the permission table lives in the core
/// (`services::permissions::can`) and a screen that decided for itself which
/// buttons a manager gets would be a second statement of it
/// (architecture.md rule 2). T5's `Can` component reads this array and
/// nothing else; no screen compares a role string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "MeDto.ts")]
pub struct MeDto {
    pub user_id: i32,
    pub name: String,
    pub role: RoleDto,
    pub permissions: Vec<PermissionDto>,
}

/// A sign-in's answer: who is now acting, and the session token.
///
/// The token is in the body because the desktop webview cannot read the
/// httpOnly cookie the same response sets, and a Tauri window has no other
/// way to learn it. A browser client ignores this field and lets the cookie
/// travel; the cookie is what protects a session already open, and an
/// attacker who could read this body would have had to send the PIN to get
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "SessionDto.ts")]
pub struct SessionDto {
    pub me: MeDto,
    pub token: String,
    /// How long the session survives with nothing happening on it, in whole
    /// minutes. The screen counts the lock screen down against this rather
    /// than holding a figure of its own (T4).
    pub idle_minutes: i64,
}

/// The three roles. One value the screens read, never a comparison they make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "RoleDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum RoleDto {
    Owner,
    Manager,
    Cashier,
}

impl From<Role> for RoleDto {
    fn from(r: Role) -> Self {
        match r {
            Role::Owner => RoleDto::Owner,
            Role::Manager => RoleDto::Manager,
            Role::Cashier => RoleDto::Cashier,
        }
    }
}

impl From<RoleDto> for Role {
    fn from(r: RoleDto) -> Self {
        match r {
            RoleDto::Owner => Role::Owner,
            RoleDto::Manager => Role::Manager,
            RoleDto::Cashier => Role::Cashier,
        }
    }
}

/// The permission list, one variant per `services::permissions::Permission`.
/// Serialised as the same string `Permission::as_str` writes, which is also
/// the name a 403 carries, so a screen matches one spelling everywhere.
///
/// The `From` below matches on the core enum, so a fifteenth permission
/// fails to compile here until it is named on the wire too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "PermissionDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum PermissionDto {
    Sell,
    DiscountAboveThreshold,
    OverrideCreditBlock,
    SeeCostAndMargin,
    EditFiches,
    EditSettings,
    SeeReports,
    ManageUsers,
    CommitMoney,
    CorrectLedger,
    ExportAndImport,
    ChangePriceAtTheTill,
    SeeAuditLog,
    OpenAndCloseTill,
}

impl From<Permission> for PermissionDto {
    fn from(p: Permission) -> Self {
        match p {
            Permission::Sell => PermissionDto::Sell,
            Permission::DiscountAboveThreshold => PermissionDto::DiscountAboveThreshold,
            Permission::OverrideCreditBlock => PermissionDto::OverrideCreditBlock,
            Permission::SeeCostAndMargin => PermissionDto::SeeCostAndMargin,
            Permission::EditFiches => PermissionDto::EditFiches,
            Permission::EditSettings => PermissionDto::EditSettings,
            Permission::SeeReports => PermissionDto::SeeReports,
            Permission::ManageUsers => PermissionDto::ManageUsers,
            Permission::CommitMoney => PermissionDto::CommitMoney,
            Permission::CorrectLedger => PermissionDto::CorrectLedger,
            Permission::ExportAndImport => PermissionDto::ExportAndImport,
            Permission::ChangePriceAtTheTill => PermissionDto::ChangePriceAtTheTill,
            Permission::SeeAuditLog => PermissionDto::SeeAuditLog,
            Permission::OpenAndCloseTill => PermissionDto::OpenAndCloseTill,
        }
    }
}

/// The idle time a shop has set, in whole minutes. Its own body rather than a
/// field of the settings block, because T2 ships the mechanism and the
/// settings screen that edits it is T8's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "SessionIdleDto.ts")]
#[serde(deny_unknown_fields)]
pub struct SessionIdleDto {
    pub idle_minutes: i64,
}
