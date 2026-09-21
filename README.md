# dz-pos

Inventory, sales and invoicing for Algerian shops. The product name is
**Dinar**; the repo, crates and paths stay `dz-pos`. The installer bundle
identifier is `com.dinar.app`. Changing that identifier after a release
means a data-path migration, so it does not change twice.

Research (law, tooling, competitor teardown, market) lives in
`research/` in this repo. The Lumina teardown the current work came out of
is `research/competitors/2026-09-20-lumina-teardown/`; the first pass at
them and the market sits under
`research/competitors/2026-09-07-lumina-and-market/`.

## Origin

Scaffold only. Nothing is copied from
[MonStock](https://github.com/samir1498/MonStock); it stays a read-only
reference at `33d2ab0` (2026-07-11). Anouar's call, 2026-09-07: document
features and architecture first, then build fresh with tests from the first
commit.

## Layout

```
crates/core      Rust: models, diesel repos, services, migrations (SQLite)
crates/api       Rust: axum HTTP server over core; launch token; the only door
apps/desktop     Tauri 2 + React 19 + Vite; Rust side in src-tauri/; e2e/
apps/mobile      Expo thin client: pair, till, queue, print (M6)
packages/shared  TS: API client, types generated from crates/api, money display
packages/design  TS: design tokens in three tiers
fixtures/        money fixtures read by cargo test and vitest alike;
                 print/{ticket_80mm,facture_a4,statement_a4,debt_slip_80mm}/
                 goldens read by cargo test alone
docs/            features.md and architecture.md, the spec every task cites;
                 roadmap.md, the milestones to a first shop;
                 release-checklist.md, what has to be true before v1.0 and
                 who holds each of them
```

Cargo workspace at the root, pnpm workspace over `apps/*`, `packages/*`
and `design`.
`.npmrc` sets `node-linker=hoisted` for Metro.

## Screens

`apps/desktop/src/routes`: dashboard, till (the app's launch screen),
products, customers, documents, suppliers, purchases, expenses, settings,
audit (the owner's audit log). `/kit` is an eleventh route, every kit
component in every state on one page, built only in a dev build
(`routes/kit.tsx` throws `notFound()` otherwise) and the page a reviewer
compares against the mockups.

## Landing page

`apps/landing`: Astro, French/English/Arabic as three routes, reading
`@dzpos/design` for its tokens and `apps/desktop/e2e/screenshots/*.png` for
every picture it shows, so it cannot drift from the application it
advertises. `just landing` serves it; `just landing-build` builds it (that
build already rides `pnpm -r build`, so a broken page fails `just gates`);
`just landing-art` reruns the committed product shots and the Open Graph
card from the design package. `just landing-deploy` would publish it to the
Cloudflare Pages project named in `apps/landing/src/lib/site.ts`, but
refuses to run unless `DZPOS_LANDING_PUBLISH=1` is set. It is blocked on
Samir: the price is not set, and the Arabic copy has not been read by a
native speaker (`context/plans/archived/20260911-landing-page.md`).

## Commands

The `justfile` at the root is the list; `just` alone prints it.

```
just gates        # fmt, lint, file sizes, clippy, generated types check, tests, builds; what a PR needs
just claim        # claim the shared build folder for this checkout before a bare `cargo`
just lint         # the desktop's one eslint rule: no bare input, button, select, textarea or table
just e2e          # Playwright against a fresh API and database, fr then en then ar
just api          # the API on 4317 with a dev database and a fresh launch token
just seed         # fill .dev/dev.db with a month of trading to develop against
just seed-clean   # delete .dev/dev.db so the next `just api` opens an empty shop
just dev          # the web UI on 5173, reads the token just api wrote
just tauri        # the native window; needs a display
just types        # rewrite packages/shared/src/generated from the Rust DTOs
just screenshot   # retake only the committed screenshots under e2e/screenshots
just status       # where we are: the ladder and the active plans
```

`just seed` writes the same file on every machine, and writes it fresh each
run: it deletes `.dev/dev.db` and fills a new one. Cleaning up is the whole
file and never a row, because the ledgers are append only and the document
series are gapless. Both recipes are development only and take no path;
`docs/architecture.md` (Local development) says where that is enforced and
why it is enforced in three places.

Every `cargo` command builds into one folder shared by every worktree and
the laptop clone (`CARGO_TARGET_DIR`, next to the main checkout's `.git`),
never a `target/` of its own; `just claim` (which every recipe above that
touches cargo runs first) points that folder at this checkout before a
bare `cargo` command reuses another checkout's build by mistake. A bare
`cargo` also needs `CARGO_TARGET_DIR` exported into the same shell; `just
claim` on its own does not export it there
(`context/processes/20260908-machines-and-heavy-jobs.md`).

Every API route but `/health` needs the launch token, so `just api` runs
first and `just dev` after it. From another machine, pass the browser's
origin to `just api` (its third argument) or the API refuses it.

The API binary takes `--db <file>` (created and migrated if missing),
`--port` (4317, 0 picks a free one), `--shop` (1), `--backup-dir <dir>`
(default: a `backups` folder beside the database; every copy the settings
screen makes and every restore reads from there) and `--daily-backup`
(copy the shop file at launch when the newest copy is a day old, and keep
checking while the server runs; the desktop does this on its own, the flag
exercises the loop without Tauri). `cargo run -p dzpos-api -- --help`
prints the same list.

## Rules that are not negotiable

From the architecture notes; the reasons are there.

1. The mobile app never knows which mode it is in, one HTTP contract.
2. `dzpos-core` sits behind one HTTP service layer; Tauri commands and the
   phone are equal callers.
3. `shop_id` on every table from day one, even single-shop.
4. One source of truth at any time. The phone is a thin client with a retry
   queue; never bidirectional sync.
5. In local mode exactly one desktop is the server.
6. Money is integer centimes, never `REAL`.

## Quality gates

`just gates` is the PR gate on this machine. GitHub Actions runs on the
public personal mirror `samir1498/dz-pos-ci` (`just ci` copies the
branch): Restricted is rustfmt, desktop eslint and the release-gate
script, on every push; Full is clippy, cargo test, pnpm test and build,
with Windows and coverage on main. Dinar-dz never starts a runner (org
budget is capped). `just e2e` runs before a merge but not in CI. Sonar:
`just sonar` on this machine against sonar.observeone.com (project
`dz-pos`). Not in CI.
