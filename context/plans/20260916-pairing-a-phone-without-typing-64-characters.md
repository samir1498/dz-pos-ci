---
title: 'Pairing a phone without typing 64 characters'
slug: 'pairing-a-phone-without-typing-64-characters'
status: 'active'
category: 'feature'
created: 20260916
tldr: 'The desktop has no pairing UI at all, so the phone''s only way in is a hand-typed 64-hex token. Build the QR on the desktop, scan it on the phone with expo-camera (in Expo Go, no dev build), and give the app a splash and an icon.'
priority: 80
tasks:
  - id: 'T1'
    desc: 'Paired phones section in desktop settings: mint QR, render it, count the 60s down, list devices, revoke behind a confirm; owner-gated at the route not just the button'
    status: 'in-progress'
  - id: 'T2'
    desc: 'expo-camera scanner on the phone''s pair route, works in Expo Go; typed box stays as the fallback when permission is refused'
    status: 'pending'
  - id: 'T3'
    desc: 'expo-splash-screen held until SessionProvider.ready, plus app icon and adaptive icon; app.json has none of the three today'
    status: 'pending'
acceptance:
  - 'A phone pairs end to end without anyone reading a token out loud: owner taps Show QR on the desktop, cashier scans it, the phone lands on sign-in'
  - 'A cashier''s session is refused by POST /pairing/qr and the revoke route themselves, proven by a request test, not by the button being hidden'
  - 'Refusing the camera permission still leaves a working pairing path: the typed box renders and claims'
---
# Pairing a phone without typing 64 characters

## What is actually broken

Not the phone screen. `grep -rIl "pairing" apps/desktop/src` returns **zero
files**. The API has had the whole pairing surface since M6 —
`POST /pairing/qr`, `GET /pairing/devices`,
`POST /pairing/devices/{id}/revoke` (`crates/api/src/lib.rs:741-745`) — and
nothing on the desktop calls any of it. There is no QR on a screen anywhere,
so there is nothing for a phone to scan, so the phone's pairing route asks a
cashier to type 64 hexadecimal characters out of a terminal. That is the
whole cause.

`apps/mobile/app/pair.tsx` already says so in its own header. This plan is
that comment coming due.

## Order, and why

The desktop half comes first because the phone half cannot be tested without
it. Each task is a PR; `just gates` between them.

### T1 — Paired phones on the desktop

A section in settings: a button that calls `POST /pairing/qr`, the QR
rendered from the returned `pairing_token`, a visible countdown of
`expires_in_seconds` (60), and the token in a small mono line underneath for
the case where a camera will not focus. Below it, `GET /pairing/devices` as a
`DataTable`, each row with a revoke action behind a confirm.

The countdown is not decoration. A pairing token lives sixty seconds and is
single use, so an owner who mints one and then goes to find the cashier has
already wasted it; the screen has to say that before it happens rather than
after.

QR rendering: check npm for a small library before hand-rolling one — global
rule. It must render to an element the existing theme can size, and it must
not pull a canvas polyfill into the Tauri bundle.

Permission: minting a QR and revoking a device are owner-level. Follow
whatever `settings_.users.tsx` does rather than inventing a second rule, and
the route must refuse a cashier's session, not just hide the button.

### T2 — Scan it on the phone

`expo-camera` is in SDK 57's bundled native modules
(`apps/mobile/node_modules/expo/bundledNativeModules.json`), which means it
works in **Expo Go** — this does not need a dev build, which is what the
old comment in `pair.tsx` assumed.

The scanner becomes the pairing screen. Camera permission is asked for with
a sentence saying why; a refusal falls back to the typed box, which stays
and does not get deleted. A scan reads the 64 hex out of the QR and posts
the same `/pairing/claim` the typed box posts, so there is one code path
below the input.

### T3 — Splash and icon

`app.json` has no `icon`, no `splash`, no adaptive icon. The app opens on a
white flash and appears in the launcher as the default Expo diamond.

`expo-splash-screen`, held deliberately until `SessionProvider.ready` flips,
so the splash covers the disk read instead of a blank frame. The Dinar
wordmark on the Comptoir background, the icon generated at the sizes Android
and iOS want.

## What this plan does not do

- No camera-based barcode scanning at the till. That is a different feature
  with a different failure mode (a wrong product rung vs a phone not
  pairing) and it belongs to its own plan.
- No pairing over anything but the shop's own network. The device gate and
  the launch token stay exactly as they are.

## Checked, not assumed (2026-09-16)

- `apps/desktop/src`: zero files mention pairing.
- `crates/api/src/lib.rs:741-745`: the three routes exist and are mounted.
- `PairingQrDto` is `{ pairing_token: string, expires_in_seconds: number }`.
- `expo-camera ~57.0.5` and `expo-splash-screen ~57.0.9` are both in SDK 57's
  bundled modules, so both run under Expo Go.
