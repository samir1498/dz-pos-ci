//! The desktop process owns the window and starts the API. The webview
//! talks HTTP to that server, never Tauri IPC for data: the browser preview
//! and, later, the phone are the same callers over the same routes
//! (architecture.md rule 2 and its consequence).

use tauri::Manager;

/// v1 is one shop and one SQLite file. M7 pairs a second till; the value
/// stops being a constant then, not before.
const SHOP_ID: i32 = 1;

pub struct DbState {
    pub db_path: String,
}

/// Where the webview reaches the API. Injected into the page rather than
/// fetched over IPC: it is configuration, not data.
pub struct ApiPort(pub u16);

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = db_path()?;
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let state = dzpos_api::AppState::open(&db_path, SHOP_ID)?;

    // The socket is bound before the window exists so the page can be told
    // its port in the first script it runs; port 0 lets the OS pick, which
    // keeps two tills on one machine from fighting over 4317.
    let (listener, port) = tauri::async_runtime::block_on(dzpos_api::bind(0))?;

    tauri::Builder::default()
        .setup(move |app| {
            let router = dzpos_api::router(state);
            tauri::async_runtime::spawn(async move {
                if let Err(e) = axum::serve(listener, router).await {
                    eprintln!("dz-pos API stopped: {e}");
                }
            });
            app.manage(DbState { db_path });
            app.manage(ApiPort(port));
            Ok(())
        })
        // The window comes from tauri.conf.json, so the script that tells
        // the page its port is injected by a plugin rather than a window
        // builder. It runs before any of the app's own scripts.
        .plugin(
            tauri::plugin::Builder::<tauri::Wry, ()>::new("dzpos-api-url")
                .js_init_script(format!(
                    "globalThis.__DZPOS_API_URL__ = \"http://127.0.0.1:{port}\";"
                ))
                .build(),
        )
        .run(tauri::generate_context!())?;
    Ok(())
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
