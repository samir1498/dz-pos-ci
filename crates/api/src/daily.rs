//! The daily chores (features.md §1): the automatic copy of the SQLite file,
//! and the stock recount that proves the cached quantity on hand still
//! matches the ledger. Both decisions of whether one is due are pure
//! functions in the core (`backup::is_due` and `stock::is_due`); this module
//! is only the loop around them and the logging, so nothing here can be
//! wrong in a way a test could not reach.
//!
//! The recount runs after the copy, so a repair the owner disagrees with is
//! recoverable from the previous copy: on an ordinary day that is the one
//! just written, and on a day the copy failed it is the last one that
//! worked. Which is why the recount is not gated on the copy having worked:
//! a backup folder the shop's antivirus has locked would otherwise leave the
//! cached quantities wrong for as long as the lock lasts.
//!
//! The loop wakes more often than once a day on purpose. A shop PC is turned
//! off at night: a task that slept a flat 24 hours would only ever fire while
//! the shop was open long enough, and a till switched on at 8 and off at 20
//! would drift a backup later every day until it fell off the end. Waking
//! hourly and asking `is_due` makes the copy happen on the first hour after a
//! day has passed, whenever the machine is on.

use std::time::Duration;

use dzpos_core::services::backup::{self, Backup};
#[cfg(feature = "retail")]
use dzpos_core::services::stock::{self, Report};

use crate::routes::backups::now;
use crate::AppState;

/// How often the loop asks whether a copy is due. Not how often one is made.
pub const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

/// Runs forever: one round of the chores now, then one every
/// [`CHECK_EVERY`].
pub async fn run(state: AppState) {
    loop {
        once(&state).await;
        tokio::time::sleep(CHECK_EVERY).await;
    }
}

/// One round of the chores: the copy, then the recount. Returns what each of
/// them did, which is what lets a test read the order and the independence
/// off the loop itself rather than off two calls a test made in that order.
///
/// The recount is deliberately not gated on the copy: see the note at the top
/// of this file.
///
/// Retail-only (S5): a kernel-only build has no cached quantity on hand to
/// recount, so the loop is the copy alone.
#[cfg(feature = "retail")]
pub async fn once(state: &AppState) -> (Option<Backup>, Option<Report>) {
    let copy = tick(state).await;
    let counted = recount(state).await;
    (copy, counted)
}

#[cfg(not(feature = "retail"))]
pub async fn once(state: &AppState) -> Option<Backup> {
    tick(state).await
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
/// Who the nightly repair is recorded under. Row 1, the owner the first
/// migration seeds and the one every document carried before there were
/// sessions (features.md §5).
///
/// This is the one place in the crate that still names a user by constant,
/// and it is not the gap M4 T2 closed: no session exists at three in the
/// morning, and there is nobody at the keyboard to take one from. Claiming a
/// person did something the machine did on its own is worse in an audit log
/// than saying the shop did, which is the argument `services::users` makes
/// for the lockout row naming the person it happened to. T7's screen is where
/// this row reads differently from the ones a person wrote.
///
/// Retail-only (S5): named beside `recount` below, which is the only reader.
#[cfg(feature = "retail")]
const NIGHTLY_ACTOR_USER_ID: i32 = 1;

#[cfg(feature = "retail")]
pub async fn recount(state: &AppState) -> Option<Report> {
    let shop = state.shop_id;
    let user = NIGHTLY_ACTOR_USER_ID;
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
#[path = "../tests/unit/daily.rs"]
mod tests;
