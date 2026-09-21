# Candidate 1: compile-time domains

Brainstorm page for D7 of
`context/plans/20260921-whether-dinar-becomes-a-core-and-modules.md`, written
2026-09-21 against main at 06308cd; every count was taken over that checkout
and its method sits beside it.

### 1. The shape

One cargo workspace, as today, with `crates/core` cut in two: `crates/kernel`
(the crate `dzpos-kernel`) holding `db.rs`, `error.rs`, `money/`, `lang.rs`,
`log.rs`, `build_info.rs`, the eleven services that do not know what a shop
sells (users, sessions, permissions, audit, settings, preferences, pairing,
backup, support_bundle, clock, shops), their seven repos and six models, the
eight print engine files (`layout`, `raster`, `escpos`, `bidi`, `png`,
`thermal`, `strings`, `mod`), and the whole `migrations/` folder; and
`crates/retail` (the crate `dzpos-retail`) depending on it, holding the other
24 services, 17 repos, 14 models and 8 print templates. `crates/api` stays
one crate, depends on the kernel and, as `optional = true`, on retail, with a
cargo feature `retail = ["dep:dzpos-retail"]` that gates its retail route
modules, DTO modules and gate rows; `apps/desktop/src-tauri` forwards the
feature and builds one binary per trade by turning features on.
The desktop React app stays one app; its 45 route files and the one TanStack
route tree stay where they are (`apps/desktop/tsr.config.json` points at one
`routesDirectory`). Until a second trade exists, `default = ["retail"]` and
the built product is byte for byte today's. The alternative, one api crate
per trade, is priced in section 2 and loses on the gate walk.

### 2. What it costs from today's code

Files that move out of `crates/core/src` into `crates/retail/src`, counted by
listing each folder and classifying by name: services 24 of 35 (all but the
eleven above; `seed.rs` goes with retail), repos 17 of 24 (kernel keeps
audit, users, sessions, settings, preferences, pairing, shops), models 14 of
20 (kernel keeps audit, user, session, shop, pairing, sql_types), print 8 of
16 (ticket, facture, facture_roll, facture_view, statement, debt_slip,
barcode_label, refusals). 63 source files move. `repos/counters` and
`repos/jobs` are owned by no service (`services_go_through_services.rs:40`,
`NO_SERVICE_OWNS_THEM`) and reached only by documents, products, purchases
and stock (grep of `repos::counters` and `repos::jobs` over `src/services`),
so both, and `models/job.rs` with them, land in retail.

Tests that move, counted by grepping each file in `crates/core/tests` for a
retail service, a print module or a retail model: 48 of 59 name retail and go
with it; 11 stay. `crates/api/tests` has 31 files and stays with the api
crate; the retail ones run under the feature.

Import edges to rewrite, counted on non-comment lines outside `#[cfg(test)]`
of every service file: 35 edges from a retail service into a kernel service
(audit 15, clock 12, shops 3, permissions 2, settings 2, users 1). Each
becomes `dzpos_kernel::services::x` instead of `crate::services::x`. Edges the
other way: 0. Two-service rings on code lines: 0. The plan page's
"prerequisite nobody has noticed" says three rings still pin users, sessions,
audit and preferences; at 06308cd `RINGS_STILL_OPEN` is `[&str; 0]`
(`crates/core/tests/services_go_through_services.rs:260`) and T11 of
`architecture-fixes-without-a-domain-split` reads `status: 'done'`. A first
walk over this page's data found rings in `audit`, `permissions` and `shifts`;
every one was a `///` line, which the test's own `code_of` strips. The test's
zero is the number.

Visibility that widens: `services/mod.rs` holds five `pub(crate)` helpers.
`role_of` (called by sales and shifts), `optional_field` (10 callers, 9 of
them retail), `bounded_field` (5 callers, 3 retail) become `pub`;
`user_names` and `end_sessions_of` have kernel callers only and stay.
`lib.rs:13` has `pub(crate) mod repos`; that line is what stops the retail
crate reaching a kernel repo, and it needs no change.

The permission enum: nothing. Samir's decision keeps the 15 variants at
`crates/core/src/services/permissions.rs:39` and the match at `:206` in one
place; they move to the kernel crate as one file. Ten of the fifteen variants
name a sale, a till, a price or a ledger (Sell, DiscountAboveThreshold,
OverrideCreditBlock, SeeCostAndMargin, EditFiches, CommitMoney,
CorrectLedger, ChangePriceAtTheTill, OpenAndCloseTill,
CloseAnotherPersonsTill; counted off the enum body). The same holds for
`CoreError`: 20 variants (`crates/core/src/error.rs`), six of them retail
words (DuplicateBarcode, PaymentAboveDebt, CreditLimit, Unstamped,
UnpricedReversal, PartyIds). Either the kernel keeps naming them, or retail
gets its own error type and `crates/api/src/error.rs` (487 lines) maps two
enums instead of one. This page assumes the first. `models/sql_types.rs`
defines 10 column enums through its `text_enum!` macro (count of the
invocations); `Role` is the one the kernel needs, the other nine
(DocumentKind, MovementKind, PaymentMethod, DebtKind, PartyKind,
DocumentStatus, SupplierDebtKind, PurchaseStatus, Unit) are shop words, so
that file splits in two.

