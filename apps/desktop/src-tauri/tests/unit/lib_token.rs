// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{launch_token, TokenHandoff};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, MockRuntime, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{Manager, Webview};

/// Calls the real `launch_token` command over the same IPC path the
/// webview uses, rather than reading the struct's field directly: the
/// point of this test is that the command answers, not that a `String`
/// can hold a string. Generic over anything `get_ipc_response` itself
/// accepts (`WebviewWindow` included), the same bound it declares.
fn ask<W: AsRef<Webview<MockRuntime>>>(webview: &W) -> String {
    let request = InvokeRequest {
        cmd: "launch_token".into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: "tauri://localhost".parse().expect("a fixed URL parses"),
        body: InvokeBody::default(),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    };
    get_ipc_response(webview, request)
        .expect("launch_token must answer, not refuse")
        .deserialize::<String>()
        .expect("launch_token answers a JSON string")
}

#[test]
fn every_caller_gets_the_same_token_over_the_real_ipc_command() {
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![launch_token])
        .build(tauri::generate_context!())
        .expect("the mock app must build; run `pnpm --filter dzpos-desktop build` first");
    app.manage(TokenHandoff("the-token".to_owned()));
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("a plain window with no on_navigation still builds for this test");

    assert_eq!(ask(&webview), "the-token");
    assert_eq!(ask(&webview), "the-token");
}
