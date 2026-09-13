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
| Design tokens | `packages/design`: primitives, semantic roles, CSS output | `packages/design/package.json` |

In use and pinned in their Cargo.toml: axum and tokio for the API, ts-rs
for the TS types, proptest for property tests. Checked on crates.io on
2026-09-07 and not yet chosen: mdns-sd 0.21.2 for LAN discovery. T5 needed
golden files before that insta decision was made and hand-rolled them
instead (`crates/core/tests/print_ticket.rs`, `UPDATE_GOLDENS=1`); insta
stays unpinned unless a later template makes the hand-rolled version
outgrow itself. Pin the version when a crate is first used; this
table does not decide.

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

- `just api` then `just dev` shows real data in a browser on another
  machine, launch token included. The IPC gap that made the first preview
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

Roles sit inside every link, not beside it. The token or session says which
device or account is asking; the session also says which person. The launch
token still answers first and is unchanged: it says the request came from
this machine's own screen. Behind it, `crates/api/src/session.rs` reads a
session token, the `x-dzpos-session` header or the httpOnly `dzpos_session`
cookie a browser was given, and refuses with 401 `session_required` when
there is none. Two credentials, two answers, and a refused permission is a
third thing again: 403 `forbidden` naming the permission.

The permission is checked in one place and not in each handler. The same
middleware that resolved the session looks the route up in
`crates/api/src/gates.rs`, a table with one row per route saying which
permission that route wants and why, and refuses before the handler runs.
So no handler names a permission, no handler can forget to, and a route
added without a row does not quietly inherit one. It fails closed instead: a
POST, PUT, PATCH or DELETE on a route the table does not name is refused
with `ungated_write`, and a test that parses the router's own source walks it
against the table in both directions so that refusal never reaches a shop.
The single statement of who may do what is `can(role, permission)` in
`crates/core/src/services/permissions.rs`; the API's table says which
permission, never which role.

Two kinds of rule cannot live in that table and stay in the service layer,
where they belong. The first is a rule about the request's contents rather
than its route: a discount is free up to the shop's threshold and needs a
permission above it, so the check happens where the basket is priced. The
second is a rule about a row: the shop's last active owner cannot be
switched off, which is a fact about the `users` table, not about who is
asking. A screen never decides either. `apps/desktop` hides what a person
cannot use, and the server refuses it independently; the hiding is
courtesy, the refusal is the control.

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
packages/design   TS: design tokens, three tiers
docs/             this file, features.md, roadmap.md
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
- **print** (`crates/core/src/print/`): rendering to HTML from a stored
  snapshot, one module per template (`ticket.rs`, `facture.rs`,
  `statement.rs`, `debt_slip.rs`). Pure function of its input, the template
  and the language; golden-file tested. The words it prints are its own
  dictionary (`print/strings.rs`), never the desktop's i18n files, so a
  server with no UI prints the same paper.

## Error policy

Anouar's "no error slop" rule, made concrete:

- One error enum per layer (`CoreError` in `crates/core`, wrapping
  `DbError` and `MoneyError`; `ApiError` in `crates/api`), each wrapping
  the one below with `thiserror`. The API maps `CoreError` variants to
  HTTP status codes in one place (`crates/api/src/error.rs`).
- No `unwrap` or `expect` outside tests and `build.rs`. Enforced by
  `clippy::unwrap_used` and `clippy::expect_used` in the workspace
  `[lints]` table.
- Every user-facing error has a translation key. The UI never shows a Rust
  or SQL message.
- Money arithmetic uses checked operations; overflow is an error, not a
  wrap.

Every failure leaves as `{ "error": { "code", "message" } }`. The code is the
core's own; only the status is the API's to choose. The UI translates the
code and shows the message to nobody.

