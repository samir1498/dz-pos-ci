# Architecture: how dz-pos is built

Normative. The reasoning behind each rule is in the research repo
(`architecture-notes.md` in the dz-pos competitor folder); this file is
the short form every task cites.

## Stack

| Layer | Choice | Version pinned in |
|---|---|---|
| Core | Rust, diesel + SQLite (bundled) | `crates/core/Cargo.toml` |
| API | Rust HTTP server in `crates/api` (axum), embedded in the desktop, standalone when hosted | `crates/api/Cargo.toml` |
| Desktop | Tauri 2, React 19, Vite, Tailwind 4, TanStack Router/Query | `apps/desktop` |
| Mobile | Expo / React Native | `apps/mobile`, not started |
| Shared TS | `packages/shared`: API client, generated types, money formatting | `packages/shared/package.json` |

Candidates checked on crates.io on 2026-09-07 and not yet chosen: axum
0.8.9 and tokio 1.53.1 for the API; ts-rs 12.0.1 for generating the TS
types; proptest 1.11.0 for property tests; insta 1.48.0 for golden files;
mdns-sd 0.21.2 for LAN discovery. Pin the version in the Cargo.toml when a
crate is first used; this table does not decide.

## The six rules

1. **The mobile app never knows which mode it is in.** One HTTP contract.
   Locally the desktop serves it; hosted, the server serves it. Same routes,
   same shapes. An `if cloud` branch on the phone is the failure condition.
2. **The core sits behind one HTTP service layer from day one.** Tauri
   commands, the browser preview, and the phone are equal callers of
   `crates/api`. Nothing bolts an API on later.
3. **`shop_id` on every table from day one**, even single-shop. Locally it
   is one boring value. Retrofitting a tenant column after there are
   customers is a miserable migration.
4. **One source of truth at any time.** The phone is a thin client with at
   most a retry queue for writes it could not deliver; the server's answer
   always wins. Never two owners of stock counts reconciled later.
5. **In local mode exactly one desktop is the server.** A second till is a
   client like the phone. Decided at pairing time, not at runtime.
6. **Money is integer centimes, never a float.** Rounding rule and where it
   applies: `docs/features.md`, fiscal rules table.

## Consequence of rule 2 nobody has written down yet

The desktop webview talks to a localhost HTTP server that the Tauri process
starts, not to Tauri IPC. Tauri stays for what only it can do: window,
tray, native print dialog, file pickers, auto-update. Two things follow:

- `pnpm desktop dev` in a browser over SSH shows real data as soon as the
  API is running on the box. The IPC gap that made the first preview
  hollow disappears.
- The API crate is the product's only entry point, so its tests are the
  product's tests. UI tests check rendering, not business rules.

## Transport and auth

Who may talk to the core, over what, and how the core knows. Five links,
each decided once here; a milestone that opens a link implements the row
and changes nothing about the others. The core never terminates TLS
itself and never learns a password: the desktop, the pairing flow and the
hosting front door do that, and the core sees a token or a session.

| Link | Transport | Who is calling | Where it is built |
|---|---|---|---|
| Desktop webview to its own core | plain HTTP on 127.0.0.1, port chosen at launch | the launch token: 32 random bytes the Tauri process makes at start, injected into its webview as `__DZPOS_API_TOKEN__` and required as `Authorization: Bearer` on every route but `/health` (and the CORS preflight, which a browser sends bare); the origin list is a second gate a stranger's page has to pass before it can even send the header | M1, `crates/api/src/token.rs` |
| Browser preview and e2e to a standalone core | same, the token comes from `DZPOS_API_TOKEN` in the environment (never a flag: `ps` shows flags) and reaches Vite as `VITE_API_TOKEN`; Vite inlines it into the bundle it serves, and `just dev --host` serves that bundle to every machine on the Tailscale net, so the token is made fresh per `just api` run and is worthless once that run ends | same launch token | M1, `justfile`, `playwright.config.ts` |
| Phone or second till to the serving desktop over the shop LAN | HTTP on the LAN address the owner enabled; TLS with a self-signed certificate whose fingerprint travels in the pairing QR is the candidate, plain HTTP on a trusted Wi-Fi the alternative; open until M6 starts | a device token: the QR carries a 60-second single-use pairing token, the phone trades it for a long-lived device token stored on the phone, revocable from settings; every request shows it the way the webview shows the launch token | M6 |
| Desktop to the hosted core (cloud mode) | HTTPS, terminated at the host's front door, the core behind it on loopback exactly as on a desktop | an account session issued by the host's login; the core receives the shop it answers for and the user from a header the front door sets and a caller cannot | after M6, open decision 1 |
| Phone to the hosted core | same as the desktop link; the phone never knows which mode it is in (rule 1) | same account session | after M6, open decision 1 |

Roles (M4) sit inside every link, not beside it: the token or session says
which device or account is asking, the user on the request says which
person, and the permission list in `docs/features.md` §5 is checked in the
service layer, so a request that skipped the screen still meets it. The
middleware that checks the launch token is the request-identity slot; M4
adds the user to it, it does not add a second gate.

