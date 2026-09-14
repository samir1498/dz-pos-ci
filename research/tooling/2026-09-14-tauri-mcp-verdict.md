# Tauri MCP verdict — 2026-09-14 (hypothesi `mcp-server-tauri`)

Trial: one run on the laptop (`fedora` @ `100.111.55.62`) with a debug build of Dinar (`dzpos-desktop`) running in `session.slice` (the same `systemd-run --user` + `zsh -lic "pnpm exec tauri dev"` used for the manual till checks). Checked `hypothesi/mcp-server-tauri` + `@hypothesi/tauri-mcp-cli` and the `tauri-plugin-mcp-bridge` Rust plugin that must be compiled into debug builds.

What it does: exposes the Tauri webview as an MCP server (screenshots, DOM, console, IPC `invoke` monitoring, webview automation) so an agent can drive the native window without HTTP.

Verdict: **not adopted for dz-pos.**

- Architecture already makes it redundant: `crates/api` is the product's only entry point (`crates/api` axum over `crates/core`), the webview talks to it over `http://127.0.0.1:4317` with a launch token, and `pnpm desktop dev` (Vite 5173) is the same app without Tauri. Playwright against `just dev` + `just api` covers the screens, the fiscal flows, and the permission refusals (`till-cashier.spec.ts` proves every 403 by code, not just status) without a plugin in the binary or a display.
- Cost: `tauri-plugin-mcp-bridge` must be in `apps/desktop/src-tauri/Cargo.toml` for debug builds only, plus a running window on the laptop, plus a `pc-ctx` binding for the native side. The earlier manual window checks already needed `systemd-run --user --unit=dinar-tauri -p Slice=session.slice` to keep the focus, and the bridge would add a second MCP server to that story.
- The hypothesis trial (verdict in `research/tooling/2026-09-08-official-skills-and-mcps.md`) predicted this: "Playwright against `pnpm desktop dev` covers most of what the MCP bridge would, without a plugin". Trial confirms.

Keep the `laptop-dev` skill's `systemd-run` + `zsh -lic` path for the rare native-window checks (updater, CSP, `startup_failure.rs` message box). Re-trial only if a bug can only be reproduced inside the webview (e.g., `tauri-plugin-updater` IPC) and not over HTTP.