| Code | Status | Raised by | Carries |
|---|---|---|---|
| `validation` | 422 | a field the caller can correct, and a payment above what is owed | `field`, and on a payment `outstanding_centimes` |
| `credit_limit` | 422 | a credit sale that would take the customer past their limit | `balance_after_centimes`, `credit_limit_centimes` |
| `party_ids` | 422 | a facture either side of which is short of what décret 05-468 art. 3 asks | `party_side`, `missing_ids` |
| `not_found` | 404 | a row that is not there, or is another shop's | |
| `duplicate_barcode` | 409 | a barcode a product already holds | |
| `conflict` | 409 | a value another row of the shop already holds where the file allows one (a supplier's name) | `field` |
| `exhausted` | 409 | a number series the shop hands out (in-store barcodes, a document kind's series for one year) has no next value | |
| `bad_request` | 422 | a body that did not parse, before any service ran | |
| `unauthorized` | 401 | no launch token, or the wrong one; the answer carries `WWW-Authenticate: Bearer` | |
| `session_required` | 401 | no session, or one that has stopped standing; carries `WWW-Authenticate: Bearer` | |
| `auth_refused` | 401 | a wrong PIN or password; carries `WWW-Authenticate: Bearer` | |
| `locked_out` | 429 | too many wrong credentials against the same person in a row | `retry_after_seconds` |
| `forbidden` | 403 | a signed-in role the permission table refuses on this route | `permission` |
| `method_not_allowed` | 405 | a route that does not take that method | |
| `ungated_write` | 500 | a write reached a route the permission table names no row for | |
| `money` | 500 | stored money a migration's CHECK makes impossible | |
| `storage` | 500 | the shop file could not complete the operation | |
| `print` | 500 | a stored row the template will not render | |
| `workbook` | 500 | an Excel workbook this app was writing could not be finished | |
| `restart_needed`, `restore_failed_restart_needed` | 500 | the shop file is not open in this process any more | |

`conflict` is a refusal about a row that is already there rather than about
what the caller wrote, which is why it is not a `validation`: a name another
supplier already holds and a name too long for a ticket are two sentences a
screen says differently, and both used to arrive as a `validation` on `name`.

A handful of codes carry more than a sentence, the one exception to "a code
and a sentence", and the payload has eight optional fields for them:
`balance_after_centimes`, `credit_limit_centimes`, `field`,
`outstanding_centimes`, `party_side`, `missing_ids`, `retry_after_seconds`
and `permission`. Each is filled by the
one error that knows it and left out of every other body rather than sent as
a null, so an ordinary refusal is the two keys it always was. They are there
because the till has to say by how much a limit was passed, the fiche by how
much a payment overshot and the facture which side is short of what, and
working any of the three out on the screen would be a second answer to a
question the core has already answered (rule 2). A payment above the debt is
a `validation` on `amount_centimes` rather than a code of its own: it is a
field the caller can correct, and the figure beside it is what makes it
correctable.

## Contract between Rust and TypeScript

Types cross the boundary once, generated, never hand-written twice. The
Rust structs in `crates/api` are the source; a build step writes
`packages/shared/src/generated/*.ts`; CI fails if the generated output is
stale. Money crosses the wire as an integer number of centimes in a
`number` (safe below 2^53, which is 90 trillion dinars) and is formatted
only in `packages/shared`.

## Design

One source of colour, four themes, no branch in TypeScript.

`packages/design` holds the tokens in three tiers: raw ramps
(`primitives.ts`), the roles that point at them (`semantic.ts`), and the
assembly TypeScript imports (`theme.ts`). The role layer carries a theme
axis: Comptoir (light, stone paper, teal selection, ink sidebar, brass on
the one action that moves money), Registre (dark, ink green surfaces, paper
text), Observe (cool grey, emerald brand, rounder) and its dark twin. A
theme owns colour, shadow and radius; space, font size and the control
heights are off the axis, because a theme changes what the app is made of
and never how much room it takes.

`apps/desktop/src/theme.css` is generated from that layer and checked in:
the token blocks (`:root` for Comptoir, one `[data-theme="<name>"]` block
per other theme), the shadcn/ui variable set pointing at our roles, and the
Tailwind v4 `@theme` map. `just theme` regenerates it and a vitest in
`packages/design` fails the gates on a stale file, the shape `just
types-check` has for the generated DTOs. The kit is shadcn/ui, so shadcn's
names (`--background`, `--primary`, `--sidebar-accent`) are the emitted API
while our roles stay the source; `--money` and `--font-numeric` are ours,
because shadcn has no slot for a brass accent that means "this moves money"
or for a figure font.

The switch is one attribute. `data-theme` on `<html>`, written by
`src/lib/theme.tsx`, and nothing else: no component branches on the theme,
and `src/theme.test.ts` greps the source and fails the gates on a theme name
or a `data-theme` outside the provider, the switcher and their test. The
choice lives in the shop file (`preferences` table, `PUT /settings/theme`)
so a second machine in the same shop opens on it; `null` means the shop has
never chosen, and the app opens on Comptoir, the default (the operating
system's light-or-dark setting is not consulted).

Colour reaches a component as a utility class from that map and never as a
literal. `apps/desktop/src/tokens.test.ts` fails the gates on `bg-[`,
`text-[` or a hex outside the generated file. Fonts are vendored through
the `@fontsource` packages and imported in `styles.css`; nothing is fetched
over the network, and a test asserts it.

**The desktop kit.** `apps/desktop/src/components/ui/` is shadcn/ui,
installed through its own CLI onto the tokens above and never through
`shadcn init`, which rewrites `styles.css`; `apps/desktop/src/components/`
holds what is built on top of it (`AppShell`, `PageHeader`, `DataTable`,
`Money`, `ThemeSwitcher` and the rest). `apps/desktop/eslint.config.js`
carries one rule for it: no bare `<input>`, `<button>`, `<select>`,
`<textarea>` or `<table>` in JSX outside `components/ui/` and the kit page,
because a bare element wears the browser's own colour and height and looks
like nothing in a diff. A screen written before the kit is named in
`apps/desktop/src/lint/allowlist.json` rather than exempted silently, and a
test fails on an entry whose file has nothing left to fix. See
`context/processes/20260908-frontend-conventions.md` for the folder shape,
the kit's two folders and what the CLI gets wrong on the way in.

## Data

- SQLite everywhere. One file per shop, also when hosted. WAL mode,
  foreign keys on, migrations embedded in the binary and applied on open.
- Migrations are forward-only and additive once released. Every migration
  ships with a test that opens a database at the previous version and
  applies it.
- Append-only ledgers (stock movements, debt) are the truth; cached
  balances are derived and re-checked.
- `documents` carries the rules its kind decides as CHECKs, not only as
  service code (`2026-09-10-000007_document_kind_rules`): an avoir names the
  document it is written against and a ticket, a facture and a proforma name
  none; an amount tendered and change go together and only on a cash
  document; a document is annulée exactly when it says when, by whom and why,
  and only a ticket and a facture are annulled at all; a proforma's balance
  triple says nothing is owed. The services already
  refuse every one of those rows, so a restored backup, a hand-repaired row
  or an import is what the constraints are for. SQLite cannot add a
  table-level CHECK to a table that exists, so the migration rebuilds
  `documents` the way migration 2 did, keeping every id: the lines, the TVA
  recap, the movements, the ledger and the avoirs all name them.
- `documents` gains `series_year INTEGER NOT NULL DEFAULT 0`
  (`2026-09-10-000009_series_year`), an additive `ADD COLUMN` and not a
  rebuild, backfilled from `issued_at` on the shop's calendar and never from
  `created_at`, which is UTC. The existing `UNIQUE (shop_id, series, number)`
  still holds because the series string already carries the year
  (`doc_facture:2026`); `series_year` is what lets `number_of(kind, year,
  number)` print the right year on a prior year's facture without parsing
  the series string back apart.
- The supply side is its own set of tables (migration
  `2026-09-10-000008`) and the customer side is untouched: `suppliers`
  holds the fiche, `supplier_ledger` and `supplier_allocations` mirror
  `debt_ledger` and `debt_allocations` row for row and CHECK for CHECK,
  with `purchase` where a customer has a sale and `return` where a
  customer has an avoir. The alternative, one `parties` table with a role
  and one party-keyed ledger, was rejected in the plan lens: every M2
  query, index, CHECK and screen is customer-keyed and a supplier never
  buys at the till.
- `purchases` and `purchase_lines` hold what was ordered, with each line's
  landed unit cost fixed when the purchase is saved; `purchase_receipts`
  and `purchase_receipt_lines` hold what actually arrived, one row per
  delivery, tied to the order by a composite key so a receipt can only name
  a line of its own purchase and names it once; each line counts what
  arrived against what was ordered and what went back against what
  arrived. A purchase is never a row of `documents` and neither is a bon
  de réception: that table's NOT NULL régime, its payment mode and its
  customer key have no honest value for something the shop buys, and its
  series are the numbering the tax code hands out for what the shop
  sells. A receipt takes its own number from the counters table under
  `reception:<year>`.
- `expense_categories` carries an i18n key per shop and not a label, seeded
  with the seven the spec names; `expenses` points at one. `jobs` holds the
  day a once-a-day job last ran, per shop, so a restart does not run it
  twice.
- The audit log's action and entity names were rewritten onto one scheme in
  the facture-and-credit milestone (`sale.credit_override` became
  `document.issue_override`, and every row about a document now says
  `document`), and no row already written under the old names is migrated.
  That is only safe because no shop has data yet: a file with a history would
  need a data migration alongside the rename, or its old rows would drop out
  of the queries that read the log by name.
- `audit_log.created_at` is on the shop's calendar like every other date a
  screen shows, since `2026-09-12-000013_audit_log_shop_clock`. Most
  `created_at` columns in the file carry SQLite's `CURRENT_TIMESTAMP` default,
  which is UTC; on the ones a day filter reads it never fires, because the
  service stamps the moment before the insert and the supplier ledger refuses
  a row that arrives without one. The audit log was the column a filter read
  and the default still reached, so a row written at 00:30 in Algiers was
  stored as 23:30 the day before and printed that way on the owner's screen
  while the day filter, which did convert, counted it under the day it was
  written. The migration shifts the
  rows already there by an hour and `services::audit::record` stamps from
  `services::clock` from now on, which is why the two have to travel in one
  version: the shift run against a build that already stamps would move those
  rows an hour into the future. `AuditRowWrite.created_at` is a plain
  `NaiveDateTime` rather than an `Option`, so no insert can fall back to the
  default that is still on the column; the supplier ledger guards the same
  rule with `CoreError::Unstamped` because a caller there picks the moment.
- Encryption at rest: OS-level (BitLocker / FileVault) on the shop PC,
  disk encryption on the server; the app does not roll its own. Backups are
  copies of the file, restorable from the settings screen and tested by
  the backup test opening the copy.

## Release

Decided (Samir, 2026-09-08): the version is semver plus the git short hash
plus the build date, all three embedded in the binary at build time and
shown in About. The product name is Dinar and the bundle identifier is
`com.dinar.app` (Samir, 2026-09-13). It does not change after the first
tag: a second change would be a data-path migration for every shop.

Decided (2026-09-11, M5 T1): the version also heads the log file and will
head the support bundle. `crates/core::build_info::BUILD_INFO` is the one
place the three come from. `build.rs` bakes the git short hash and the
build date in with `env!` at compile time (`git rev-parse --short=8 HEAD`
and the UTC date the crate was compiled), `CARGO_PKG_VERSION` is the
semver. A checkout with no `git` binary or no `.git` still builds: the hash
is `"nogit"`, a sentinel a test can name, never an invented value or the
word `"unknown"`. `crates/api` exposes `GET /build-info`
(`BuildInfoDto`) as the one route `apps/desktop`'s About screen (reachable
from Settings) reads; the web side keeps no second copy of any of the
three. `AppState::open_with_backup_dir` calls
`dzpos_core::log::head_session`, which appends
`build_info::header_line` to `dzpos.log` beside the shop file at the start
of every session, standalone binary and desktop process alike. T3's
support bundle carries the same line: `build_info::header_line(&BUILD_INFO)`
is the one function that formats it, and the bundle's own header will be
that call.

A new build that opens an older shop file copies that file before it
migrates it. The copy goes beside the shop file as
`<name>.before-upgrade-<stamp>.sqlite`, so the daily prune cannot reach it,
and it is taken only when the file already holds a schema and this build has
migrations it has not had: a fresh install copies nothing, and a restart
after the upgrade copies nothing either. A copy that cannot be written stops
the app from starting, because the alternative is a migration running with
nothing to go back to. `crates/api/src/lib.rs`, `open_and_upgrade`, is the
only caller; `crates/api/tests/upgrade_from_a_previous_version.rs` builds a
file at the previous version and opens it. Restoring one is a file swap by
hand today: the restore route takes a daily copy's name and no other kind.

Decided (2026-09-11, M5 T6): a tag matching `v<semver>` on `main` is
the only thing that publishes the GitHub release
(`.github/workflows/release.yml`); an off-main tag or a malformed one
refuses and says why, and a plain push never triggers the workflow at all.
The tag's version, `Cargo.toml`'s and `tauri.conf.json`'s must all agree,
since the first is what `build_info.rs` bakes into the binary and About
shows. A manual dispatch builds the same installers as run artifacts for
exercising the pipeline early; it can never publish. No certificate yet
means the release is cut unsigned and left as a draft, not presented as
finished.

Decided (2026-09-13, Samir): the organisation's Actions minutes are
capped, so tags are pushed to both remotes (`just release vX.Y.Z`) and the
minutes are spent on the public mirror, while the release itself is created
on Dinar-dz/dz-pos by API call (no runner, no minutes) with
`ORG_RELEASE_TOKEN` (today Samir's login token, stored as a secret on the
mirror). Decided the same day: this mirror detour is temporary -- next
month the release is cut directly in the org repo's own CI, and the mirror
goes back to being CI-only. Nothing in the workflow needs rewriting for
that day except retiring the mirror tag push: the same tag on the org
already starts the same jobs there. A private repo's releases are
private downloads -- a GitHub rule, not this file's -- so org releases
stay draft/collaborators-only until the certificate lands. The build is a
matrix of three installers (Windows nsis, macOS dmg on Apple Silicon,
Linux appimage); the build job renames each to a stable filename
(`Dinar-Setup.exe`, `Dinar.dmg`, `Dinar.AppImage`, the version living in
the tag) so the landing page links never carry a version and the updater
manifest points at files that do not move between releases.

Decided (2026-09-11, M5 T0): the window runs under a policy that admits no
remote origin and no inline or evaluated script, and a check on the Rust
side refuses any navigation away from the app's own origins, since the
directive that would have done it was dropped from the CSP spec before any
engine shipped it. The launch token is fetched from the Rust side rather
than left in a page global. What protects it is the policy that stops
injected script from running at all; keeping it in a module variable buys
little on its own.

Decided (2026-09-11, M5 T3): the support bundle a shop can send to whoever
is fixing something is `GET /support-bundle`, gated the same as the
backups block beside it (`EditSettings`), answering a zip of six plain
text files and nothing else. `README.txt` (headed by
`build_info::header_line`, then the sentence a shopkeeper is sent in
French, English and Arabic), `log.txt` (the shop's own `dzpos.log`
verbatim), `migrations.txt` (applied vs. shipped, off
`db::applied_versions` and the now-public `db::embedded_versions`),
`schema.txt` (every table's name and its columns' own names and SQL
types, read live off `sqlite_master`/`PRAGMA table_info`, never a row),
`counts.txt` (products, documents, customers, the shop file's byte size,
backup count and newest date) and `system.txt` (OS, machine language,
time zone). The shop file itself is never in it, and neither is any
customer or product name, a price or amount, any part of a document, a
PIN, a password or a hash of either, the launch token, or the shop's own
RC/NIF/NIS/AI: `crates/core::services::support_bundle`'s own doc names
that list, and `crates/api/tests/support_bundle.rs` seeds a shop with all
of it and fails on any of it turning up anywhere in the zip's bytes,
entry names included. The cost of that list: a bundle this small cannot
answer "why is this customer's balance wrong" (a count of customers and
no row of any of them); it can answer whether the file's schema matches
the build, whether the last upgrade's migrations ran, whether the file's
size looks like a month of trading, and whether a backup was ever taken.
A fault that needs the actual figures still needs the shop to read a
screen aloud, or a supervised look at the file itself. A row is written
to the shop's own audit log each time one is built
(`ACTION_SUPPORT_BUNDLE`), the same reasoning an export's row rests on:
the file is on its way out of the shop even though what is in it never
is. No new dependency: `zip` and `iana-time-zone` were already in
`Cargo.lock`, pulled in transitively through `rust_xlsxwriter`'s own zip
backend and another dependency, and become direct dependencies of the one
crate that reads them rather than a second copy at a different version.

Decided (2026-09-12, M5 T7): `tauri-plugin-updater` is in
`apps/desktop/src-tauri`, configured through `tauri.conf.json`'s
`plugins.updater` block (`pubkey`, `endpoints`) rather than in Rust code,
which is what lets a test read the checked-in config without building
anything. The endpoint is the GitHub release manifest,
`https://github.com/Dinar-dz/dz-pos/releases/latest/download/latest.json`;
this assumes the repository is public, or the manifest mirrored somewhere
that is, by the time a shop's copy is old enough to check --
`releases/latest` on a private repository answers nothing to an
unauthenticated request, and the plugin carries no credential to be one.
`pubkey` is the literal string `UNSET-waiting-on-anouar-and-samir-...`
until the key named in the table below exists; `updater_config_tests`'s
`the_updater_key_is_still_the_named_placeholder` fails if that string is
anything but the placeholder, so the commit that lands the real key has
to retarget this test in the same move rather than the config quietly
drifting first. A second test in the same module fails if any endpoint
stops being `https://`, so a build cannot ship trusting a look-alike or a
plaintext one by accident.

The check is a button on the About screen and nothing else: no interval,
no check on launch, no check on a schedule. `src-tauri/src/updater.rs`
exposes exactly two commands, `check_for_update` and `install_update`,
never the plugin's own IPC commands, so the page cannot reach anything
wider than these two. `check_for_update` answers one of three values --
`Newest`, `Newer { version, size }` or `Unreachable` -- rather than a
rejected promise for the third case, because a shop with no internet today
is not an error the screen needs to explain twice. `size` is `Some` only
because the release workflow writes an extra `size` key into `latest.json`,
beside the `url` and `signature` each `platforms` entry already carries,
and `size_for_download` in `updater.rs` reads it back off `Update::raw_json`
by matching `url` against `Update::download_url` -- not by re-deriving the
`platforms` key, which is `{os}-{arch}` (or with an installer suffix),
picked by the plugin's own `Updater::get_urls`, and not the same string as
`Update::target` (the plain OS the endpoint URL was templated with -- an
earlier version of this file read the field named `target` and assumed it
was the `platforms` key, which is wrong for Windows the moment an arch is
in the key at all). A test,
`size_for_download_reads_the_manifest_the_workflow_actually_writes`, is
pinned to the exact shape the workflow writes, and a second test proves a
root-level `size` or an entry for a URL the plugin did not pick is never
read by mistake. The plugin's own manifest shape carries no byte count
until a download is already running, which is after
a shop has already said yes, too late to be "what it weighs" in the sense
the shop needs before agreeing. `install_update` re-checks, downloads,
verifies against `pubkey`, then installs. On Windows -- the only platform
that ships today -- the plugin itself calls `std::process::exit(0)` right
after launching the installer (`Update::install_inner`); the command's
`download_and_install` call never returns on that success path, so
`app.request_restart()` after it is unreached there, and `RunEvent::Exit`
never fires either. `AppHandle::request_restart` (rather than `restart`,
which skips `RunEvent::Exit` and the axum task's cleanup in `run()` along
with it) is the path this line would take on Linux or macOS, where the
plugin returns instead of exiting the process -- dead code until either
ships, not a bug in the one platform that does.

A shop navigating from the till to Settings already discards whatever cart
was on screen: `routes/till.tsx` keeps it in the route's own state, and
only one route is mounted under `AppShell` at a time (the lock overlay
covers the till without unmounting it, but does not keep it and About both
live). The result is that a check cannot coincide with an open sale today,
by construction rather than by a guard this file added -- there is nothing
mounted to guard. If the cart is ever moved to state that survives
navigation, the About screen's check button needs a guard reading that
state that does not exist yet; this is the day that guard becomes real
work instead of dead code proving nothing.

The workflow refuses to publish an *unsigned* manifest rather than
skipping the release: `.github/scripts/release-gate.sh` takes a seventh
argument, `UPDATER_KEY_PRESENT` (the workflow reads
`secrets.TAURI_SIGNING_PRIVATE_KEY` and hands in `true`/`false`, the same
shape `ON_MAIN` already uses), and emits `updater=publish` only for a real
release with the key present, `updater=skip` for a dry run or a release
missing it. `release.yml`'s `publish` job assembles `latest.json` by hand
across all three installers only when `updater=publish` -- `tauri build`
writes each installer and its `.sig` sidecar, never a combined manifest,
so this is where the "size" field above gets written, off each
installer's own byte count -- and attaches it to the org release only
then. A release with no updater manifest still ships installers a shop
can fetch by hand; the
alternative this guards is a manifest signed with nothing, which an
existing install would trust as if it were real. Proven by four new cases
in `release-gate.test.sh` (eighteen total); the manifest-assembly steps
themselves are written but unexercised, the same as the Windows
code-signing branch beside them -- there is no real release to run either
against yet.

A `pubkey` being configured at all, not `createUpdaterArtifacts`, is what
makes `tauri build` require `TAURI_SIGNING_PRIVATE_KEY` for any installer
target (nsis included); the check is `tauri-cli`'s own `sign_updaters`,
which runs whenever `plugins.updater.pubkey` is set and one of
nsis/msi/appimage/deb/rpm is being built, key or no key. Left alone, this
would fail every dry run the moment `pubkey` was added, key present or
not, since the placeholder string is never a valid signing key regardless.
The `gate` job now reads the secret's presence into its own output,
`updater_key`, deliberately separate from `updater` (which also asks
whether this run is a real release): `release.yml`'s "Build the installer
(unsigned)" step passes `--no-sign` unless `updater_key` is
`true`, and the "signed" one refuses outright rather than accept
`--no-sign` at all (below). Gating on key presence rather than on
`updater == 'publish'` means a manual dry run with the secret configured
still asks `tauri build` to sign the update payload for real, which is
the point of a dry run: proving that machinery works before the first tag
needs it to, the same way the dry run already proves the Windows build
itself. Nothing from a dry run reaches a release regardless, because the
manifest-assembly step below stays gated on `updater == 'publish'` alone.
`sign_updaters` skips decoding the key entirely when `--no-sign` is set,
so the unresolved placeholder `pubkey` is never asked to be a real
minisign key until a real one exists to replace it.

The "signed" step (Windows code-signing certificate configured) never
passes `--no-sign`: the flag skips Authenticode signing along with the
updater signature (`tauri-cli`'s own log line says so, "Skipping binary
signing due to --no-sign flag"), so adding it there on a missing updater
key would import a real certificate and then quietly ship an unsigned
`.exe` while the `publish` job -- which only reads whether a certificate
was configured, not whether anything was actually signed with it -- still
labelled the release "Windows installer, signed." The two keys are
coupled in practice: a certificate cannot be exercised in this workflow
until the updater key exists too, so that step stops with an error
instead, until whoever holds both lands them together
(docs/release-checklist.md, "the key that signs updates").

Raised in the 2026-09-08 handoff, still open, each settled before
`docs/roadmap.md` M5 closes. `docs/release-checklist.md` is the page that
tracks them, alongside the release gates and who holds each:

- Tauri updater signing key: who generates it and who holds it; it never
  enters the repo, CI signs with a secret.
- Windows code-signing certificate: cost and lead time, for Anouar.

## Testing matrix

| What | Tool | Where it runs |
|---|---|---|
| Services, ledgers, numbering | `cargo test`, integration tests against a temp SQLite | this machine, `just gates` |
| TVA, stamp, rounding, amount in words | property tests (proptest) + fixed fixtures named in features.md | same |
| Invoice templates | golden files (hand-rolled, `UPDATE_GOLDENS=1` regenerates and fails the run on purpose; insta still not pulled in), every template × language | same |
| API routes | request tests against an in-process server and temp DB | same |
| Coverage | `cargo llvm-cov` on the personal mirror, main only; `just sonar` ingests `lcov.info` if it is already there | Full CI on `samir1498/dz-pos-ci`; this machine when someone asks |
| Sonar | `just sonar` to sonar.observeone.com, project `dz-pos`, gate ObserveOne way | this machine, before merge and again on main; not in CI |
| React components | vitest + Testing Library, jsdom | this machine, `just gates`; Full CI on the personal mirror |
| Restricted CI | rustfmt, desktop eslint, release-gate script | every `just ci` push to `samir1498/dz-pos-ci` |
| Full CI | clippy, cargo test, pnpm test and build; Windows and coverage on main | `just ci` of main, or `just ci <branch> full` |
| Browser end-to-end | Playwright + chromium against a fresh API and database (`just e2e`); ObserveOne is the recorded tool, Playwright the interim | before a merge, not in CI |
| Mobile | Jest (RN preset), Maestro flows on a real device | later |

Gates (`just gates`): `cargo fmt --check`, the desktop's eslint
(`just lint`), `cargo clippy --all-targets -D warnings`, the generated TS
types diffed against the DTOs, `cargo test`, `pnpm -r test`, `pnpm -r
build`. That is the PR gate on this machine. GitHub Actions runs on the
public personal mirror (`just ci`): Restricted on every push, Full on
main. Dinar-dz never starts a runner. A change is done when the gates
pass and the behaviour was driven, not when a check is green.

## Local development

- Everything runs on the WSL box. Rust and web tests run there directly.
- UI: `just api 4317 .dev/dev.db http://<laptop>:5173` then `just dev`;
  open it from the laptop's browser. The token travels with `just dev`.
- Native window: `pnpm desktop tauri dev` needs a display (WSLg or
  `xvfb-run`). Windows builds come from CI.
- Phone: Expo Go over Tailscale, EAS for builds.
- A shop to develop against: `just seed` fills `.dev/dev.db` with a
  catalogue, twelve customers, five suppliers and thirty days of trading, and
  `just seed-clean` deletes the file so the next `just api` opens an empty
  shop. Cleaning up is the whole file and never a row: the ledgers are append
  only and the document series are gapless by rule, so there is no honest way
  to take a seeded sale back out of a shop from the inside.

The seeder is a development tool and cannot reach a shop's books. That is
enforced in three places rather than one, because any single one of them is a
line somebody edits:

1. `dzpos-seed` is its own crate. Neither `dzpos-api` nor the desktop depends
   on it, so no release build and no bundle can produce the binary; the API
   has no `--seed` flag and no seed route.
   `crates/api/tests/no_seed_entrypoint.rs` fails the moment any of that
   changes, and `just gates` runs it with the rest of `cargo test --workspace`.
2. The binary refuses to run unless `DZPOS_DEV=1` is set, refuses any file
   that is not directly inside a `.dev/` directory, and refuses one whose
   settings carry a shop's own name and identifiers unless `--force` says
   otherwise. `--force` means that and nothing else: it does not lift the
   other two.
3. `just seed` and `just seed-clean` take no path at all, so no argument a
   caller typed can point either of them at a real database.
