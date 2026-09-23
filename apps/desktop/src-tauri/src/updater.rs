//! The shop-initiated update check (M5 T7, docs/architecture.md §
//! Release). Nothing in this file runs on a timer: `check_for_update` only
//! ever runs because the About screen's own button was clicked, since a
//! shop on a phone connection did not agree to thirty megabytes moving on
//! its own, and `install_update` only ever runs because the shop then
//! chose to install what the check found. Both are plain Tauri commands,
//! the same shape `launch_token` in `lib.rs` already uses, rather than the
//! plugin's own IPC commands: the page never learns Tauri's updater API
//! exists, only that these two calls answer.

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

/// The three answers named in docs/architecture.md § Release: already on
/// the newest version, a newer one exists, or the endpoint could not be
/// reached. `Unreachable` is a value here and not a rejected promise,
/// because a shop with no internet today is not an error the About screen
/// needs to explain twice.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UpdateCheck {
    Newest,
    Newer {
        version: String,
        /// The plugin's own manifest carries no byte count until a
        /// download is already running (`Update::download`'s
        /// `content_length`, read off the response after the shop has
        /// already said yes), so this is only ever `Some` because the
        /// release workflow writes an extra `size` key into `latest.json`
        /// itself. `raw_json` is the server's manifest exactly as it
        /// answered, unknown keys included, which is the only reason this
        /// field can exist at all without the plugin knowing about it.
        size: Option<u64>,
    },
    Unreachable,
}

/// The release workflow writes `size` beside `url` and `signature` on each
/// entry under `platforms` in `latest.json` (`.github/workflows/release.yml`'s
/// "Assemble the updater manifest" step). Matched here by `url` rather than
/// by a `platforms` key spelled out by hand: `Update::target` is the plain
/// operating system (`"windows"`, set from the `{{target}}` the endpoint URL
/// itself was templated with), but the `platforms` key the plugin actually
/// looked up to pick this update (`tauri_plugin_updater::Updater::get_urls`)
/// is `{os}-{arch}` or `{os}-{arch}-{installer}` -- a different string this
/// crate would otherwise have to reconstruct and could get wrong the day the
/// arch or installer naming changes upstream. Matching on `download_url`
/// instead reads back the exact entry the plugin already chose, so it stays
/// correct without knowing that naming at all. `raw_json` is the server's
/// manifest exactly as it answered, unknown keys included, which is the
/// only reason a field the plugin itself does not define can be read back.
fn size_for_download(raw: &serde_json::Value, download_url: &str) -> Option<u64> {
    raw.get("platforms")?
        .as_object()?
        .values()
        .find(|platform| {
            platform.get("url").and_then(serde_json::Value::as_str) == Some(download_url)
        })?
        .get("size")?
        .as_u64()
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> UpdateCheck {
    let Ok(updater) = app.updater() else {
        return UpdateCheck::Unreachable;
    };
    match updater.check().await {
        Ok(Some(update)) => {
            let size = size_for_download(&update.raw_json, update.download_url.as_str());
            UpdateCheck::Newer {
                version: update.version,
                size,
            }
        }
        Ok(None) => UpdateCheck::Newest,
        Err(_) => UpdateCheck::Unreachable,
    }
}

/// Re-checks rather than trusting a version string handed back from the
/// page: the two calls are seconds apart at most, and a stale `Update`
/// cached between them would need its own shared state (`Arc<Mutex<...>>`)
/// for a saving that never matters here. Downloads, verifies against the
/// configured `pubkey`, then installs. On Windows -- the only platform
/// that ships today -- the plugin launches the installer and calls
/// `std::process::exit(0)` itself once that launch succeeds
/// (`Update::install_inner`); `download_and_install` never returns on that
/// path, so `app.request_restart()` below is unreached there, and
/// `RunEvent::Exit` never fires either, which is why the empty closure
/// passed as `on_before_exit` is the only hook this crate has a chance to
/// run before the process ends -- nothing needs it today (SQLite is
/// crash-safe, and `crates/core::services::backup::before_upgrade` runs
/// again on the next launch regardless). `request_restart` (over
/// `restart`, which skips `RunEvent::Exit` and the axum task's own cleanup
/// in `lib.rs`'s `run` handler along with it) is what would run on Linux
/// or macOS, where the plugin returns instead of exiting -- dead code
/// until either ships, kept because the alternative is restarting neither.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no update to install".to_owned())?;
    update
        .download_and_install(|_chunk_len, _content_len| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.request_restart();
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/updater.rs"]
mod update_check_tests;
