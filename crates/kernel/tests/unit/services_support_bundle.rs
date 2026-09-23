// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::read_log;

/// The one branch the walking test cannot reach: it opens the shop
/// through `AppState`, which heads the log before anything can ask for a
/// bundle, so the file always exists by then. A shop that hits trouble
/// before its first session ever writes a line is exactly the shop that
/// reaches for this, and it must get a bundle rather than a refusal.
#[test]
fn a_log_that_was_never_written_reads_as_empty_rather_than_refusing() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("dzpos.log");
    assert!(!missing.exists());
    assert_eq!(read_log(&missing).unwrap(), "");
}

/// The other half of the same branch: a log that is there is read
/// verbatim, so the empty answer above means absent and nothing else.
#[test]
fn a_log_that_is_there_is_read_whole() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dzpos.log");
    std::fs::write(&path, "dz-pos 0.1.0\nsession started\n").unwrap();
    assert_eq!(read_log(&path).unwrap(), "dz-pos 0.1.0\nsession started\n");
}
