# Candidate 2: microservice-shaped modules on one machine, one database

D7 brainstorm page, judged against the constraints block in
`context/plans/20260921-whether-dinar-becomes-a-core-and-modules.md`. Written
2026-09-21 from `main` at 06308cd; every number says how it was counted.

## 1. The shape on the shop PC

The Dinar window stays the Tauri process it is today. It opens the shop file
once, generates the launch token, binds the API on a loopback port the OS
picks, and runs the axum server as a tokio task inside its own process
(`apps/desktop/src-tauri/src/lib.rs`, `run()` at line 324: `AppState::open`,
`LaunchToken::generate`, `dzpos_api::bind(0)` at 339, the spawn at 382, and
`task.abort()` on `RunEvent::Exit` at 434 to 439). There is no sidecar today:
`apps/desktop/src-tauri/tauri.conf.json` has no `externalBin` entry (grep count
0), so a module process is a new kind of thing. Each trade module is a second
Rust binary shaped like `crates/api/src/main.rs` (156 lines by `wc`: `--db`,
`--port`, `--shop`, `--lan`, the token read from `DZPOS_API_TOKEN` at line 52),
spawned by the desktop at startup with the same shop file path, the same
launch token in its environment, and port 0; the module prints the port it
bound and the desktop remembers it. The module opens its own diesel connection
to the same SQLite file through `dzpos_core::db::open` (`crates/core/src/db.rs:69`),
so it links `dzpos-core` and gets the money kernel, the permission enum, the
repos and the embedded migrations. When it needs something the core owns (a
session, a fiche, a document number) it calls the core's API on 127.0.0.1 with
the launch token; the device gate lets a loopback caller through bare
(`crates/api/src/device.rs:80-81`, the `is_loopback` test on `ConnectInfo`), so
a module needs no device token, only the session header it forwards from its
caller. The desktop webview may call a module port directly: the served CSP
allows `connect-src http://127.0.0.1:*` (asserted in
`apps/desktop/src-tauri/src/lib.rs` at line 197). The phone cannot: rule 1 in
`docs/architecture.md` says one HTTP contract, `crates/api/src/mdns.rs:10`
announces one `_dzpos._tcp.local.` service per shop with one port, and
`apps/mobile/lib/api.ts:21-24` builds every URL from one `API_BASE`. So the core
reverse-proxies `/m/<module>/...` to the module's port, or the phone learns N
bases and rule 1 is gone. The proxy is part of this candidate; axum ships none.

Stopping: on exit the desktop aborts its own server task and would also have to
kill its modules. One that outlives the window keeps the shop file and its port
open; `tauri_plugin_single_instance` (`lib.rs:353`) guards the Tauri process only.

## 2. What it costs from today's code

- A supervisor in the desktop: spawn N children with env and args, read the
  port each prints, forward the launch token, kill them on exit, restart one
  that dies. None of it exists; the closest code is the 156-line
  `crates/api/src/main.rs` and the one spawn-and-abort in
  `apps/desktop/src-tauri/src/lib.rs`.
- The gate stack, per module or lifted into a crate both binaries link:
  `crates/api/src/session.rs` 331 lines, `device.rs` 144, `token.rs` 150,
  `gates/` 709 across three files (`wc -l`). Sessions resolve against the
  `sessions` table (`session.rs:119`, `sessions::resolve`), which a module can
  read from the shared file, so the code is reusable as is if `dzpos-api` is
  the library both binaries depend on; the desktop depends on it that way now
  (`apps/desktop/src-tauri/Cargo.toml`).
- A reverse proxy for the phone and a port registry in the core. New code.
- The router walk: `crates/api/tests/route_gates.rs:174` does
  `include_str!("../src/router.rs")` at compile time, so it sees one router and
  never a module's. Each module carries its own copy of the test and its own
  table, or the walk takes a list of routers. Today's table has 67 rows
  (`grep -c 'Gate {' crates/api/src/gates/table.rs`) over 90 `.route(` lines
  (`grep -c` on `crates/api/src/router.rs`; 97 method tokens counted
  separately, because some lines chain `get().post()`).
- Transactions: 39 `.transaction(` lines across 20 service files (`grep -c`
  over `crates/core/src/services/*.rs`, summed, no comment hits), each a
  deferred begin because that is what diesel's `transaction()` issues; the 2
  `BEGIN IMMEDIATE` statements are separate `batch_execute` calls
  (`pairing.rs:100`, `:284`), and `immediate_transaction` appears 0 times under
  `crates/`. Section 3 says why each of the 39 becomes a contention point.
- Backup and restore coordination (section 7): a close-all protocol between
  processes, which exists in no form.
- Migrations: `db::open` migrates on every open (`db.rs:69-73`) from the set
  embedded in whichever binary opens the file (`db.rs:7`); two binaries from
  different commits carry two sets, and the second to start applies what the
  first did not know. 19 migration folders (`ls crates/core/migrations | wc
  -l`), 31 tables (`grep -c 'diesel::table!' crates/core/src/schema.rs`).

