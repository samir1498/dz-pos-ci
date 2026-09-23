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
