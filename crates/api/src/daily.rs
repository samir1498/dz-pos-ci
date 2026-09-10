//! The daily chores (features.md §1): the automatic copy of the SQLite file,
//! and the stock recount that proves the cached quantity on hand still
//! matches the ledger. Both decisions of whether one is due are pure
//! functions in the core (`backup::is_due` and `stock::is_due`); this module
//! is only the loop around them and the logging, so nothing here can be
//! wrong in a way a test could not reach.
//!
//! The recount runs after the copy, so a repair the owner disagrees with is
//! recoverable from that day's backup. It is not gated on the copy having
//! worked: a backup folder the shop's antivirus has locked would otherwise
//! leave the cached quantities wrong for as long as the lock lasts, and
//! yesterday's copy is still there to go back to.
//!
//! The loop wakes more often than once a day on purpose. A shop PC is turned
//! off at night: a task that slept a flat 24 hours would only ever fire while
//! the shop was open long enough, and a till switched on at 8 and off at 20
//! would drift a backup later every day until it fell off the end. Waking
//! hourly and asking `is_due` makes the copy happen on the first hour after a
//! day has passed, whenever the machine is on.

use std::time::Duration;

use dzpos_core::services::backup::{self, Backup};
use dzpos_core::services::stock::{self, Report};

use crate::routes::backups::now;
use crate::AppState;

/// How often the loop asks whether a copy is due. Not how often one is made.
pub const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

/// Runs forever: one check now, then one every [`CHECK_EVERY`].
pub async fn run(state: AppState) {
    loop {
        tick(&state).await;
        recount(&state).await;
        tokio::time::sleep(CHECK_EVERY).await;
    }
}

/// The nightly stock recount, once per shop day. The whole rule is
/// `stock::recount_if_due`: it reads the marker and does the work under one
/// transaction, so a relaunch at 23:59 and the check a minute later are one
/// run on the day each of them falls in.
///
/// `None` means the shop was already counted today, or that the attempt
/// failed. A failure is logged and dropped for the reason a failed copy is:
/// the cached quantity is a convenience the ledger can rebuild, and nothing
/// here may take the till down.
pub async fn recount(state: &AppState) -> Option<Report> {
    let shop = state.shop_id;
    // TODO(M4): the user the repair is recorded under comes from the
    // request identity once there are users to have one.
    let user = state.user_id;
    match state
        .blocking(move |c| stock::recount_if_due(c, shop, user))
        .await
    {
        Ok(Some(report)) => {
            println!(
                "dz-pos: stock recount for {}: {} products checked, {} corrected",
                report.day,
                report.checked,
                report.drifts.len()
            );
            Some(report)
        }
        Ok(None) => None,
        Err(e) => {
            eprintln!("dz-pos: the daily stock recount failed: {e}");
            None
        }
    }
}

/// One check. Returns the copy it made, or `None` when none was due or the
/// attempt failed.
///
/// A failure is logged and dropped: the copy is a background chore, and a
/// full disk or a folder the shop's antivirus has locked must not take the
/// till down with it. The next tick tries again.
pub async fn tick(state: &AppState) -> Option<Backup> {
    let dir = state.backup_dir().to_path_buf();
    let newest = match tokio::task::spawn_blocking(move || backup::list(&dir)).await {
        Ok(Ok(found)) => found.first().map(|b| b.taken_at),
        Ok(Err(e)) => {
            eprintln!("dz-pos: the backup folder could not be read: {e}");
            return None;
        }
        Err(_) => return None,
    };
    if !backup::is_due(newest, now()) {
        return None;
    }
    let at = now();
    let state = state.clone();
    match tokio::task::spawn_blocking(move || state.create_backup(at)).await {
        Ok(Ok(made)) => {
            println!("dz-pos: backup written to {}", made.path.display());
            Some(made)
        }
        Ok(Err(e)) => {
            eprintln!("dz-pos: the daily backup failed: {e}");
            None
        }
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    // Tests may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{recount, tick};
    use crate::AppState;

    #[tokio::test]
    async fn the_first_tick_writes_a_copy_and_the_next_one_does_not() {
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
    }

    #[tokio::test]
    async fn the_first_check_recounts_the_shop_and_the_next_one_the_same_day_does_not() {
        // The once-a-day rule as the loop meets it. The decision itself is
        // `stock::is_due`, tested on its own in the core; what this proves is
        // that the loop asks and does not run twice on one day.
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::open(dir.path().join("t.db"), 1).unwrap();

        let first = recount(&state).await.expect("the shop was never recounted");
        assert!(first.drifts.is_empty(), "a fresh file drifted");
        assert!(
            recount(&state).await.is_none(),
            "the same day was recounted twice"
        );
    }
}
