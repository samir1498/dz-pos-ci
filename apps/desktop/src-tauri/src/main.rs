// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKitGTK Wayland workaround — see tauri #9394, #15050
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    if let Err(e) = dzpos_desktop::run() {
        eprintln!("dz-pos failed to start: {e}");
        std::process::exit(1);
    }
}