## 3. The single-writer question

Today there is one connection on the whole machine, behind one mutex:
`AppState.conn: Arc<Mutex<Option<Conn>>>` (`crates/api/src/lib.rs:68`); every
query goes through `with_conn` (`lib.rs:382`) and the async side reaches it via
`spawn_blocking` (`lib.rs:403`). Requests are serialized in Rust before SQLite
sees one, which `crates/core/src/services/pairing.rs:93` says in its own words:
the race cannot happen in the API as it stands. `crates/core/src/db.rs:93` sets
three pragmas on open, `journal_mode=WAL`, `foreign_keys=ON`, `busy_timeout =
5000`, and nothing else; there is no `synchronous` line (grep count 0), so
durability is SQLite's compiled default and this candidate does not move it.

With a module process there are two connections to one file, and the busy
timeout does work for the first time. What follows is SQLite behaviour, not
repo code. WAL allows any number of readers beside one writer, each reader on
a consistent snapshot. Two writers take turns on the WAL write lock; the second
waits up to the 5 000 ms of `busy_timeout`, then gets `SQLITE_BUSY`. The catch
is the deferred `BEGIN` every `.transaction(` in this repo uses: a transaction
that has read under a deferred begin and then tries to write, after the other
process committed in between, is refused with `SQLITE_BUSY_SNAPSHOT` at once,
and the timeout never runs. `pairing.rs:87-92` describes this case and is why
that one service begins `IMMEDIATE`. So the till and a module both writing
looks like this: the sale at `crates/core/src/services/sales.rs:228` reads
settings, the shop and the idempotency table, then writes; if the module
committed anything between that read and the first write, the whole sale
unwinds as a `CoreError::Query` the API answers with the shop-file message at
`crates/api/src/error.rs:166-169` (status not checked), and the cashier rings again.
To make the two take turns instead, all 39 deferred transactions become
immediate, and then a long module transaction stalls a sale for up to five
seconds before failing it. Neither outcome can happen today.

## 4. A money write across the core and a module

