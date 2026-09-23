//! Backups and restores.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// One copy of the shop file in the backup folder. `name` is both what the
/// screen shows and the id the restore route takes back, so a caller never
/// builds a path: the server owns the folder and only the name crosses.
/// `bytes` is a file size, which is why it is not money and not centimes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "BackupDto.ts")]
pub struct BackupDto {
    pub name: String,
    /// `YYYY-MM-DDTHH:MM:SS` on the shop's own calendar, the time the copy
    /// was taken, read from the name rather than from the file's mtime.
    pub taken_at: String,
    pub bytes: i64,
}

impl From<Backup> for BackupDto {
    fn from(b: Backup) -> Self {
        BackupDto {
            name: b.name,
            taken_at: b.taken_at.format(STAMP_FORMAT).to_string(),
            // A backup past 9.2 exabytes would round in JavaScript. The
            // clamp is what keeps the wire honest rather than a silent
            // rounding.
            bytes: i64::try_from(b.bytes).unwrap_or(MAX_SAFE_INTEGER),
        }
    }
}

/// What the settings screen reads: the daily copies, the copies taken on the
/// way into a restore, and the copies taken on the way into an upgrade. The
/// three are separate lists because they are kept under different rules: the
/// daily ones are pruned to thirty, the other two are never touched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "BackupsDto.ts")]
pub struct BackupsDto {
    pub backups: Vec<BackupDto>,
    pub safety_copies: Vec<BackupDto>,
    pub upgrade_copies: Vec<BackupDto>,
}

/// What the shop file holds after a restore: the copy it came from and the
/// counts read out of it, so the screen can say what landed instead of
/// "done".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "RestoreDto.ts")]
pub struct RestoreDto {
    pub restored_from: String,
    /// The copy of the shop file as it was a moment before, taken on the way
    /// in and kept beside the shop file. Named on the wire because nothing
    /// deletes it and the owner is the only one who can decide to.
    pub safety_copy: String,
    /// Retail-only (S5): the counts a kernel-only restore has none of,
    /// `AppState::restore`'s own doc says why.
    #[cfg(feature = "retail")]
    pub products: i64,
    /// Null only for a copy taken before the documents table existed
    /// (migration 2, the sale); every copy since carries the count.
    #[cfg(feature = "retail")]
    pub documents: Option<i64>,
}