The gates table: `crates/api/src/gates/table.rs` has 67 rows (count of
`Gate {`), 25 on kernel paths and 42 on retail paths, classified by the first
path segment. `router.rs` has 90 `.route(` calls, 31 kernel and 59 retail,
same method. With one api crate the retail rows and routes sit under
`#[cfg(feature = "retail")]` blocks in the same two files; the table stays one
`const &[Gate]` (`gates/mod.rs:108`) and `gate_for` (`:111`) is unchanged.
With one api crate per trade instead, `route_gates.rs:174` reads the router
as `include_str!("../src/router.rs")` and would never see a second router,
and the table would have to be assembled at startup from two slices, which
is the runtime registry the decision above ruled out for permissions. That is
why the api stays one crate. `crates/api/src/daily.rs:25` imports
`services::stock::Report` for the nightly recount, so that chore takes a cfg
too.

The generated DTOs: 122 at 06308cd, counted three ways that agree (occurrences
of `derive(... TS ...)` under `crates/api/src/dto`, files in
`packages/shared/src/generated`, and `FILES: [&str; 122]` in
`crates/api/tests/export_bindings.rs`); 40 are on kernel DTO files and 82 on
retail ones. The brief said 111; that figure is not what the checkout holds.
Nothing happens to them: the DTO modules stay in the api crate, `just types`
runs one export test into one folder, and a shop build's TypeScript client
carries the types of trades it was not built with, the way the plan says a
shop build carries unused permission variants.

The desktop routes: 45 non-test files under `apps/desktop/src/routes`, 15 of
them settings, audit, kit, index and root, 30 retail screens, counted by
file name. They do not move. `apps/desktop/eslint.config.js` says in its
header that the import rules of `frontend-conventions` are written against a
`features/` tree the app does not have; this candidate does not need it.

The schema: `crates/core/src/schema.rs` is one hand-written file (its header:
"Hand-written to match that SQL"), 31 `table!` blocks, 10 kernel tables and
21 retail, plus one `allow_tables_to_appear_in_same_query!` naming all 31 and
the `joinable!` lines from `:493`. The retail crate would hold its 21
`table!` blocks and its own `allow_tables_to_appear_in_same_query!` naming
the kernel tables by path. Migrations: 19 folders embedded by
`embed_migrations!("migrations/")` at `crates/core/src/db.rs:7`; 5 touch
kernel tables only, 10 retail only, 4 both (init, documents, 000008,
000009; counted by the table names each `up.sql` creates or alters). Under
Samir's cheap answer in D5 the folder stays one and lives in the kernel, so
a shop build embeds every trade's tables. The retail crate then declares
`table!` blocks for SQL that lives in another crate: two crates, one truth,
and the migration tests that open a previous version stay in the kernel.

Files edited rather than moved: the workspace `Cargo.toml`, four crate
`Cargo.toml`, `router.rs`, `gates/table.rs`, `routes/mod.rs`, `dto/mod.rs`,
`daily.rs`, and `apps/desktop/src-tauri/src/lib.rs:381`. 11 files.

### 3. On a Windows 7 machine

The binary does not start, and the shape has nothing to do with it. The
workspace builds with rustc 1.98.1 (`rustc --version` on the WSL box;
`.github/workflows/ci-restricted.yml:21` pins `stable`). Rust 1.78 raised the
minimum to Windows 10 for `x86_64-pc-windows-msvc` and every other
`*-pc-windows-*` target (release notes, compatibility section, fetched
2026-09-21). The `x86_64-win7-windows-msvc` target is tier 3, ships no
prebuilt standard library and needs `build-std` or a self-built toolchain
(rustc book, platform support page, fetched the same day). On top of that
the WebView2 runtime stopped at 109 on Windows 7, as the constraints block
says. So a Dinar built the way the release workflow builds it
(`.github/workflows/release.yml:171`, `windows-latest`, nsis) is a Windows 10
product today. The two escape hatches are a rustc pinned at 1.77 with a
dependency fallout nobody has counted, or nightly plus `build-std`; neither
is made cheaper or dearer by splitting a crate.

### 4. How a money write stays one transaction

`crates/core/src/services/sales.rs:228` opens `conn.transaction(|conn| ...)`
and, inside the closure, calls `documents::issue`, the stock movement, the
ledger and the idempotency row on that one `&mut SqliteConnection`. Every
kernel entry point takes the same connection type (`services/mod.rs:40`,
`:82`; `services::audit` the same), so an audit row written from inside the
sale's closure rides the sale's transaction exactly as now, only the path in
the `use` line changes. One process, one connection behind one mutex
(`crates/api/src/lib.rs:62`, `AppState.conn`), one closure in the retail
crate. Nothing is given up, so nothing has to be got back.

