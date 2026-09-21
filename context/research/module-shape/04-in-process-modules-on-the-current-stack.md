# Candidate 4: in-process modules on the current stack

D7 brainstorm page, judged against the constraints block in
`context/plans/20260921-whether-dinar-becomes-a-core-and-modules.md`. Written
2026-09-21 from `main` at 21c837b, on top of candidate 1
(`01-compile-time-domains.md`): its crate split, 63 moved files, Windows 7
finding and "kernel names the shop on day one" list are taken as read. This
page adds the kernel calling traits at runtime, so it never names a module.

### 1. The shape

D4 followed through means nothing in users, sessions, permissions, audit,
settings, preferences or the money kernel names a sale, so each place where
candidate 1 found the kernel naming one becomes a trait the kernel calls:
`CoreError` (6 retail variants of 20, `crates/core/src/error.rs`),
`models/sql_types.rs` (9 of 10 column enums), `print::number`
(`crates/core/src/print/mod.rs:66` takes `&Document`, `:77` `DocumentKind`)
and the schema. `Permission` is exempt by Samir's decision (plan page, line
141): a module uses the enum, it never adds to it.

One trait, `Module`, object safe, every method with an empty default. Six
traits behind six `Option<&dyn X>` accessors would say the same thing six
times and add no rule, so the trait per extension point is a method per point:

