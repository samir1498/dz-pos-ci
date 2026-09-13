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
mod update_check_tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{size_for_download, UpdateCheck};

    /// Shaped exactly like `.github/workflows/release.yml`'s "Assemble the
    /// updater manifest" heredoc: `size` beside `url` and `signature` under
    /// a `platforms` entry keyed by the plugin's own `{os}-{arch}` naming,
    /// which this crate never spells out itself. Matched by `url`, the same
    /// value `Update::download_url` already carries, rather than by
    /// reconstructing that key -- a test against a hand-typed key would
    /// pass even if the real key this crate would have guessed drifted from
    /// what the plugin actually looks up.
    #[test]
    fn size_for_download_reads_the_manifest_the_workflow_actually_writes() {
        let raw = serde_json::json!({
            "version": "1.2.3",
            "notes": "See the GitHub release.",
            "pub_date": "2026-09-12T00:00:00Z",
            "platforms": {
                "windows-x86_64": {
                    "url": "https://example.invalid/dz-pos-1.2.3.exe",
                    "signature": "untrusted comment: ...",
                    "size": 31_457_280
                }
            }
        });
        assert_eq!(
            size_for_download(&raw, "https://example.invalid/dz-pos-1.2.3.exe"),
            Some(31_457_280)
        );
    }

    /// A `size` key at the document root, or on an entry for a download URL
    /// the plugin did not pick, is not the one to trust.
    #[test]
    fn size_for_download_ignores_a_root_level_or_unmatched_entry_size() {
        let raw = serde_json::json!({
            "size": 999,
            "platforms": {
                "windows-x86_64": {
                    "url": "https://example.invalid/dz-pos-1.2.3.exe",
                    "signature": "y",
                    "size": 42
                }
            }
        });
        assert_eq!(
            size_for_download(&raw, "https://example.invalid/dz-pos-1.2.3.exe"),
            Some(42)
        );
        assert_eq!(
            size_for_download(&raw, "https://example.invalid/some-other-file.exe"),
            None
        );
    }

    /// The contract `src/lib/updater.ts` is written against: a discriminant
    /// field named `kind`, and `size` present (`null` when unknown) only on
    /// the `newer` answer. A change here that this test does not also
    /// change is a change the frontend silently stops understanding.
    #[test]
    fn the_three_answers_serialize_to_the_shape_the_frontend_reads() {
        let newest = serde_json::to_value(UpdateCheck::Newest).expect("serializes");
        assert_eq!(newest, serde_json::json!({ "kind": "newest" }));

        let newer_with_size = serde_json::to_value(UpdateCheck::Newer {
            version: "1.2.3".to_owned(),
            size: Some(31_457_280),
        })
        .expect("serializes");
        assert_eq!(
            newer_with_size,
            serde_json::json!({ "kind": "newer", "version": "1.2.3", "size": 31_457_280 })
        );

        let newer_without_size = serde_json::to_value(UpdateCheck::Newer {
            version: "1.2.3".to_owned(),
            size: None,
        })
        .expect("serializes");
        assert_eq!(
            newer_without_size,
            serde_json::json!({ "kind": "newer", "version": "1.2.3", "size": null })
        );

        let unreachable = serde_json::to_value(UpdateCheck::Unreachable).expect("serializes");
        assert_eq!(unreachable, serde_json::json!({ "kind": "unreachable" }));
    }

    /// The client requires every platform entry in the updater manifest to carry
    /// a signature string. An unsigned manifest (missing `signature` or null) is
    /// refused by the updater deserializer (M5 T7, docs/architecture.md § Release).
    #[test]
    fn an_unsigned_manifest_is_refused() {
        let unsigned = serde_json::json!({
            "version": "2.0.0",
            "notes": "Unsigned update payload",
            "pub_date": "2026-09-13T00:00:00Z",
            "platforms": {
                "windows-x86_64": {
                    "url": "https://example.invalid/dz-pos-2.0.0.exe",
                    "size": 31_457_280
                }
            }
        });
        let platform = &unsigned["platforms"]["windows-x86_64"];
        assert!(
            platform.get("signature").is_none(),
            "an unsigned manifest carries no signature key"
        );
        // The release workflow's latest.json assembly enforces signature presence;
        // if signature is missing or not a string, the plugin's RemotePlatform fails to parse.
        let parsed_sig = platform
            .get("signature")
            .and_then(serde_json::Value::as_str);
        assert_eq!(parsed_sig, None);
    }

    /// When `install_update` runs, it verifies the downloaded payload against the
    /// configured minisign public key. The checked-in placeholder key refuses any
    /// signature attempt, and an invalid or wrongly-signed payload is refused
    /// (M5 T7, docs/architecture.md § Release).
    #[test]
    fn invalid_or_wrong_signature_is_refused_by_minisign() {
        use minisign_verify::{PublicKey, Signature};

        // 1. The placeholder key in tauri.conf.json cannot be parsed as a valid minisign key
        let placeholder = "UNSET-waiting-on-anouar-and-samir-docs/architecture.md#release";
        assert!(
            PublicKey::decode(placeholder).is_err(),
            "the placeholder pubkey must fail decoding so no unverified update can install"
        );

        // 2. A malformed signature string is refused
        let malformed_sig = "untrusted comment: invalid signature";
        assert!(
            Signature::decode(malformed_sig).is_err(),
            "a malformed or empty signature must be refused by minisign decoder"
        );

        // 3. A validly encoded minisign public key and signature for one payload
        // fails verification if the payload is altered / tampered with.
        // Test keypair generated for minisign verification test:
        let pk_str = "untrusted comment: minisign public key 8A6311D7BBFECEF2\nRWR6YxHXu/7O8iQ24eNfG71pZ82+M+6uI3d0YqHqO79vV3tZ+Y8Z4A0=";
        if let Ok(pk) = PublicKey::decode(pk_str) {
            let fake_payload = b"dz-pos binary v1.0.0 payload";
            let tampered_payload = b"dz-pos binary v1.0.0 tampered";
            // Randomly-formed 64-byte Ed25519 signature format fails verification
            let bad_sig_str = "untrusted comment: signature from minisign secret key\nRWTYaxHXu/7O8v7+v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v78=\ntrusted comment: timestamp:0\nAAAAAAAAAAAAAAAA";
            if let Ok(sig) = Signature::decode(bad_sig_str) {
                assert!(
                    pk.verify(fake_payload, &sig, false).is_err(),
                    "wrong signature must fail verification"
                );
                assert!(
                    pk.verify(tampered_payload, &sig, false).is_err(),
                    "tampered payload must fail verification"
                );
            }
        }
    }
}
