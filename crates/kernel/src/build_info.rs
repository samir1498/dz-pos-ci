//! The one place the shipped version, the git short hash and the build date
//! come from (M5 T1, docs/architecture.md § Release). `build.rs` bakes the
//! last two in with `env!` at compile time; a shipped binary has no
//! repository beside it, so nothing here shells out to `git` at run time.
//! `crates/api` reads `BUILD_INFO` into `BuildInfoDto` for the About screen
//! and the web side never spells a second copy of any of the three
//! (`apps/desktop/src/routes/settings_.about.test.tsx` fails if it does);
//! `AppState::open_with_backup_dir` (`crates/api/src/lib.rs`) writes
//! [`header_line`] at the head of the log file on every session; and
//! `header_line` is the line T3's support bundle carries as its own header,
//! read straight off this module rather than reassembled from the DTO.

/// `build.rs`'s own answer when `git` is missing or the checkout carries no
/// `.git`. Not a hash and not `"unknown"`: a value that would pass a check
/// for "not a placeholder" is worse than one a test can name and refuse.
pub const NO_GIT: &str = "nogit";

/// The three release identifiers, plus whether this binary was built with
/// debug assertions on. `Copy`: every field is a `&'static str` or a `bool`,
/// baked in at compile time, so a caller never needs to clone or hold a
/// reference to `BUILD_INFO` itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildInfo {
    /// `CARGO_PKG_VERSION`, i.e. `[workspace.package] version`. Semver,
    /// no `v` prefix.
    pub version: &'static str,
    /// Eight lowercase hex characters, or [`NO_GIT`].
    pub git_hash: &'static str,
    /// `YYYY-MM-DD`, UTC, the day `crates/kernel` was compiled.
    pub build_date: &'static str,
    /// `cfg!(debug_assertions)`: on for `cargo build` and `cargo test`, off
    /// for `cargo build --release`, which is what the installer ships.
    pub is_debug: bool,
}

pub const BUILD_INFO: BuildInfo = BuildInfo {
    version: env!("CARGO_PKG_VERSION"),
    git_hash: env!("DZPOS_GIT_HASH"),
    build_date: env!("DZPOS_BUILD_DATE"),
    is_debug: cfg!(debug_assertions),
};

/// One line carrying all three, plus a debug marker when `is_debug` is set.
/// What the log file heads every session with, and the line T3's support
/// bundle will carry as its own header rather than reformatting the three
/// fields again.
pub fn header_line(info: &BuildInfo) -> String {
    let mut line = format!(
        "dz-pos {} ({}, built {})",
        info.version, info.git_hash, info.build_date
    );
    if info.is_debug {
        line.push_str(" [debug build]");
    }
    line
}

#[cfg(test)]
#[path = "../tests/unit/build_info.rs"]
mod tests;
