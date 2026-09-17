//! What the server says about itself: health, its clock, and the build it is.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "HealthDto.ts")]
pub struct HealthDto {
    pub status: String,
    pub shop_id: i32,
    /// True while nobody in the shop has a PIN or a password. The desktop
    /// shows the first-setup screen then, not a sign-in for a user who does not
    /// exist yet (features.md §5).
    pub needs_first_setup: bool,
}

/// The day the shop is on, `YYYY-MM-DD`. A screen that needs "today" asks
/// for it rather than reading the machine's calendar: the core dates every
/// document on Algeria's, UTC+1 with no daylight saving, and a browser in
/// another zone would date a statement a day either side of what the ledger
/// holds (features.md §2, "One clock").
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "ClockDto.ts")]
pub struct ClockDto {
    pub today: String,
}

/// The version, the git short hash and the build date the running binary
/// was built with (M5 T1, docs/architecture.md § Release), plus whether it
/// is a debug build. The About screen's only source for the three: there is
/// no second, hand-typed copy in `apps/desktop`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "BuildInfoDto.ts")]
pub struct BuildInfoDto {
    pub version: String,
    pub git_hash: String,
    pub build_date: String,
    pub debug: bool,
}

impl From<dzpos_core::build_info::BuildInfo> for BuildInfoDto {
    fn from(info: dzpos_core::build_info::BuildInfo) -> Self {
        BuildInfoDto {
            version: info.version.to_string(),
            git_hash: info.git_hash.to_string(),
            build_date: info.build_date.to_string(),
            debug: info.is_debug,
        }
    }
}