### 5. Which of the source walks and the permission match survive

The five walks, found by grepping the test folders for tests that read
source under `CARGO_MANIFEST_DIR`: `crates/core/tests/services_go_through_services.rs`
(4 tests over `src/services`), `crates/core/tests/repos_own_the_queries.rs`
(1 test, same folder), `crates/api/tests/route_gates.rs` (`include_str!` of
`router.rs`), `crates/api/tests/one_handler_decides.rs` (walks `src/routes`),
and `apps/desktop/src/theme.test.ts` (walks the screens).

With the api and the desktop staying single, the two api walks and the
desktop ones survive untouched in the default build. `route_gates.rs` reads
`router.rs` as text and cannot see a `#[cfg]`, so a build with the feature
off has 90 declared routes against fewer compiled rows and the walk fails by
construction; it is green for the default build only, which is a con of the
cfg blocks. The two core walks each see only the crate
they live in after the split: the ring test stops seeing a kernel-to-retail
ring because cargo refuses a dependency cycle between crates at all, which
is a stronger rule than a text walk; the repo-reach test stops seeing a
retail service reaching a kernel repo because `pub(crate) mod repos` makes
that a compile error. Both files are copied into `crates/retail/tests` with
their lists cut down (`REACHES_PAST_A_SIBLING` is two rows, both retail;
the three services that talk to SQLite directly are all kernel). Two walks
get narrower and the compiler picks up what they lose. The exhaustive
permission match is untouched by Samir's decision and moves as one file.

The rule D4 wants, that nothing in permissions, audit, settings or the money
kernel names a sale, fails on the day of the split under this candidate,
because the permission enum and `CoreError` name sales by design (section
2), and `crates/core/src/print/mod.rs:66` is `pub fn number(doc: &Document)`
with `Document` a retail model; so are `escpos.rs:16` and `strings.rs` (7 and
6 code lines naming `Document`, counted outside comments). D4 has to exempt
the two enums and the print engine, or the dictionary and `escpos` go to
retail with the templates and the kernel keeps `layout`, `raster`, `bidi`,
`png`, `thermal`, the five files with no `Document` on a code line. The
second is one more move and the one D4 can then check.

### 6. Pros, cons, the one thing that kills it

Pros. It is a rearrangement of files the compiler checks at every step: a
wrong move fails to build, not to run. Money writes, the permission match,
the api walks and the desktop are untouched. The retail crate cannot reach a
kernel repo or import a kernel cycle because Rust says so, not because a
test says so. A shop build links no other trade's Rust, the one place a
second trade's code could otherwise cost a till a bug. With
`default = ["retail"]` the product does not change until a second trade is
written, so the split can be done and left.

Cons. 63 source files and 48 test files move for no user; there are none
(the plan: "no users yet"). The kernel still names retail in three places
and the shared migration list means it embeds retail tables, so the crate
boundary protects repos and cycles and nothing else the source walks do not
protect today. The retail crate's `table!` blocks describe SQL that lives in
the kernel's folder. The TypeScript side gains nothing and carries every
trade's types. `pub(crate)` helpers become `pub`. A second trade sharing
`documents` takes it as retail shaped it, or the kernel grows a documents
service and the split is done twice.

What kills it. The kernel cannot avoid naming the shop: its error enum, its
permission enum, nine of the ten column enums in `sql_types.rs` and, under
the shared migration list, its schema all carry retail words on day one. If
the reason for the split is "adding a module
cannot break an existing one", the compile-time wall gives that for repos
and cycles only, and D4's source-walk rule gives the rest without creating a
crate. If the day comes when a second trade cannot live inside the kernel's
enums and tables (D3's tear list on `documents`), this candidate has no
answer that is not candidate 4's trait per extension point, so it is either
a folder rename with a `Cargo.toml` or the first half of a different shape.

### 7. Noticed and not pursued

- Windows 7 (section 3) is a toolchain fact, not a shape fact: it rules
  Windows 7 out for candidates 1 to 4 and for the WebAssembly host in 5.
  Only a .NET Framework or an old Electron build in candidate 5 can say
  anything else, and that page should say what .NET version still installs
  on Windows 7 SP1.
- Candidate 4 is where this one's killer leads: a trait the kernel calls and
  retail implements would let `CoreError` and `Permission` stop naming a
  sale; that page should price it against the 10 and 6 variants counted here.
- Candidate 6 (fork per trade) is the baseline this page's 63 moved files
  should be compared against: a fork moves zero files and duplicates the
  kernel's quarter, which the plan measures as the cheap quarter.
- The plan page is stale on the rings (section 2) and calls `schema.rs`
  "one generated file" when its header says hand-written; bookkeeping.
- `crates/api/src/daily.rs` runs a stock recount beside the backup, a retail
  job in the api's own file; candidates 2 and 3 have to say which process
  owns it.
