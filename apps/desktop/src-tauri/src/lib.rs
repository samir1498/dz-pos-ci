//! The desktop process owns the window and starts the API. The webview
//! talks HTTP to that server, never Tauri IPC for data: the browser preview
//! and, later, the phone are the same callers over the same routes
//! (architecture.md rule 2 and its consequence).

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tauri::{Manager, Url};

pub mod startup_failure;
pub mod updater;

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

/// The launch token, handed out to whoever asks. An earlier version of this
/// gave it to the first caller and refused every caller after; that broke
/// the moment the page evaluated a second time in the same process (a
/// reload, WebView2's own accelerator keys being on by default, an HMR
/// re-import of `api.ts` under `tauri dev`) -- the second `launch_token`
/// call came back refused, its `.catch` turned that into no bearer at all,
/// and every request went out unauthorized until the process restarted.
/// What actually keeps the token off a document it should not reach is that
/// Tauri injects no IPC bridge into a foreign origin in the first place,
/// plus `on_navigation` below keeping the window on the app's own origins;
/// a one-time hand-over added nothing on top of that, since a same-origin
/// script able to ask a second time already holds the `api` client itself
/// (docs/architecture.md § Release).
struct TokenHandoff(String);

#[tauri::command]
fn launch_token(state: tauri::State<TokenHandoff>) -> String {
    state.0.clone()
}

/// Not on Windows, and the reason is the harness rather than the code. Two
/// `cfg` attributes rather than one `all(test, not(windows))`, because
/// clippy reads a literal `#[cfg(test)]` to know a module is a test module
/// and folding the guard inside `all(...)` hides that: the items below the
/// last test module then read as items after it and `items_after_test_module`
/// fails the build.
///
/// These drive `tauri::test`'s mock runtime, which links the webview
/// loader into the test binary; on the windows job that binary refuses to
/// start at all with `STATUS_ENTRYPOINT_NOT_FOUND` (0xc0000139), a symbol
/// missing from a DLL it imports, before a single test runs. Left in, the
/// whole crate's unit tests are lost on Windows, the navigation ones below
/// included. What this costs: `launch_token` answering over IPC is proven
/// on Linux only until somebody runs the suite on a real Windows machine,
/// which the installer work needs anyway. Nothing here is platform
/// behaviour; the command clones a String.
#[cfg(test)]
#[cfg(not(windows))]
#[path = "../tests/unit/lib_token.rs"]
mod token_tests;

/// Where the main window may navigate. Every screen reaches the API over
/// `fetch`, never by loading a new document (rule 2), so this list is the
/// app's own origins and, in a debug build, the Vite dev server the window
/// loads from. Nothing here names the API's own loopback origin: no screen
/// links to it today, and refusing every origin but these is what keeps
/// that true for a screen added later, per the CSP note in
/// docs/architecture.md § Release.
///
/// `lib/download.ts` clicks an `<a download href="blob:...">`; a `blob:`
/// URL is scoped to the document that created it, so it never names a
/// remote origin either way. It is not in this match because `download`
/// anchors are not expected to reach a navigation handler at all in
/// WebView2 or webkitgtk -- they are handed to the platform's download
/// flow before a navigation would start. Not verified against a real
/// webview from here; see the report.
fn allowed_navigation(url: &Url) -> bool {
    match (url.scheme(), url.host_str(), url.port()) {
        ("tauri", Some("localhost"), None) => true,
        ("http", Some("tauri.localhost"), None) => true,
        #[cfg(debug_assertions)]
        ("http", Some("127.0.0.1" | "localhost"), Some(5173)) => true,
        _ => false,
    }
}

