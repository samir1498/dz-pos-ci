use tauri::Manager;

pub struct DbState {
    pub db_path: String,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let db_path = db_path();
            if let Some(parent) = std::path::Path::new(&db_path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            dzpos_core::db::open(&db_path)?;
            app.manage(DbState { db_path });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn db_path() -> String {
    let base = dirs::data_dir().expect("Cannot find data directory");
    base.join("dzpos")
        .join("dzpos.db")
        .to_string_lossy()
        .to_string()
}