- `routes()`: `RouteDecl { method, path, handler }` rows, data and not a
  `Router`. `crates/api/src/router.rs` is one builder with 90 `.route(` calls
  (candidate 1's count) that `tests/route_gates.rs:174` reads as text; data
  lets the walk in section 7 iterate instead of parse. The kernel folds the
  rows into the `guarded` router at `router.rs:116` behind its three layers.
- `gates()`: `&'static [Gate]`, the row at `crates/api/src/gates/mod.rs:95`
  unchanged. `ROUTE_GATES` is one `const &[Gate]` today (`gates/table.rs:9`,
  67 rows), scanned by `gate_for` (`gates/mod.rs:111`) at
  `session.rs:126`. The kernel concatenates the slices at startup into
  `AppState`: the runtime gate registry the plan's cost 3 said candidate 1
  avoided, paid for in section 7.
- `migrations()`: `EmbeddedMigrations` from the module's own folder (section 5).
- `numbering()`: the prefix a paper prints under, so `print::number_of`
  (`print/mod.rs:77`) takes a `&dyn Numbering`. The five print files that
  candidate 1 section 5 found with no `Document` on a code line stay the
  kernel's engine; templates and `strings.rs` go with retail.
- `error_status()`: `CoreError` keeps its 14 kernel variants and grows one,
  `Module(Box<dyn ModuleError>)`, carrying the `code`, status and fields the
  error table in `docs/architecture.md` lists; `crates/api/src/error.rs`
  (487 lines, 40 `CoreError::` mentions) forwards it.
- `chores()`: the recount `crates/api/src/daily.rs:25` imports from
  `services::stock` becomes a chore retail hands the loop. `hooks()`: section 4.

Two points are free, because today's code takes data and not an enum. Audit
actions are `&'static str` (`crates/core/src/services/audit.rs:276`, 44
`ACTION_` constants; `Facets` at `:396` reads the distinct actions off the
rows), so a module writes its own with 0 kernel change; `settings`
(`schema.rs:72`) and `preferences` (`schema.rs:46`) are `key`/`value`
tables, so a module's keys need no migration. The catch is the read side:
`SettingsDto` (`crates/api/src/dto/settings.rs:310`) is one fixed struct
behind one `GET /settings`, and 3 of the 8 settings routes at `router.rs:202`
are retail (`regime`, `discount-threshold`, `facture-layout`); the module
serves its own `GET /retail/settings` and the screen calls both.

The dashboard goes to retail whole: `Dashboard`
(`crates/core/src/services/dashboard.rs:205`) has 12 fields and 10 are a
shop's, `repos/dashboard.rs` (244 lines) joins five retail tables, and
`apps/desktop/src/routes/dashboard.tsx` reads those fields in 15 places. A
`tiles()` point composing one screen from two modules is needed the day two
modules share a screen; named so the cost is visible, priced at 0 till then.

Offered to no trait: the one connection behind one mutex
(`crates/api/src/lib.rs:68`), the three gates, `Permission` and `can`, audit,
users, sessions, the key/value tables, backup, restore, the money types, the
print engine, the migration runner, the router assembly, the desktop shell.

Registration is an explicit `fn modules() -> Vec<Box<dyn Module>>`, called
where a router is built today, `apps/desktop/src-tauri/src/lib.rs:381` and
`crates/api/src/main.rs:135`. No `inventory`, `linkme` or `ctor` in the five
`Cargo.toml` (grep, 0 hits). A shop build edits one line to leave a module
out: no generated code, no linker magic.

### 2. The cost from today's code, on top of candidate 1's split

Candidate 1 leaves routes, DTOs and gate rows in `crates/api` under `cfg`.
Here a module owns its handlers, so they move. `crates/api/src/routes` has
20 files, 12 retail and 8 kernel, classified by name; `crates/api/src/dto`
has 20, 11 retail and 9 kernel, same method; 82 of the 122 generated DTOs go
with them (candidate 1's split). `settings.rs` in both folders splits in two.

Files edited beyond candidate 1's 11: `router.rs` and `gates/mod.rs`
(assembly), `session.rs:126` (lookup), `lib.rs` (`AppState` carries the list
and the table), `main.rs` and the Tauri `lib.rs` (the list),
`crates/core/src/error.rs` and `crates/api/src/error.rs` (the module arm),
`print/mod.rs` (numbering), `db.rs:102` (the source loop), `daily.rs`
(chores), `tests/route_gates.rs` (the parser at `:174` to `:220` goes),
`tests/one_handler_decides.rs:32` (one `src/routes`),
`tests/export_bindings.rs:37` (`FILES: [&str; 122]` becomes per crate),
`crates/core/tests/migration.rs:2053` (the revert names its source). 15
files, plus the trait, `RouteDecl`, the assembly, the module error and the
hook types, uncounted because they do not exist. Tests: `crates/core/tests`
has 65 files at 21c837b (candidate 1 counted 59 at 06308cd; shifts and cash
refunds landed between), `crates/api/tests` 32; the retail ones move too.

### 3. The desktop side of a module

One bundle, by decision: `apps/desktop/vite.config.ts` sets
`autoCodeSplitting: false` and says why. Nothing loads at runtime; a module
on the desktop is a folder and three arrays. The generator takes one
`routesDirectory` (`apps/desktop/tsr.config.json`), but
`@tanstack/virtual-file-routes@1.162.0` (transitive today, a direct
devDependency then) exports `physical(pathPrefix, directory)`, which mounts
a second folder of route files at a prefix (`dist/esm/api.d.ts:15`), through
the generator's `virtualRouteConfig`. So `src/routes/` keeps the kernel's 13
screens (`__root`, `index`, `kit`, `audit`, the settings files minus `regime`
and `data`; 45 route files in all, by name), `src/modules/retail/routes/`
holds the other 32 mounted at `''`, and `routeTree.gen.ts` stays one file.
Whether two folders mounted at one prefix compose, and whether a flat
`settings.regime.tsx` in one nests under `settings.tsx` in the other, was
not checked; that is the one spike needed.

The nav is a const array at `apps/desktop/src/components/AppShell.tsx:84`
to `:169`, 12 items with `to`, `label: Key` and an optional `permission`,
filtered at `:192`; a module exports its own and the shell concatenates.
i18n: `Key = keyof typeof en` (`apps/desktop/src/i18n/index.tsx:8`), 818
keys in each of three JSON files, `i18n/keys.test.ts` fails a key missing
from one language. A module ships three files, `index.tsx` spreads them into
one typed object so `Key` stays exact, and `keys.test.ts` runs per folder
plus a check that no key is spelled twice. One `modules.ts` imports the
retail folder; a build without retail edits that line and Vite drops the rest.

### 4. A money write with a module hook inside it

First the fork this candidate cannot dodge. The sale is
`crates/core/src/services/sales.rs:228`, one `conn.transaction` closure to
`:663` that calls `documents`, `audit`, `shifts`, `debt`, `idempotency`,
`permissions`, `stock`, `customers`, `shops` and `settings` (grep over the
closure). If retail is a module, the kernel has no sale to hook. Either
`documents`, the counters and the TVA and stamp rules (the "real reuse" the
plan measures at line 68) move into the kernel and retail hooks the issue,
or the sale stays retail's and the hook is a point retail offers a third
module. Candidate 1's con on `documents` (its section 6) is the same fork.
D3's tear list decides it; the contract is the same either way.

The contract, modelled on what is there. `tag_if_outside_a_shift`
(`crates/core/src/services/shifts.rs:538`) is the shape: it takes the
closure's `&mut SqliteConnection`, reads, writes an audit row that dies with
the sale, returns `Result`. So `fn inside(&self, conn, issued: &Issued)
-> Result<Outcome, CoreError>` runs inside the closure after the lines are
priced and the number drawn. A hook may read any table through that
connection, may write rows that must roll back with the sale, and may return
`Err`, which unwinds everything the closure wrote, number included. It may
not change the total (`Issued` is a shared reference), open its own
transaction, print, sleep, touch the network or a file (the one mutex at
`lib.rs:68` is held, so the phone and the till wait on it), or write a row
that must survive a rollback. That case has a home: `sales.rs:220` to `:226`
hoist `refused` and `price_refused` out of the closure and write them after
`:663`, and `permissions.rs:259` to `:268` say why. So a second method,
`after(&self, conn, &Result<Sale>, Outcome)`, runs outside the closure for a
row about the attempt. A hook that is itself money (a repair deposit) writes
it in `inside`, in the same transaction: what one process buys over
candidate 2's outbox.

### 5. Migrations from two crates into one file

Verified against `diesel_migrations` 2.3.2 (`Cargo.lock:1128`, source under
`~/.cargo/registry`): `run_pending_migrations` takes one source per call
(`migration_harness.rs:31`); `pending_migrations` keys a source's migrations
by version, drops the applied ones and sorts the rest (`:111` to `:130`);
`applied_migrations` is one table of versions; `revert_last_migration(source)`
looks the top applied version up in the source it is given and fails with
`UnknownMigrationVersion` when that source lacks it (`:89` to `:105`).
`db.rs:7` embeds one folder of 19 today and `db.rs:102` runs it once.

So `db::open` runs the kernel's source first, then each module's in
registration order, on the one connection, under three rules that are each a
test in section 7. Versions are unique across sources: two folders in two
crates sharing a `2026-09-21-000017` prefix are one version to the harness
and the second is silently skipped as applied (`:119` to `:123`). A module
`up.sql` names only its own tables and a kernel `up.sql` never names a
module's; that keeps `revert_last_migration` meaningful, since a module
`down.sql` cannot undo an `ALTER` on a kernel table and the revert test
(`crates/core/tests/migration.rs:2053`) must be told which source owns the
top version. The 4 mixed folders
candidate 1 counted (`init`, `documents`, `000008`, `000009`) split into a
kernel half and a retail half, allowed only because 0 shop files exist. D5's
cheap answer, one shared list in the kernel, still works here as a fallback;
it puts the shop's tables back in the kernel, the thing D4 forbids.

### 6. Windows 7

Candidate 1 section 3, inherited whole: the toolchain (Rust 1.78 and later
need Windows 10 for every `pc-windows` target) and WebView2 109 rule it out
before any shape does. Same binary, same build (`release.yml:171`); no new source.

### 7. Which walks survive, and the test that replaces the crate wall

The five walks candidate 1 named. `services_go_through_services.rs` (4
tests) and `repos_own_the_queries.rs` narrow per crate as under candidate 1
and the compiler keeps what they lose. `one_handler_decides.rs` reads one
`src/routes` (`:32` to `:41`); each routes folder carries a copy.
`route_gates.rs` loses its parser: the text walk at `:174` to `:220`
cannot see a module's rows, and the guard at `:242` to `:246` exists because
that parser can break, so a walk over every module's `routes()` data is the
stronger test; the HTTP half (`:105` to `:114`) iterates the assembled table
unchanged. `apps/desktop/src/theme.test.ts` walks all of `src` (`:28`), so a
module folder there is inside it. The exhaustive permission match
(`permissions.rs:205` to `:225`) survives whole; nothing adds a variant.

The new test, `modules_compose`, replaces candidate 1's crate wall. It
registers the kernel and every module and builds the real router, which
matters because axum panics on a second route at one path (`panic_on_err!`
at `axum-0.8.9/src/routing/mod.rs:180`), a crash at launch on a shop PC
otherwise. Then, over the assembled data: every gate row names a declared
route and every declared write has a row (the two directions
`route_gates.rs:269` and `:284` check today); migration versions are unique
across sources; each module `up.sql` names no kernel table and each kernel
`up.sql` no module table; every module DTO's `export_to` is in its crate's
`FILES`. Last, Anouar's sentence run rather than argued: a `fixtures` module
in the test tree contributes one route, one table and one hook that returns
`Err`; the retail suite passes with it registered, and a sale through that
hook writes no `documents`, `counters` or `audit_log` row. Candidate 1 proves
isolation by the compiler for repos and cycles; this proves it by a run.

### 8. Pros, cons, the one thing that kills it

Pros. The kernel stops naming a sale, which is what kills candidate 1. Audit
actions and the two key/value tables are extension points today at 0 cost.
One process, one connection, one transaction: a module's money lands inside
the sale, so constraints 3 and 4 hold without an outbox. The module list is
one line, the desktop is folders and arrays in one bundle.

Cons. The gate table is assembled at startup, which the plan's cost 3 counted
as a loss and the compose test only partly buys back. The module error arm
loses the exhaustive `CoreError` match. The kernel has no reports screen.
The migration rules are three tests and a discipline where candidate 1 has
one folder. 15 more files change, `documents` waits on D3, a trait is complexity.

What kills it: every method of the trait has exactly one implementer on the
day it is written. The boundary is drawn from imagination, not from D3's
tear list, and each place the second trade differs is a kernel change plus a
retail change, candidate 1's two-crate cost with a trait on top. A trait
extracted from two real modules is cheap; one invented ahead of the second
is wrong exactly where it matters. Second worst: the `documents` fork.

### 9. Belongs to another candidate, noted and not pursued

- Section 1's trait surface is what candidate 5's WebAssembly host would put
  across a wasm boundary; that page should price serialising `Issued`.
  Candidate 7 is this plus a dynamic ORM; `tiles()` is its first step.
- Candidate 6 is the baseline: a fork edits 0 traits, and the trait invented
  here is what a fork lets two real cases extract later. Candidate 3's table
  prefix falls out of section 5's ownership rule with no `ATTACH`; candidate
  2's 39 deferred transactions are untouched, one connection means
  `BUSY_SNAPSHOT` never arises.
