// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::Value;

/// Named so a real key generated later cannot be mistaken for this:
/// anything that does not spell this exact sentinel fails the test
/// below, which is what keeps a look-alike string (`"changeme"`, a
/// blank one, a key pasted in without updating this constant) from
/// shipping as if it were real. `docs/release-checklist.md` names the
/// key itself as waiting on Anouar and Samir; the day it exists, this
/// constant is what changes.
const PLACEHOLDER_PUBKEY: &str = "UNSET-waiting-on-anouar-and-samir-docs/architecture.md#release";

fn config() -> Value {
    let raw = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tauri.conf.json"));
    serde_json::from_str(raw).expect("tauri.conf.json must parse as JSON")
}

#[test]
fn the_updater_key_is_still_the_named_placeholder() {
    let config = config();
    let pubkey = config["plugins"]["updater"]["pubkey"]
        .as_str()
        .expect("plugins.updater.pubkey must be a string");
    assert_eq!(
        pubkey, PLACEHOLDER_PUBKEY,
        "the checked-in pubkey no longer matches the named placeholder; if the real \
         key has arrived, update PLACEHOLDER_PUBKEY to match it rather than deleting \
         this assertion"
    );
}

#[test]
fn every_updater_endpoint_is_https() {
    let config = config();
    let endpoints = config["plugins"]["updater"]["endpoints"]
        .as_array()
        .expect("plugins.updater.endpoints must be an array");
    assert!(
        !endpoints.is_empty(),
        "at least one updater endpoint must be configured"
    );
    for endpoint in endpoints {
        let url = endpoint.as_str().expect("each endpoint must be a string");
        assert!(
            url.starts_with("https://"),
            "endpoint '{url}' is not https; a build shipped with this would trust \
             whatever answered on plain http (docs/architecture.md § Release)"
        );
    }
}
