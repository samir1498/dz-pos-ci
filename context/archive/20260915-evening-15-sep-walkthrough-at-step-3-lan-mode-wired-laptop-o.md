---
title: 'Evening 15 Sep: walkthrough at step 3, LAN mode wired, laptop offline, phone pairing is next'
slug: 'evening-15-sep-walkthrough-at-step-3-lan-mode-wired-laptop-o'
status: 'active'
category: 'handoffs'
created: 20260915
tldr: 'Branch green through 187a31c (PR #93 open), LAN flag wired for the phone, laptop dropped offline with Expo state unknown; pair the phone first thing tomorrow'
---
# Handoff — evening 2026-09-15: walkthrough paused at step 3, phone pairing is next, laptop went offline

## Where everything is

- Branch `fix/first-setup-validation`, pushed through `187a31c`, PR #93 open and unmerged. `main` is red on `tsc` (T4 fallout); this branch is green. Merge priority.
- What's in the branch, oldest first: first-setup hint + enabled button (`3d8c35c`), settings `min-w-0` unclip, sidebar `collapsible=icon`, dev session persist in `localStorage`, till sends one `idempotency_key` per press, `hidden` (not `sr-only`) import file input + kit warning comment, products filter bar on one line + regenerated `products.png`, API `--lan` flag + `just api ... lan`, shared/till test key fixes.
- Gates on the branch: `just clippy` clean, `cargo test --workspace` green (incl. new `lan_mode_is_opt_in`), desktop 527/527, shared 344/344, products e2e fr green. Landing `shots.test.ts` 5 webp failures are pre-existing on the clean tree (proven via stash) — environment goldens, not ours.

## The walkthrough (manual, laptop, step-by-step)

Done: step 1 sign-in as owner Samir (EN), step 2 added and activated Café (barcode 2000010000012, 100.00, 19%, stock 20), step 3 sold one café on the desktop till.
Next: step 4 pair the phone. Blocker found and fixed today: the dev API only bound loopback, so no phone could reach it. `--lan` now exists (`bind_lan` + mDNS `Dinar-1`, decided posture per architecture.md Transport and auth). Laptop API restarted in LAN mode on the same `dev.db` (Café + sale intact), `ss` shows `0.0.0.0:4317`, health answers over Tailscale with shop_id 1.

## Laptop state (fedora, 100.111.55.62) — OFFLINE as of ~22:00, last seen 5 min prior

tmux `dz` windows api/dev/mobile. Before it dropped: api in LAN mode on `187a31c`, dev serving desktop, mobile/Metro state UNKNOWN (restart attempts hit wrong package filter `mobile` vs real name `dinar-mobile`, which has no `start` script; last try `cd apps/mobile && npx expo start` timed out with SSH).
First thing tomorrow: wake the laptop, confirm SSH, `tmux capture-pane` on all three windows.
Expo restart recipe (run ON the laptop, expand the token there, never locally): `cd apps/mobile && EXPO_PUBLIC_API_URL=http://192.168.0.134:4317 EXPO_PUBLIC_API_TOKEN=$(cat ../../.dev/api-token) REACT_NATIVE_PACKAGER_HOSTNAME=192.168.0.134 npx expo start`. LAN IP `192.168.0.134`, firewall 4317/tcp already open, avahi active.
Pairing: `POST /pairing/qr` as owner (cookie login via curl, launch token from laptop `.dev/api-token`), 60s TTL — mint only when the phone shows the pair screen, type the 64-hex by hand. Phone signs in as owner with password (no cashier in dev DB).

## Gotchas learned today (do not relearn)

- A stale laptop browser tab shows yesterday's JS and no fix lands in it: close the tab, fresh one, Ctrl+Shift+R.
- Dev persist needs ONE fresh sign-in after the fix to seed storage; incognito starts empty; long idle expiry is the server, correct; use `http://127.0.0.1:5173` not `localhost` so the cookie flows.
- `sr-only` on a kit `Input` loses to `w-full` and makes a full-width invisible box (the settings scrollbar saga). `hidden` instead; warning comment on `ui/input.tsx`.
- A launch token leaked into tmux scrollback today; it rotates on the next `just api` restart, no action needed.
- Open questions from Samir, untouched: "For sale" label reads like a discount (it means active; FR/AR fine, EN confusing) and the TanStack Table question (answered: hand-rolled DataTable, migrate only for sorting/paging).

## Tomorrow

1. Wake laptop, verify SSH + tmux trio, restart Expo per recipe, finish step 4 pairing, continue walkthrough (sell, offline retry, revoke).
2. Video plan `demo-video-paired-phone-proven` T1–T4 is ready when the walkthrough is green.
3. Merge PR #93 (dz-review first: touches money-adjacent till key, roles untouched, no deletions).
4. Report standup 15 Sep is expanded with 3 shots and deployed; production sits behind Access, verify via preview URLs.

