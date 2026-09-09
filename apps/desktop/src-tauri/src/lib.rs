//! The desktop process owns the window and starts the API. The webview
//! talks HTTP to that server, never Tauri IPC for data: the browser preview
//! and, later, the phone are the same callers over the same routes
//! (architecture.md rule 2 and its consequence).

use std::sync::{Arc, Mutex};

use tauri::Manager;

/// The task serving the API, shared between `setup` and the run handler that
/// stops it.
type ApiTask = Arc<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>>;

/// v1 is one shop and one SQLite file. M6 pairs a second till; the value
/// stops being a constant then, not before.
const SHOP_ID: i32 = 1;

/// tauri.conf.json names no label for its one window, so it gets Tauri's
/// default. The single-instance callback needs it by name.
const MAIN_WINDOW: &str = "main";

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
    // Fresh each launch and handed only to this process's own webview: the
    // loopback port is open to every process on the machine, the token is
    // what makes the API answer this window and nobody else
    // (docs/architecture.md, "Transport and auth").
    let token = dzpos_api::LaunchToken::generate().map_err(|e| format!("no randomness: {e}"))?;

    // The socket is bound before the window exists so the page can be told
    // its port in the first script it runs; port 0 lets the OS pick, which
    // keeps two tills on one machine from fighting over 4317.
    let (listener, port) = tauri::async_runtime::block_on(dzpos_api::bind(0))?;

    // Held so RunEvent::Exit can stop the server. The task owns the listener
    // and the connection; leaving it running past the window would hold the
    // port and the SQLite file open with nothing to answer for them. Two
    // closures reach it, setup and the run handler, so it is shared.
    let api_task: ApiTask = Arc::new(Mutex::new(None));
    let started = Arc::clone(&api_task);

    tauri::Builder::default()
        // First, per the plugin's own docs: it has to see the launch before
        // anything else runs. A second dz-pos on one machine would open a
        // second connection to the same file and bind a second port, so the
        // second launch focuses the window that is already there.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
                // Best effort: a window that refuses to come forward is not
                // a reason to fail the launch that is already running.
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup({
            let token = token.clone();
            move |app| {
                let router = dzpos_api::router(state, &token);
                let task = tauri::async_runtime::spawn(async move {
                    if let Err(e) = axum::serve(listener, router).await {
                        eprintln!("dz-pos API stopped: {e}");
                    }
                });
                // A poisoned lock here would mean setup already panicked once.
                if let Ok(mut slot) = started.lock() {
                    *slot = Some(task);
                }
                app.manage(DbState { db_path });
                app.manage(ApiPort(port));
                Ok(())
            }
        })
        // The window comes from tauri.conf.json, so the script that tells
        // the page its port and its token is injected by a plugin rather
        // than a window builder. It runs before any of the app's own
        // scripts. The token is hex, so it needs no escaping.
        .plugin(
            tauri::plugin::Builder::<tauri::Wry, ()>::new("dzpos-api-url")
                .js_init_script(format!(
                    "globalThis.__DZPOS_API_URL__ = \"http://127.0.0.1:{port}\";\
                     globalThis.__DZPOS_API_TOKEN__ = \"{}\";",
                    token.expose()
                ))
                .build(),
        )
        .build(tauri::generate_context!())?
        .run(move |_app, event| {
            if let tauri::RunEvent::Exit = event {
                // The window is gone; nothing is left to serve. Aborting the
                // task drops the listener and the connection with it.
                if let Ok(mut slot) = api_task.lock() {
                    if let Some(task) = slot.take() {
                        task.abort();
                    }
                }
            }
        });
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
