//! `daily.rs`'s own tests, declared there with `#[path]` rather than inline:
//! `scripts/file-sizes.mjs` caps that file at 600 lines, and this module is
//! still a child of it (`super::` below resolves its private items) so
//! nothing about what it can reach changes.
//!
//! Both tests are retail-only (S5 of
//! `a-kernel-crate-and-retail-as-the-first-module`): the nightly stock
//! recount they prove does not exist without the feature, and `once`'s
//! kernel-only shape (the copy alone, no `Report` beside it) has nothing
//! left here to test until S7 gives a kernel-only build a shop file to open.

// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(feature = "retail")]

use super::{once, recount, tick};
use crate::AppState;

#[tokio::test]
async fn a_copy_that_could_not_be_written_still_lets_the_recount_run() {
    // The backup folder is a regular file here, so both reading it and
    // writing into it fail the way a folder the shop's antivirus has
    // locked does. The cached quantities must not be left wrong for as
    // long as that lasts: the recount is a chore of its own and the
    // previous copy is still there to go back to.
    let dir = tempfile::tempdir().unwrap();
    let blocked = dir.path().join("backups");
    std::fs::write(&blocked, b"not a folder").unwrap();
    let state = AppState::open_with_backup_dir(dir.path().join("t.db"), 1, &blocked).unwrap();

    let (copy, counted) = once(&state).await;
    assert!(copy.is_none(), "a file was somehow written into a file");
    let counted = counted.expect("the failed copy took the recount down with it");
    assert!(counted.drifts.is_empty(), "a fresh file drifted");
    assert!(
        once(&state).await.1.is_none(),
        "the same day was recounted twice"
    );
}

/// The once-a-day rule, on both chores the loop makes: the decision either
/// is due is a pure function in the core (`backup::is_due`, `stock::is_due`);
/// what this proves is that the loop calls each once and does not run either
/// twice on the same day. One test rather than two of the same shape, first
/// the copy then the recount.
#[tokio::test]
async fn the_first_tick_and_the_first_recount_each_run_once_a_day() {
    let dir = tempfile::tempdir().unwrap();
    let state = AppState::open(dir.path().join("t.db"), 1).unwrap();

    let made = tick(&state).await.expect("no copy on an empty folder");
    assert!(made.path.is_file());
    assert!(
        tick(&state).await.is_none(),
        "a second copy was taken the same day"
    );
    let listed = dzpos_core::services::backup::list(state.backup_dir()).unwrap();
    assert_eq!(listed.len(), 1);

    let first = recount(&state).await.expect("the shop was never recounted");
    assert!(first.drifts.is_empty(), "a fresh file drifted");
    assert!(
        recount(&state).await.is_none(),
        "the same day was recounted twice"
    );
}