// Reads the policy off the page Tauri actually serves, the way a real
// launch would get it: `get_asset` (tauri's own manager) rewrites the CSP
// into `index.html` and returns the same string as a header value at serve
// time, which is what `dist/index.html` on disk never carries -- the
// directive is added when the asset is handed out, not baked into the
// build. `pnpm --filter dzpos-desktop build` has to have run first, the
// same requirement `generate_context!()` already has for `cargo run`.
//
// Getting there needs the `custom-protocol` feature on, which Cargo.toml
// turns on for this dev-dependency only. Two things ride on it: it is what
// `tauri::is_dev()` is (`!cfg!(feature = "custom-protocol")`), and,
// separately, `AssetResolver` reads `dist/index.html` straight off disk
// with `csp_header: None` -- the whole CSP mechanism skipped -- whenever
// `devUrl` is configured (it always is here) and `is_dev()` is true, which
// is every plain `cargo test` otherwise. Without the feature this test
// would pass for the wrong reason: `csp_header` would be `None` because
// the policy was never reached, not because it was absent.
// Not on Windows, for the reason written above `token_tests`: the mock
// runtime this needs refuses to start the test binary there.
#[cfg(test)]
#[cfg(not(windows))]
#[path = "../tests/unit/lib_csp.rs"]
mod csp_tests;

#[cfg(test)]
#[path = "../tests/unit/lib_navigation.rs"]
mod navigation_tests;

// Reads the checked-in config off disk (`include_str!` at compile time, so
// it needs no built `dist/` and runs on every platform, unlike the CSP
// test above): the updater's own `pubkey` and `endpoints`
// (docs/architecture.md § Release). No `tauri::test` mock runtime is
// needed for either assertion, so this is not excluded on Windows.
#[cfg(test)]
#[path = "../tests/unit/lib_updater_config.rs"]
mod updater_config_tests;

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
        // Config comes from tauri.conf.json's `plugins.updater` (pubkey,
        // endpoints); `updater.rs` is the only file that calls
        // `AppHandle::updater()`, behind the two commands below.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            launch_token,
            updater::check_for_update,
            updater::install_update
        ])
        .setup({
            let token = token.clone();
            move |app| {
                // The daily chores (features.md §1): the copy of the shop
                // file, and the stock recount that follows it. Started
                // before the router takes the state, and detached: a chore
                // that fails logs and waits for the next check rather than
                // stopping the till.
                let chores = state.clone();
                tauri::async_runtime::spawn(dzpos_api::daily::run(chores));
                let router = dzpos_api::router(state, &token);
                let task = tauri::async_runtime::spawn(async move {
                    // With the peer address attached, like the standalone
                    // server: the device gate tells loopback from LAN by it.
                    if let Err(e) = axum::serve(
                        listener,
                        router.into_make_service_with_connect_info::<SocketAddr>(),
                    )
                    .await
                    {
                        eprintln!("dz-pos API stopped: {e}");
                    }
                });
                // A poisoned lock here would mean setup already panicked once.
                if let Ok(mut slot) = started.lock() {
                    *slot = Some(task);
                }
                app.manage(DbState { db_path });
                app.manage(ApiPort(port));
                app.manage(TokenHandoff(token.expose().to_owned()));

                // `tauri.conf.json` sets `create: false` on this window so it
                // is built here instead of by the builder: `on_navigation`
                // has to be wired before the first document loads, and a
                // window built from the config alone has no way to carry it.
                // This is the CSP note's `navigate-to` equivalent
                // (docs/architecture.md § Release) -- there is no such CSP
                // directive, `navigate-to` was dropped from the spec before
                // any engine shipped it, so the refusal lives here in Rust
                // rather than in the policy string.
                tauri::WebviewWindowBuilder::from_config(
                    app.handle(),
                    &app.config().app.windows[0],
                )?
                .on_navigation(allowed_navigation)
                .build()?;

                Ok(())
            }
        })
        // The window comes from tauri.conf.json, so the script that tells
        // the page its port is injected by a plugin rather than a window
        // builder. It runs before any of the app's own scripts. The launch
        // token does not travel this way: see `launch_token`.
        .plugin(
            tauri::plugin::Builder::<tauri::Wry, ()>::new("dzpos-api-url")
                .js_init_script(format!(
                    "globalThis.__DZPOS_API_URL__ = \"http://127.0.0.1:{port}\";"
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

/// `pub` so `main.rs` can name it too: on a startup failure it recomputes
/// the same path to hand to `startup_failure::show`, since `run()` keeps
/// its own copy private and is not being restructured to return one out.
pub fn db_path() -> std::io::Result<String> {
    let base = dirs::data_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "no user data directory")
    })?;
    Ok(base
        .join("dzpos")
        .join("dzpos.db")
        .to_string_lossy()
        .to_string())
}