What is out of scope and stays so: the core does not encrypt the SQLite
file (Data, below), does not rate-limit loopback, and does not defend the
machine from software already running as the same user; a keylogger reads
the token from the webview like it reads the PIN.

## Layout

```
crates/core       models → repos (diesel) → services; migrations; errors
crates/api        HTTP routes over services; launch token; shop scoping
apps/desktop      React UI + src-tauri (starts the API, owns the window)
apps/mobile       Expo app, same API client                        (later)
packages/shared   TS: API client, types generated from Rust, money
docs/             this file, features.md, decisions as they land
```

Dependency direction is one way: `api → core`, `desktop → api`,
`shared ← desktop, mobile`. Core never imports Tauri, HTTP, or UI types.

## Layers inside core

- **models**: plain structs, diesel derives, no logic.
- **repos**: one per aggregate, the only place diesel queries live, every
  query scoped by `shop_id`.
- **services**: business rules (stock ledger, totals, TVA, stamp, numbering,
  debt). Take a connection, return domain results. This is where the tests
  concentrate.
- **documents**: invoice rendering to HTML from a snapshot. Pure function
  of (document, template, language); golden-file tested.

## Error policy

Anouar's "no error slop" rule, made concrete:

- One error enum per layer (`DbError`, `ServiceError`, `ApiError`), each
  wrapping the one below with `thiserror`. The API maps `ServiceError`
  variants to HTTP status codes in one place.
- No `unwrap` or `expect` outside tests and `build.rs`. Enforced by
  `clippy::unwrap_used` and `clippy::expect_used` in the workspace
  `[lints]` table, landing with the first service code.
- Every user-facing error has a translation key. The UI never shows a Rust
  or SQL message.
- Money arithmetic uses checked operations; overflow is an error, not a
  wrap.

## Contract between Rust and TypeScript

Types cross the boundary once, generated, never hand-written twice. The
Rust structs in `crates/api` are the source; a build step writes
`packages/shared/src/generated/*.ts`; CI fails if the generated output is
stale. Money crosses the wire as an integer number of centimes in a
`number` (safe below 2^53, which is 90 trillion dinars) and is formatted
only in `packages/shared`.

## Data

- SQLite everywhere. One file per shop, also when hosted. WAL mode,
  foreign keys on, migrations embedded in the binary and applied on open.
- Migrations are forward-only and additive once released. Every migration
  ships with a test that opens a database at the previous version and
  applies it.
- Append-only ledgers (stock movements, debt) are the truth; cached
  balances are derived and re-checked.
- Encryption at rest: OS-level (BitLocker / FileVault) on the shop PC,
  disk encryption on the server; the app does not roll its own. Backups are
  copies of the file, restorable from the settings screen and tested by
  the backup test opening the copy.

## Release

Decided (Samir, 2026-09-08): the version is semver plus the git short hash
plus the build date, all three embedded in the binary at build time and
shown in About. The bundle identifier `com.dzpos.app` changes exactly
once, with the final name, before the first tag (open decision 2 in
`features.md`).

Raised in the 2026-09-08 handoff, still open, each settled before
`docs/roadmap.md` M5 closes:

- Whether the version also heads the log file and the support bundle.
- Migrations tied to the app version, with an automatic backup of the
  SQLite file before any migration runs.
- Tauri updater signing key: who generates it and who holds it; it never
  enters the repo, CI signs with a secret.
- Windows code-signing certificate: cost and lead time, for Anouar.
- Whether a tag on `main` is the only thing that builds the installer and
  publishes the GitHub release.
- A content security policy on the webview (`tauri.conf.json` has
  `csp: null`): the launch token sits in a page global, so one link that
  navigates the main frame to a remote page would hand it over. No screen
  has such a link today; the policy is what keeps it so.

## Testing matrix

| What | Tool | Where it runs |
|---|---|---|
| Services, ledgers, numbering | `cargo test`, integration tests against a temp SQLite | every push, Linux + Windows |
| TVA, stamp, rounding, amount in words | property tests (proptest) + fixed fixtures named in features.md | same |
| Invoice templates | golden files (insta), every template × language | same |
| API routes | request tests against an in-process server and temp DB | same |
| Coverage | `cargo llvm-cov` → lcov artifact; Sonar ingestion once Rust support on the team server is verified | Linux job |
| React components | vitest + Testing Library, jsdom | every push |
| Desktop end-to-end | tauri-driver + WebDriver under `xvfb-run` | later, nightly |
| Mobile | Jest (RN preset), Maestro flows on a real device | later |

Gates already in CI: `cargo fmt --check`, `cargo clippy -D warnings`,
`cargo test`, `cargo llvm-cov`, `pnpm -r build`, `pnpm -r test`. A change
is done when they pass and the behaviour was driven, not when they pass.

## Local development

- Everything runs on the WSL box. Rust and web tests run there directly.
- UI: `pnpm desktop dev` bound to the box's Tailscale address; open it
  from the laptop's browser. Real data once the API runs.
- Native window: `pnpm desktop tauri dev` needs a display (WSLg or
  `xvfb-run`). Windows builds come from CI.
- Phone: Expo Go over Tailscale, EAS for builds.
