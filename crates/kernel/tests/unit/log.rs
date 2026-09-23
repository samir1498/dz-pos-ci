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
