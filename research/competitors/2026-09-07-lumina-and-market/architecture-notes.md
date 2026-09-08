# dz-pos — architecture notes

Working notes from the 2026-09-07 discussion. These are decisions and rules,
not a design doc. The SaaS-vs-offline question is still open with Anouar; the
point of these rules is that the answer becomes a deployment choice rather than
a rewrite.

## Stack

| Layer | Choice | Why |
|---|---|---|
| Core | `monstock-core`, Rust | Already exists, 8k LOC, clean repo/service layering. Wails/Go would mean porting it for no gain. |
| Desktop | Tauri, TS/React UI | egui is a nightmare to style, and RTL + HTML invoice rendering are nearly free in a webview. `monstock-desktop` (egui) gets dropped. |
| Mobile | Expo / React Native | Samir's call; no Tauri mobile. Developed against a real phone over Tailscale, built via EAS. |
| Local DB | SQLite | Already in place. |
| Dev CLI | Gut to `seed` only | `dev`/`build` are thin wrappers over `cargo tauri dev/build` once egui is gone. Keep `seed` (291 working lines you'll want daily). Revisit `doctor` if a customer ever can't print. |

## The rules

1. **The mobile app never knows which mode it is in.** It speaks one HTTP
   contract. In local mode the desktop serves it; in cloud mode the hosted
   server serves it. Same routes, same shapes. An `if cloud` branch on the phone
   is the failure condition.

2. **`monstock-core` sits behind one HTTP service layer from day one.** Tauri
   commands and the mobile app are equal callers. Do not bolt an API on later.

3. **`shop_id` in the schema from day one, even local.** Locally it is one boring
   value. Retrofitting a tenant column across every table and query after there
   are customers is a miserable migration. Costs nothing today.

4. **One source of truth at any time.** The phone is a thin client with at most
   a retry queue for writes it could not deliver; the server's answer always
   wins. A desktop and a phone must never both own stock counts while offline
   and reconcile later — that is Lumina's `sync/push` + `sync/pull` pair, and it
   is where their bugs live. One-directional is a week of work; bidirectional
   is months and a bug class that never closes.

5. **In local mode exactly one desktop is the server.** A second till is a
   client like the phone. Decided at pairing time, not at runtime.

6. **Money is integer centimes, never `REAL`.** MonStock's schema currently has
   `cost_price REAL`, `total REAL`, `line_total REAL`. Migrate before writing
   any fiscal code; property-test the rounding on TVA and stamp duty.

## Local mode connection

- Server advertises itself via mDNS (`mdns-sd` crate — pure Rust, no system
  dependency, 4.8M downloads, released 2026-09-05). Clients discover it; nobody
  ever types an IP. Lumina still has a manual server-IP field with
  auto-discovery as a toggle; we can just not have that field.
- Phone pairs by scanning a QR the desktop shows: host + port + short-lived
  pairing token.
- The thing that actually breaks LAN setup in the field is Windows Firewall
  silently blocking the listener on first run. Detect it and either add the
  rule at install or show one clear instruction.

## Cloud mode

- **Shared multi-tenant server, not per-customer instances.** At Lumina's price
  point (12,000 DZD one-time) per-customer containers eat the margin and the
  ops time.
- **Database: SQLite everywhere, one file per shop on the server.** The
  alternative — SQLite local, Postgres hosted — is two backends behind the repo
  layer, every query tested twice, and the "same code both places" promise
  quietly dies. These are tiny shops; a file per tenant is trivially isolated,
  backup is `cp`, and the exact same core binary runs in both modes. This also
  answers "one server for all users or per user": one server, one file per
  shop, which is both. Keep `shop_id` anyway as cheap insurance.
- Auth is email/password on an account. Desktop asks once at first run — "this
  shop, or my account" — and never again.
- **Phone-only online falls out for free.** A shop with a phone and a Bluetooth
  thermal printer logs in and never owns a desktop. Lumina cannot serve that
  customer; their desktop is mandatory.

## Phone-only offline — parked

Rejected for now, but the viable path is recorded so nobody reaches for the
wrong one later.

- **Do not** give the RN app its own local SQLite with business logic in TS.
  That is a second implementation of stock ledger, TVA, stamp, rounding and
  amount-in-words that must match the Rust one to the centime. The first time
  they disagree a shop prints a wrong facture and nobody knows which side is
  right.
- **The path, if it is ever a real segment:** compile `monstock-core` into the
  RN app as a native module via `uniffi` (Mozilla, v0.32, 12M downloads).
  Rust cross-compiles to `aarch64-linux-android` and `aarch64-apple-ios`;
  SQLite builds on both; uniffi generates the Kotlin and Swift bindings; an
  Expo Module wraps them. This is still React Native — the UI never changes.
  Costs: Expo Go is replaced by `expo-dev-client` builds, Android NDK plus a
  Mac (EAS covers it) for cross-compiling, a few hundred lines of Expo Module
  wrapper. iOS will not let a backgrounded app serve other devices, so a phone
  can run the core for itself but cannot be the shop's LAN server.
- Even then, rule 4 holds. A phone running the core offline is the phone-only
  shop's whole system, or it is a thin client. Never both.

## Build order

Local mode first, cleanly, behind the API. Cloud becomes a second deployment of
code that already exists.

customers + debt ledger → invoice data model → print engine → users/roles →
Arabic/RTL → LAN multi-device → (cloud, when Anouar decides)

Money-type migration before any of it.

## Testing — what the JS-only list missed

- **Rust:** `cargo test` plus `cargo-llvm-cov` (v0.9.1) for lcov that Sonar can
  ingest. This is where the money logic lives.
- **Sonar and Rust:** unverified on sonar.observeone.com — probing `rust:S1481`
  returned 404, which is not conclusive. Check before promising a gate on the
  core.
- **Tauri E2E:** `tauri-driver` + WebDriver. Maestro/Detox only cover the phone.
  Weakest part of the Tauri story.
- **Golden-file tests for invoice templates.** Highest priority of the lot. The
  facture is the product; render each template against fixtures and diff. A
  wrong field on a printed facture is a legal problem, and no UI test catches it.
- **Rust ↔ RN contract:** generate the TS types from the Rust structs, or they
  drift the first week.
- Mobile: Maestro (simpler, works with Expo dev builds) over Detox. Jest with
  the RN preset — Vitest and RN do not get along. pnpm works with Metro if
  `.npmrc` has `node-linker=hoisted`.
