# dz-pos

Inventory, sales and invoicing for Algerian shops. **`dz-pos` is a
placeholder name**; the product name, GitHub org and bundle identifier
(`com.dzpos.app`) all change once, together, when the real name lands.
Changing the identifier after a release means a data-path migration, so it
does not change twice.

Research, competitor teardown and the architecture rules live in the
`Observeone-research` repo under
`engineering-and-product/2026-09-07-dz-pos-competitor/`.

## Origin

Code copied from [MonStock](https://github.com/samir1498/MonStock) at
`33d2ab021a1cc3d27624a11771e1df709a069ca0` (2026-07-11), renamed, without
history. Left behind on purpose: the egui desktop (`monstock-desktop`), the
dev CLI (`dev`/`build`/`doctor`/`test`/`lint`/`clean`), the release
workflows, docs, plans and changelog.

## Layout

```
crates/core      Rust: models, diesel repos, services, migrations (SQLite)
crates/backup    daily DB backup
crates/import    import products from a backup file
crates/seed      dev-only: `cargo run -p dzpos-seed` fills the local DB
apps/desktop     Tauri 2 + React 19 + Vite; Rust side in src-tauri/
apps/mobile      Expo / React Native — placeholder, not started
```

Cargo workspace at the root, pnpm workspace over `apps/*`.
`.npmrc` sets `node-linker=hoisted` for Metro.

## Commands

```
cargo test --workspace                       # Rust
cargo llvm-cov --workspace --lcov --output-path lcov.info
pnpm install && pnpm -r build && pnpm -r test # web
pnpm desktop tauri dev                       # desktop app (needs Linux webkit deps, see ci.yml)
cargo run -p dzpos-seed                      # fake data into the local DB
```

Over SSH: `ssh -L 5173:localhost:5173 <box>` then `pnpm desktop dev` shows the
UI in a browser with Tauri IPC unavailable; `tauri dev` needs a display
(WSLg / `xvfb-run`).

## Rules that are not negotiable

From the architecture notes; the reasons are there.

1. The mobile app never knows which mode it is in — one HTTP contract.
2. `dzpos-core` sits behind one HTTP service layer; Tauri commands and the
   phone are equal callers.
3. `shop_id` on every table from day one, even single-shop.
4. One source of truth at any time. The phone is a thin client with a retry
   queue; never bidirectional sync.
5. In local mode exactly one desktop is the server.
6. Money is integer centimes, never `REAL`. The schema still carries `REAL`
   from MonStock — migrate before any fiscal code.

## Quality gates

`cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo
llvm-cov`, `pnpm -r build`, `pnpm -r test`. CI runs them on Linux and
Windows. Sonar: not wired yet — Rust support on the team server is
unverified (`rust:S1481` probe returned 404); check before promising a gate
on the core.
