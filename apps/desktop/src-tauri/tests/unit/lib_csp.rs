// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use tauri::test::mock_builder;

fn served_index_html_csp() -> String {
    let app = mock_builder()
        .build(tauri::generate_context!())
        .expect("the mock app must build; run `pnpm --filter dzpos-desktop build` first");
    app.asset_resolver()
        .get("index.html".to_owned())
        .expect("index.html must be an embedded asset")
        .csp_header
        .expect("index.html must carry a Content-Security-Policy")
}

#[test]
fn the_served_page_carries_the_policy_and_allows_no_eval_or_remote_origin() {
    let csp = served_index_html_csp();
    assert!(csp.contains("default-src 'self'"), "{csp}");
    assert!(!csp.contains("unsafe-eval"), "{csp}");
    assert!(
        !csp.contains("https://"),
        "no directive names a remote origin: {csp}"
    );
    assert!(csp.contains("frame-ancestors 'none'"), "{csp}");
    assert!(csp.contains("object-src 'none'"), "{csp}");
    assert!(csp.contains("form-action 'self'"), "{csp}");
    assert!(csp.contains("base-uri 'self'"), "{csp}");
    assert!(
        csp.contains("connect-src 'self' http://127.0.0.1:* ipc://localhost http://ipc.localhost"),
        "connect-src must allow both shapes invoke() actually fetches: \
         `ipc://localhost/<cmd>` on Linux and macOS, `http://ipc.localhost/<cmd>` \
         on Windows and Android (wry's custom-protocol workaround, \
         use_https_scheme defaults to false) -- without both, the very \
         first launch_token() call is a CSP violation on whichever \
         platform is missing: {csp}"
    );
    // Tauri hashes every inline <script>/<style> found in the built
    // index.html at compile time and appends the hash here (`csp_hashes`)
    // instead of `'unsafe-inline'`; this is what lets the lang/theme
    // flash-prevention IIFE in index.html run at all under this policy.
    // Asserting the hash is present, not just that the assertion above
    // passed, is the difference between "the script runs" and "the
    // script silently never ran because the hash never landed".
    assert!(
        csp.contains("script-src 'self' 'sha256-"),
        "the inline lang/theme script must be hash-allowed: {csp}"
    );
}
