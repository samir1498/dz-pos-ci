// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKitGTK Wayland workaround — see tauri #9394, #15050
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    if let Err(e) = dzpos_desktop::run() {
        // Linux and a dev run keep a console, so this is read there. A
        // Windows release build has none (see the cfg_attr at the top of
        // this file): the message box below is what a shopkeeper sees
        // instead, since this line goes nowhere for them.
        eprintln!("dz-pos failed to start: {e}");
        #[cfg(windows)]
        {
            // `run()` keeps its own `db_path` private and is not being
            // restructured to hand one back with the error; recomputing it
            // is deterministic (dirs::data_dir() reads no state this
            // process changed) and cheap next to a process that is about
            // to exit anyway.
            let db_path = dzpos_desktop::db_path().ok();
            let db_path = db_path.as_deref().map(std::path::Path::new);
            dzpos_desktop::startup_failure::show(e.as_ref(), db_path);
        }
        std::process::exit(1);
    }
}
