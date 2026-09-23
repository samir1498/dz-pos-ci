//! The one write this crate does outside the shop's own database: a line
//! appended to a plain text file at the start of a session (M5 T1). There is
//! no logging framework yet, only this header; a later task that adds one
//! writes into the file this opens rather than a second file.

use std::io::Write;
use std::path::Path;

use crate::build_info::{header_line, BUILD_INFO};

/// Appends `build_info::header_line` to `path`, creating the file and its
/// parent folder if neither exists yet. Called once per process start
/// (`AppState::open_with_backup_dir` in `crates/api`, which both the
/// standalone API binary and the desktop process go through), so the file
/// reads as a sequence of sessions, each headed by the build that ran it.
///
/// Best effort by design: a folder the process cannot write to should not
/// stop a shop's till from opening. The caller logs the error to stderr and
/// carries on; nothing here panics.
pub fn head_session(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(
        file,
        "{}, session started {}",
        header_line(&BUILD_INFO),
        chrono::Utc::now().to_rfc3339()
    )
}

#[cfg(test)]
#[path = "../tests/unit/log.rs"]
mod tests;
