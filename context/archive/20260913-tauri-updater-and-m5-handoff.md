---
title: 'Handoff 2026-09-13: Tauri updater (M5 T7) and next milestone plans'
slug: '20260913-tauri-updater-and-m5-handoff'
status: 'active'
category: 'handoffs'
created: 20260913
tldr: 'M5 T7 (Tauri updater) finished and verified with quality gates on feat/a-shop-can-take-the-next-version; next is PR merge, M5 T8 rename to Dinar, and M5 T9 sweep'
---

# Handoff — 2026-09-13: Tauri Updater (M5 T7) & Next Steps

This handoff documents the completion of **M5 T7** (Tauri updater) and the state of the repository for the next session.

---

## 1. Where Things Are

- **Active branch / worktree**: `feat/a-shop-can-take-the-next-version` at `.claude/worktrees/updater`
- **Main repo checkout**: `/home/samir/dz-pos` on branch `docs/the-date-boxes-are-built-now`
- **Quality gates**: `just gates` (`fmt`, `lint`, `clippy`, `types-check`, `test`, `build`) passing cleanly across the workspace.
- **Context store**: Managed via `pc-ctx` / `just ctx`. Current active milestones: `m5-first-release-v1` (12/15 tasks done before T7 close), `m1-sale-and-ticket-on-one-desktop` (8/9 done, T6 blocked on hardware).

---

## 2. Completed in M5 T7: Tauri Updater

The Tauri updater implementation covers the updater endpoint, signature verification, user-initiated update flow, release manifest assembly, and gating.

1. **User-Initiated Flow (About Screen)**:
   - Button on Settings > About (`UpdateCheckPanel` in `apps/desktop/src/routes/settings_.about.tsx`).
   - Checks are strictly on-demand (no background polling, no timers).
   - Three possible responses: `newest`, `newer { version, size }`, or `unreachable` (network issue handled gracefully as an alert, not a crash).
   - Install requires explicit modal confirmation from the user, warning that the application will restart.
   - Tested in `apps/desktop/src/routes/settings_.about.test.tsx` (6 unit tests).

2. **Backend Tauri Commands & Minisign Verification (`apps/desktop/src-tauri/src/updater.rs`)**:
   - `check_for_update` and `install_update` exposed as custom IPC commands (not exposing raw updater plugin IPC directly).
   - Manifest size extracted from `raw_json` using matched download URL.
   - **Client signature refusal tests added**:
     - `an_unsigned_manifest_is_refused`: Proves that a manifest missing a platform signature cannot be parsed as a valid update.
     - `invalid_or_wrong_signature_is_refused_by_minisign`: Proves that the placeholder pubkey cannot decode as a valid minisign key (preventing premature execution), and malformed or tampered signatures fail verification.

3. **Release Workflow & Gates (`.github/workflows/release.yml` & `.github/scripts/release-gate.sh`)**:
   - Gate script accepts `UPDATER_KEY_PRESENT` (`secrets.TAURI_SIGNING_PRIVATE_KEY`).
   - Emits `updater=publish` only for official tag releases with key present; emits `updater=skip` on dry-runs or when missing.
   - The release workflow only assembles `latest.json` when `updater=publish`. Releases without a signing key ship installers but skip the updater manifest to prevent existing installs from trusting unsigned updates.
   - Verified by 18 test cases in `.github/scripts/release-gate.test.sh`.

---

## 3. Immediate Next Steps for T7

1. **Commit the remaining test additions in the updater worktree**:
   ```bash
   cd /home/samir/dz-pos/.claude/worktrees/updater
   git add apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/src/updater.rs Cargo.lock
   git commit -m "test(updater): verify unsigned manifests and wrong signatures are refused

   ctx: m5-first-release-v1/T7 close"
   ```
2. **Push and create PR**:
   - Push `feat/a-shop-can-take-the-next-version` to origin.
   - Open PR to `main` and run CI / mirror check.
3. **Update context plan**:
   - Mark `T7` as `done` in `context/plans/20260908-m5-first-release-v1.md`.
   - Update `context/progress/daily.md` with completion entry.
   - Once merged, clean up worktree using `just worktree-rm updater`.

---

## 4. Next Plans & Tasks in the Pipeline

### M5 Tasks Remaining
- **M5 T8: The Name, Once ("Dinar")** — **UNBLOCKED**:
  - Samir confirmed the product name is **Dinar** (see `context/references/20260913-product-name-dinar.md`).
  - Scope: Single atomic commit renaming bundle identifier `com.dzpos.app`, window/installer titles, landing page copy, sharing card, and docs placeholder.
- **M5 T9: Closing Sweep**:
  - Sync documentation (`docs/features.md`, `docs/architecture.md`).
  - Update `docs/release-checklist.md`.
  - Perform `dz-review` over the milestone diff (with money and deletion lenses).
  - Update status site and prepare checkpoint PR.

### Other Milestones
- **M1 T6 (Hardware-blocked)**: Thermal printer ESC/POS printing over USB stays blocked (no physical hardware, see `context/references/20260913-no-thermal-printer-for-m1-t6.md`). Do not attempt to code this.
- **M6 (Mobile / Phone in Shop)**: Scheduled after M5 v1.0 release (LAN mode, QR pairing, Expo thin client).

---

## 5. Tooling & Environment Notes

- **Build Directory**: Shared `.cargo-target` with file lock (`flock`) across checkouts. Always run commands through `just` or ensure `CARGO_TARGET_DIR` is set.
- **Antigravity MCP**: `~/.gemini/config/mcp_config.json` is currently empty. The CLI operates directly using `just ctx`. To enable pc-ctx tools in Antigravity sessions, add the stdio server config to `~/.gemini/config/mcp_config.json`.
