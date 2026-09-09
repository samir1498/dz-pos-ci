//! The daily backup (features.md §1: an automatic daily copy of the SQLite
//! file). The decision of whether one is due is
//! `dzpos_core::services::backup::is_due`, a pure function of the newest
//! copy and the clock; this module is only the loop around it and the
//! logging, so nothing here can be wrong in a way a test could not reach.
//!
//! The loop wakes more often than once a day on purpose. A shop PC is turned
//! off at night: a task that slept a flat 24 hours would only ever fire while
//! the shop was open long enough, and a till switched on at 8 and off at 20
//! would drift a backup later every day until it fell off the end. Waking
//! hourly and asking `is_due` makes the copy happen on the first hour after a
//! day has passed, whenever the machine is on.

use std::time::Duration;

use dzpos_core::services::backup::{self, Backup};

use crate::routes::backups::now;
use crate::AppState;

/// How often the loop asks whether a copy is due. Not how often one is made.
pub const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

/// Runs forever: one check now, then one every [`CHECK_EVERY`].
pub async fn run(state: AppState) {
    loop {
        tick(&state).await;
        tokio::time::sleep(CHECK_EVERY).await;
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

    use super::tick;
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
}
