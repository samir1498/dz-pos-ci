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
mod tests {
    // Tests may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::head_session;

    #[test]
    fn a_missing_file_and_its_missing_folder_are_both_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dzpos.log");
        head_session(&path).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("dz-pos"));
        assert!(written.contains("session started"));
    }

    #[test]
    fn a_second_session_is_appended_and_the_first_line_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dzpos.log");
        head_session(&path).unwrap();
        head_session(&path).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = written.lines().collect();
        assert_eq!(
            lines.len(),
            2,
            "the first session's line was not kept: {written}"
        );
        for line in lines {
            assert!(line.contains("dz-pos") && line.contains("session started"));
        }
    }
}
