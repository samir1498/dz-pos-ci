//! Staff: the fiche, the PIN, the password, and the first owner.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// A fiche on the users screen (M4 T8). No hash and no failure counter ever
/// travel: `has_pin` and `has_password` are the honest answer to "can this
/// person sign in", and a reset never has an old PIN to show because there
/// is not one to show (`services::users`' own doc).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "UserDto.ts")]
pub struct UserDto {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub role: RoleDto,
    pub has_pin: bool,
    pub has_password: bool,
    pub active: bool,
}

impl From<User> for UserDto {
    fn from(u: User) -> Self {
        UserDto {
            id: u.id,
            shop_id: u.shop_id,
            name: u.name,
            role: RoleDto::from(u.role),
            has_pin: u.has_pin,
            has_password: u.has_password,
            active: u.active,
        }
    }
}

/// One name on the sign-in picker. `GET /auth/staff` answers a list of these
/// to a caller that is inside the device gate and has no session yet, so a
/// cashier taps their name and types only their PIN: the user id is a row
/// number nobody standing at a counter knows, and the screens that asked
/// for it were asking for the database's key.
///
/// The fields are chosen for what leaves the shop if a paired phone is
/// stolen: a name, a role, and which door opens for that name. No id of the
/// shop, no `active` (the list holds only active fiches), and no hash, the
/// same rule `UserDto` keeps. `has_pin` decides which box the picker shows
/// after the tap; `has_password` is what the owner's door needs to know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "StaffDto.ts")]
pub struct StaffDto {
    pub id: i32,
    pub name: String,
    pub role: RoleDto,
    pub has_pin: bool,
    pub has_password: bool,
}

impl From<User> for StaffDto {
    fn from(u: User) -> Self {
        StaffDto {
            id: u.id,
            name: u.name,
            role: RoleDto::from(u.role),
            has_pin: u.has_pin,
            has_password: u.has_password,
        }
    }
}

/// A fiche's name and role, the two fields the screen's "add a user" dialog
/// sends. No credential: a PIN is its own call
/// (`services::users::create`'s own doc), so `POST /users/{id}/pin` is what
/// gives a fresh row its first one.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewUserDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewUserDto {
    pub name: String,
    pub role: RoleDto,
}

/// The body `POST /users/{id}/pin` takes: a PIN alone, on the fiche the path
/// names. The same call gives a fresh row its first PIN and resets one that
/// is forgotten; `services::users::set_pin` does not tell the two apart and
/// neither does this.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "SetPinDto.ts")]
#[serde(deny_unknown_fields)]
pub struct SetPinDto {
    pub pin: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "SetPasswordDto.ts")]
#[serde(deny_unknown_fields)]
pub struct SetPasswordDto {
    pub password: String,
}

/// The body `POST /auth/first-setup` takes: a name and a password, no
/// `user_id`. Nobody signed in yet is not in a position to name a row; the
/// shop's own owner is who `services::users::claim_first_owner` finds and
/// acts on. The PIN for the till is set later from the users screen.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "ClaimFirstOwnerDto.ts")]
#[serde(deny_unknown_fields)]
pub struct ClaimFirstOwnerDto {
    pub name: String,
    pub password: String,
}