It does not stay one transaction. Two connections in two processes cannot
share a SQLite transaction; a `BEGIN` cannot be handed across a socket. What
replaces it: the core stays the only writer of the tables a sale touches,
which from the calls inside the closure at `sales.rs:228-663` are
`documents::issue`, `stock::record`, `debt::append_at`,
`debt::settle_from_credit`, `audit::record` (six calls), `idempotency::record`
and `shifts::tag_if_outside_a_shift`. Read in the repos: `document_lines` and
`document_tva` (`crates/core/src/repos/documents.rs:53,63`), `counters` for the
gapless number (`repos/counters.rs:30`), `debt_ledger` (`repos/debt.rs:33`);
inferred from the call names: `documents`, `debt_allocations`,
`stock_movements`, `audit_log`, `sale_idempotency_keys`, `shifts`. A module
writes only its own tables, in its
own transaction, after the core answered for the money part. A crash between
the two leaves a core document with no module row, and the module owns the
repair: on start it looks for documents it should have a row for and writes
them, an outbox with the shop file as the outbox. `foreign_keys=ON` forces the
order (a module row naming an uncommitted document id fails) and forbids the
reverse: the core can never hold a foreign key into a module table, or the
module's absence breaks a sale. The centime arithmetic stays in one place,
since both binaries link `crates/core/src/money`; what is lost is atomicity
between a sale and whatever the module adds to it. When the module's write is
itself money (a clinic's fee ledger, a repair deposit), that money lands
outside the sale's transaction. The constraints block asks how a shape that
gives this up gets it back. The honest answer: it does not; it gets an
idempotent retry and a reconciliation on start instead.

## 5. On a Windows 7 machine

This candidate changes nothing about Windows 7. The user-facing part is still
the Tauri webview, and WebView2 on Windows 7 stopped at 109 in January 2023
(the constraints block's own fact). The API process and a module process are
plain Rust: `dzpos-api` depends on axum, tokio, tower-http, mdns-sd and
getrandom 0.3.4 (`crates/api/Cargo.toml`, `Cargo.lock:1825-1826`), and
`dzpos-core` bundles SQLite through `libsqlite3-sys` with the `bundled`
feature (`crates/core/Cargo.toml`), so nothing in them needs WebView2. From
memory and not from the repo, to verify before D6: Rust's tier-1 Windows
targets have required Windows 10 since 1.78, and the `*-win7-windows-msvc`
targets are tier 3 and built from source; CI builds on
`dtolnay/rust-toolchain@stable` and `windows-latest`
(`.github/workflows/ci-restricted.yml:21`, `release.yml:171`). A Windows 7
build of even the headless API is a toolchain project of its own, and
splitting into processes neither helps nor hurts it. A headless core serving a
browser over the LAN with no window is candidate 5's territory, not this one.

## 6. Which of the five source-walk tests survive

Reading the plan's list as `crates/api/tests/route_gates.rs`,
`crates/api/tests/one_handler_decides.rs`,
`crates/core/tests/services_go_through_services.rs`,
`crates/core/tests/repos_own_the_queries.rs` and `scripts/file-sizes.mjs`:

- `route_gates.rs`: reads one file at compile time (line 174) and walks
  `gates::ROUTE_GATES`. It survives for the core and is blind to every module
  router; each module ships its own copy and table, or the test grows a list
  of routers. Its HTTP half has to start the module process to cover it.
- `one_handler_decides.rs`: `read_dir` over `crates/api/src/routes` (line 32).
  Same story, one crate's folder.
- `services_go_through_services.rs` walks all of `crates/core/src`
  (`core_source_files()`, line 480); `repos_own_the_queries.rs` walks
  `crates/core/src/services` (line 46). A module's services live in another
  crate and neither walk sees them. Worse, the module links
  `dzpos-core` and holds its own connection, so it can call any core repo
  directly, which is the reach these walks exist to forbid. The process
  boundary does not restore the rule; it is a crate boundary that would.
- `scripts/file-sizes.mjs`: path-based, survives untouched.
- The exhaustive match at `crates/core/src/services/permissions.rs:206`
  (`const fn can`, 15 variants counted in the enum, three roles at line 24)
  survives whole, because permissions stay one global list and both binaries
  link the one enum. A sixteenth permission is added in the core crate and
  placed for every role, or nothing compiles.

Two of five keep their meaning for the core only, two lose their reach to the
code they are about, one is unaffected, and the permission match is intact.

## 7. A power cut with two processes mid-write

For SQLite each process's transaction is atomic on its own and at most one
holds the WAL write lock, so the file after the cut holds whichever commits
reached the log, the same guarantee as today. What is new is section 4's gap:
a core sale committed and a module row not yet written, which the module has
to notice at start.

The coordination cost shows in restore, not in the cut. `AppState::restore`
(`crates/api/src/lib.rs:224-301`) takes the mutex, runs `VACUUM INTO` for a
safety copy, runs `PRAGMA wal_checkpoint(TRUNCATE)`, closes its connection
with `drop(guard.take())` at line 281, and renames the staged copy over the
shop file at line 283. With a module holding a read snapshot the checkpoint
answers busy, which `crates/core/src/db.rs:51-63` turns into `CheckpointBusy`
(its message says the shop file is in use by another connection), and restore
stops at step 5. Past that, the rename at line 283 is over a file another
process holds open, which on Windows fails (from memory, not the repo), and
`reopen_original` (`lib.rs:497`) puts the old file back. Restore needs a
protocol: tell every
module to close, wait, swap, tell them to reopen. The daily backup
(`crates/api/src/daily.rs:35`, `backup::create` at
`crates/core/src/services/backup.rs:317`, a `VACUUM INTO`) is a read and
coexists with a module writer under WAL. The upgrade copy in
`open_and_upgrade` (`lib.rs:431`) is taken when the desktop opens the file; a
module started first opens and migrates the file before that copy exists.

## 8. Pros, cons, the one thing that kills it

Pros. A module crash does not take the window down, and a module ships
without rebuilding the desktop. The launch token and the loopback rule extend
to a second local process with no new auth code (`device.rs` lets loopback
through; the token travels by environment as `just api` does, `justfile:146-153`).
The money kernel, the permission enum and the repos are linked, not copied.

Cons. The transaction question has no good answer (section 4). Each of 39
deferred transactions becomes a `BUSY_SNAPSHOT` hazard, or an immediate one
that can wait five seconds. Restore and the upgrade copy need a close-all
protocol. The phone needs a reverse proxy in the core or a second base URL.
Four source walks lose reach. A supervisor, a registry and a proxy exist for
the shape and not for a shop. Against Samir's "less complex" line, this is the
heaviest way to put a second writer on a file designed around having one.

The one thing that kills it: it turns Anouar's ask inside out. He asked that
adding a module cannot break the till. Here a module is a second writer on the
till's file, so a slow or hung module transaction stalls a sale for
`busy_timeout` and then fails it (`db.rs:93`), and a module reading under a
snapshot at the wrong moment makes a sale roll back with `BUSY_SNAPSHOT`. The
module needs no bug in the core to break the core; it only needs to hold the
write lock. Today that failure cannot happen, because there is one connection
behind one mutex (`lib.rs:68`).

## 9. Noticed, belongs elsewhere

- Making the 39 deferred transactions immediate only matters with a second
  writer; it belongs to whichever candidate has one (this one and candidate
  3), not to today's code.
- Restore needing every handle closed hits candidate 3 too, with N files.
- A module reaching past the core into repos it links is exactly the line D4
  draws as a test; a process boundary does not draw it, a crate boundary does
  (candidates 1 and 4).
- The token-by-environment and print-the-port handshake in
  `crates/api/src/main.rs` is what candidate 5's Electron or web-app shell
  would need too.
- WebView2 109 and the Rust Windows 7 targets are one question shared by
  every candidate that keeps Rust; answer it once before D6, not per candidate.
