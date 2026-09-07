use std::fs;
use std::path::PathBuf;

const BACKUP_DIR: &str = "dzpos/backups";
const MAX_BACKUP_DAYS: i64 = 30;

pub fn backup_dir() -> PathBuf {
    let base = dirs::data_dir().expect("Cannot find data directory");
    base.join(BACKUP_DIR)
}

pub fn ensure_backup_dir() -> std::io::Result<PathBuf> {
    let dir = backup_dir();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn last_backup_date() -> Option<String> {
    let dir = backup_dir();
    if !dir.exists() {
        return None;
    }
    let entries = fs::read_dir(&dir).ok()?;
    let mut latest: Option<String> = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("dzpos_") && name.ends_with(".db") {
            let date = name
                .trim_start_matches("dzpos_")
                .trim_end_matches(".db")
                .to_string();
            if latest.as_ref().is_none_or(|l| date > *l) {
                latest = Some(date);
            }
        }
    }
    latest
}

pub fn create_backup(db_path: &str) -> std::io::Result<PathBuf> {
    let dir = ensure_backup_dir()?;
    let now = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let backup_path = dir.join(format!("dzpos_{}.db", now));
    fs::copy(db_path, &backup_path)?;
    Ok(backup_path)
}

pub fn prune_old_backups() -> std::io::Result<usize> {
    let cutoff =
        chrono::Local::now().naive_local().date() - chrono::Duration::days(MAX_BACKUP_DAYS);
    let dir = backup_dir();
    if !dir.exists() {
        return Ok(0);
    }
    let mut removed = 0;
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("dzpos_") && name.ends_with(".db") {
            let date_str = name.trim_start_matches("dzpos_").trim_end_matches(".db");
            if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                if date < cutoff {
                    fs::remove_file(entry.path())?;
                    removed += 1;
                }
            }
        }
    }
    Ok(removed)
}

pub fn backup_if_needed(db_path: &str) {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let last = last_backup_date();
    if last.as_deref() != Some(&today) {
        match create_backup(db_path) {
            Ok(path) => {
                eprintln!("Backup created: {}", path.display());
                match prune_old_backups() {
                    Ok(n) => {
                        if n > 0 {
                            eprintln!("Pruned {} old backup(s)", n);
                        }
                    }
                    Err(e) => eprintln!("Backup prune failed: {}", e),
                }
            }
            Err(e) => eprintln!("Backup failed: {}", e),
        }
    }
}
