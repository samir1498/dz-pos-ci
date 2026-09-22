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
mod tests {
    use super::*;

    /// The version, the hash and the date this binary was actually built
    /// with: not empty, not `"unknown"`, the hash the shape a real one takes
    /// or the named "no git" sentinel, the date parseable. Guards against
    /// `build.rs` silently emitting an empty string, which `env!` would
    /// still compile against, or a placeholder word standing in for a value
    /// nobody checked.
    #[test]
    fn the_three_are_present_and_none_of_them_is_a_placeholder() {
        let info = BUILD_INFO;
        assert!(!info.version.is_empty(), "no version was embedded");
        assert_ne!(info.version, "unknown");
        assert!(!info.git_hash.is_empty(), "no git hash was embedded");
        assert_ne!(info.git_hash, "unknown");
        assert!(!info.build_date.is_empty(), "no build date was embedded");
        assert_ne!(info.build_date, "unknown");
    }

    /// Semver as `CARGO_PKG_VERSION` always writes it: three dot-separated
    /// numeric groups, pre-release and build metadata allowed after them.
    #[test]
    fn the_version_looks_like_semver() {
        let core = BUILD_INFO.version.split(['-', '+']).next().unwrap_or("");
        let parts: Vec<&str> = core.split('.').collect();
        assert_eq!(
            parts.len(),
            3,
            "{} is not major.minor.patch",
            BUILD_INFO.version
        );
        for part in parts {
            assert!(
                !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()),
                "{part} in {} is not a number",
                BUILD_INFO.version
            );
        }
    }

    /// This checkout has git: `build.rs` ran inside it, so the embedded hash
    /// is the real thing, eight lowercase hex characters, never the "no
    /// git" sentinel and never invented text of some other shape.
    #[test]
    fn the_git_hash_this_checkout_was_built_with_is_real_and_the_right_shape() {
        let hash = BUILD_INFO.git_hash;
        assert_ne!(
            hash, NO_GIT,
            "this worktree has git; build.rs should have found it"
        );
        assert_eq!(hash.len(), 8, "{hash} is not eight characters");
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "{hash} is not hexadecimal"
        );
    }

    /// The contract a build with no git available has to meet: whatever
    /// `build.rs` embeds is either a real hash or exactly [`NO_GIT`], never
    /// empty and never a third spelling invented for the occasion. This is
    /// what a CI container with no `.git` would see; `build.rs`'s own
    /// fallback is exercised by hand (see the PR description) since a
    /// `cargo test` run always has git.
    #[test]
    fn the_git_hash_is_either_real_or_names_itself_as_missing() {
        let hash = BUILD_INFO.git_hash;
        let looks_real = hash.len() == 8 && hash.chars().all(|c| c.is_ascii_hexdigit());
        assert!(
            looks_real || hash == NO_GIT,
            "{hash} is neither a real hash nor the no-git sentinel"
        );
    }

    /// `YYYY-MM-DD`, parseable, and not a date claiming to be from before
    /// this product existed.
    #[test]
    fn the_build_date_is_a_real_calendar_day() {
        let date = chrono::NaiveDate::parse_from_str(BUILD_INFO.build_date, "%Y-%m-%d")
            .unwrap_or_else(|e| panic!("{} does not parse: {e}", BUILD_INFO.build_date));
        let earliest = chrono::NaiveDate::from_ymd_opt(2026, 9, 8)
            .unwrap_or_else(|| panic!("the earliest bound date is invalid"));
        assert!(
            date >= earliest,
            "{date} is before the project's first commit"
        );
    }

    /// A debug build says so, in the same line a release build shows,
    /// rather than only in a field a screen might drop. Built from two
    /// hand-made `BuildInfo` values rather than `BUILD_INFO`, so the test
    /// proves the branch both ways regardless of which mode compiled this
    /// test binary.
    #[test]
    fn a_debug_build_says_so_and_a_release_build_does_not() {
        let debug = BuildInfo {
            version: "0.1.0",
            git_hash: "abcd1234",
            build_date: "2026-09-11",
            is_debug: true,
        };
        let release = BuildInfo {
            is_debug: false,
            ..debug
        };

        let debug_line = header_line(&debug);
        let release_line = header_line(&release);
        assert!(
            debug_line.to_lowercase().contains("debug"),
            "{debug_line} does not say it is a debug build"
        );
        assert!(
            !release_line.to_lowercase().contains("debug"),
            "{release_line} claims to be a debug build"
        );
        assert_ne!(debug_line, release_line);
    }

    /// `header_line` names the fields it carries; used by the log file and,
    /// later, the support bundle, so a caller reading it back can trust the
    /// three values are literally present in the text rather than summarised
    /// away.
    #[test]
    fn header_line_carries_all_three_fields() {
        let line = header_line(&BUILD_INFO);
        assert!(line.contains(BUILD_INFO.version));
        assert!(line.contains(BUILD_INFO.git_hash));
        assert!(line.contains(BUILD_INFO.build_date));
    }
}
