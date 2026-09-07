// Dev-only: fills the local DB with fake shop data. Same path the desktop app opens.
fn main() {
    let base = dirs::data_dir().expect("Cannot find data directory");
    let db_path = base.join("dzpos").join("dzpos.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).expect("Cannot create data directory");
    }
    let db_path = db_path.to_string_lossy().to_string();
    match std::env::args().nth(1) {
        Some(file) => dzpos_seed::run_from_file(&db_path, &file),
        None => dzpos_seed::run(&db_path),
    }
}
