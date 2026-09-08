use tauri::Manager;

pub struct DbState {
    pub db_path: String,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            let db_path = db_path()?;
            if let Some(parent) = std::path::Path::new(&db_path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            dzpos_core::db::open(&db_path)?;
            app.manage(DbState { db_path });
            Ok(())
        })
        .run(tauri::generate_context!())
}

fn db_path() -> std::io::Result<String> {
    let base = dirs::data_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "no user data directory")
    })?;
    Ok(base
        .join("dzpos")
        .join("dzpos.db")
        .to_string_lossy()
        .to_string())
}
