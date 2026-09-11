// Bakes the git short hash and the build date into the crate at compile
// time (M5 T1, docs/architecture.md § Release). `src/build_info.rs` reads
// both back with `env!`, which only sees a variable this script sets with
// `cargo:rustc-env=`; there is no other way for a value made here to reach
// the compiled crate.
//
// With no `cargo:rerun-if-changed` at all, cargo falls back to watching
// every file under this crate, which a change to HEAD is not: a commit that
// only touches `apps/desktop` or `docs/` would leave this build script
// unrun and the embedded hash pointing at the last commit that did touch
// `crates/core`. `watch_head` below names the two files that actually move
// on a commit or a checkout: `.git/HEAD` and the ref it points at, so a
// dev rebuild picks up the new hash regardless of which crate changed. When
// `git` is unavailable neither is named, and cargo's package-wide fallback
// is what runs the script again on the next build.

use std::process::Command;

/// Set when `git` is missing, the checkout carries no `.git` (a source
/// tarball, a `COPY` into a container that left `.git` out), or the command
/// otherwise fails. Not a hash and not `"unknown"`, so `build_info`'s own
/// tests can tell "no git" apart from a real one rather than a placeholder
/// that reads like a value nobody checked.
const NO_GIT: &str = "nogit";

fn main() {
    let hash = git_short_hash();
    if hash != NO_GIT {
        watch_head();
    }
    println!("cargo:rustc-env=DZPOS_GIT_HASH={hash}");
    println!(
        "cargo:rustc-env=DZPOS_BUILD_DATE={}",
        chrono::Utc::now().date_naive().format("%Y-%m-%d")
    );
}

/// Eight lowercase hex characters, fixed width so a caller never has to
/// guess how many `--short` picked for this repository's object count, or
/// `NO_GIT` when there is no commit to name.
fn git_short_hash() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let hash = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if hash.len() == 8 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
                hash
            } else {
                NO_GIT.to_string()
            }
        }
        _ => NO_GIT.to_string(),
    }
}

/// `git rev-parse --git-path <name>`: the path to a file under the git
/// directory this checkout actually uses, worktree or not (a worktree's
/// `HEAD` lives under `.git/worktrees/<name>/`, not `.git/` itself).
fn git_path(name: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--git-path", name])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Tells cargo to rerun this script when the commit checked out changes:
/// `HEAD` itself, moved by a checkout or a commit, and the branch ref it
/// points at, moved by a commit on the branch that stays checked out.
/// A detached HEAD names a commit directly and has no second file to watch.
fn watch_head() {
    let Some(head_path) = git_path("HEAD") else {
        return;
    };
    println!("cargo:rerun-if-changed={head_path}");
    let Ok(contents) = std::fs::read_to_string(&head_path) else {
        return;
    };
    if let Some(branch_ref) = contents.trim().strip_prefix("ref: ") {
        if let Some(ref_path) = git_path(branch_ref) {
            println!("cargo:rerun-if-changed={ref_path}");
        }
    }
}
