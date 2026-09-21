# Candidate 5: a full rewrite on a stack built for plugins

D7 brainstorm page, judged against the constraints block in
`context/plans/20260921-whether-dinar-becomes-a-core-and-modules.md`. Written
2026-09-21 from `main` at ea4caf8. Three sub-cases, priced rather than imagined:
(a) .NET on Windows, (b) Electron or a web app with a local server and
JavaScript plugins, both rewrites priced by what is rebuilt, and (c) the current
Rust core hosting WebAssembly modules, an addition priced by what is added.
Every line count below is `find -name '*.rs' | xargs cat | wc -l` (the same for
`.ts` and `.tsx`), so comments and blanks are in; the plan page's 44 793 is
code-only and not compared.

## What a rewrite has to reproduce, counted from the repo

Shipped Rust: `crates/core/src` 32 993 lines (`services` 16 667, `repos` 5 558,
`models` 3 258, `print` 5 094 in 16 files, `money` 1 020 in 5 files),
`crates/api/src` 10 092, `crates/seed` 414, `apps/desktop/src-tauri/src` 939;
44 438 together. Rust tests: `crates/core/tests` 41 035 lines and
`crates/api/tests` 15 211, 56 246 together, in 90 files
(`ls crates/*/tests/*.rs | wc -l`, one cargo test binary each), holding 1 054
`#[test]` and 314 `#[tokio::test]` (`grep -rc` over `crates`) and 9 `proptest!`
blocks. 35 service files under `crates/core/src/services` and 19 route files
under `crates/api/src/routes` (`mod.rs` excluded from both), 90 `.route(` lines
in `crates/api/src/router.rs` over 60 distinct paths
(`grep -oE '\.route\("[^"]+"' | sort -u`), 19 migration folders under
`crates/core/migrations`. The fiscal rules table at `docs/features.md:759` has
17 rule rows (19 table lines less header and separator). 120 fixture files under
`fixtures/` (`find -type f`): 8 in `fixtures/money`, 112 print goldens. 9 HTML
templates in `crates/core/templates`, rendered by the core in three languages,
plus the ESC/POS raster path (`print/escpos.rs`, `raster.rs`, `png.rs`). Words:
818 keys per language in `apps/desktop/src/i18n/{fr,ar,en}.json` (leaf count),
77 print keys times three languages at `crates/core/src/print/strings.rs:206`
(`ALL: [Key; 77]`), about 80 per language under `packages/shared/src/i18n`.
Desktop: 45 `.tsx` files under `apps/desktop/src/routes` that are not tests,
41 774 lines of `.ts` and `.tsx` including tests, 605 vitest cases (`it(` and
`test(`) in 41 files. `packages/shared`: 7 972 hand-written lines plus 1 566
generated in 122 files, written by `ts-rs` 12.0.1 (`crates/api/Cargo.toml:15`).
Phone: an Expo app, 7 screen files under `apps/mobile/app`, 4 067 lines, 48
vitest cases in 9 files, talking HTTP to the api.

Two facts decide what survives a rewrite. `docs/architecture.md` ("Consequence
of rule 2") says the desktop webview talks to a localhost HTTP server, not Tauri
IPC, so any rewrite that reproduces the 60 paths keeps the React app,
`packages/shared` and the phone. And the 120 fixtures are language-neutral
golden files that transfer to any stack whole: the one part nobody pays twice.

## (a) .NET on Windows, plugins as assemblies

**1. Shape.** A C# solution: a core assembly (money, users, sessions,
permissions, audit, settings, printing, backup), a host that opens the shop file
with `Microsoft.Data.Sqlite` and loads trade assemblies from a `modules/` folder
at start (`AssemblyLoadContext` on modern .NET, `Assembly.LoadFrom` on Framework
4.8), and a UI in WPF or WinUI 3. The shape most Windows till software has.

**2. Price.** Everything in the count above except the fixtures and the spec
rows. With a WPF or WinUI front, the 41 774 lines of React and the 605 vitest
cases go too, 45 screens rewritten in XAML, and `packages/shared` with them; only
the phone survives, if the C# server reproduces the 60 paths and the wire shape
`ts-rs` writes today (OpenAPI plus NSwag would take that role). So: about 44 400
lines of shipped Rust, 56 200 lines of Rust tests, 41 800 lines of desktop
TypeScript and 9 500 lines of shared TypeScript written again in C#, plus 19
migrations, 9 templates and the ESC/POS raster. Keeping the React UI inside
WebView2 saves the front end at the WebView2 floor of the constraints block.

**3. Windows 7.** Verified 2026-09-21 from `dotnet/docs`,
`docs/framework/get-started/system-requirements.md` line 75: "| Windows 7 SP1† |
32-bit and 64-bit | -- | .NET Framework 4.6.2 [...] .NET Framework 4.8 |", with
line 77 marking Windows 7 "out-of-support". The row stops at 4.8; 4.8.1 is not
in it. On modern .NET the task's
premise is half right: `dotnet/core` `release-notes/6.0/supported-os.md` line
191 reads "Windows | 7 SP1 | 2020-01-14", under the heading at line 120, "## Out
of support OS versions", so .NET 6 shipped with Windows 7 SP1 and dropped it;
`release-notes/8.0/supported-os.md` has no Windows 7 row anywhere (a grep for
"7 SP1" finds nothing). From memory, not fetched: .NET 6 itself left support in
November 2024. WinUI 3 is out on Windows 7 by the Windows App SDK page,
verified: "Windows App SDK APIs run on Windows 11 and earlier versions starting
from Windows 10, version 1809." So (a) on a Windows 7 till means WPF on .NET
Framework 4.8: a runtime frozen in 2019, still serviced as a Windows component
(both memory).

**4. The money write.** In-process, so it is easy. The host opens a
`SqliteTransaction`, calls the core's issue routine, then hands the open
connection and transaction to each module's `OnSale(sale, conn, tx)`; a module
that throws unwinds the whole thing. The closure at `sales.rs:228`, another syntax.

**5. Guarantees.** Checked centimes survive as a compiler setting: `long`
centimes inside `checked { }` blocks or a project-wide overflow check (memory).
The exhaustive match at `permissions.rs:206` does not: a C# `switch` expression
missing a case is warning CS8509, an error only under `TreatWarningsAsErrors`
(memory), and the point of a runtime-loaded assembly is that the core's `switch`
cannot name a module's permission, which is the runtime registry Samir turned
down. Gapless numbering survives: `repos/counters.rs:25` is three SQL statements
inside the caller's transaction, the same in any language. The five walks
(`crates/api/tests/route_gates.rs`, `include_str!` of `router.rs` at `:174`;
`one_handler_decides.rs`; `crates/core/tests/services_go_through_services.rs`;
`repos_own_the_queries.rs`; `apps/desktop/src/theme.test.ts`) read Rust and
TypeScript source and die with it; their replacements are Roslyn analyzers or
reflection tests over the loaded assemblies, written from scratch, and clippy's
`unwrap_used` deny (`Cargo.toml:25`) has no C# twin.

**6. Pros, cons, the killer.** Pros: the plugin loader is a solved problem on
.NET; a native printer stack; the one sub-case with a supported runtime on
Windows 7. Cons: the largest price on the page, every line including the UI; a
suite of 1 368 Rust cases and 653 vitest cases rebuilt by hand; the permission
table becomes a registry. The killer: rewriting a fourteen-day-old repo with no
user into a language nobody here writes, for a Windows 7 machine nobody has seen.

## (b) Electron, or a web app with a local Node server, JavaScript plugins

**1. Shape.** The React app stays. Behind it, a Node server in TypeScript
replaces `crates/api` and `crates/core`: `better-sqlite3` on the same shop file,
the 19 migrations rewritten, plugins as npm packages `require`d into the server
process at start, each exporting hooks (`onSale`, routes, screens). Electron
bundles that server with Chromium; the "plain web app" variant runs it as a
Windows service and opens the shop's browser.

**2. Price.** What survives: 41 774 lines of desktop TypeScript and its 605
tests, `packages/shared` (whose `totals.ts`, 261 lines, and `money.ts`, 64
lines, compute totals in TypeScript pinned by the same 8 `fixtures/money/*.json`
the Rust side reads at `crates/core/tests/money_fixtures.rs:13`, so the money
kernel exists twice today), the phone, the fixtures. What is rebuilt:
`crates/core/src` plus `crates/api/src`, 43 085 lines, and the 56 246 lines of
Rust tests behind them; 19 migrations; 9 templates re-rendered from Node, and
the ESC/POS raster path (`print/png.rs`, `raster.rs`, `bidi.rs`) on a Node image
library; the 77 print keys times three. Roughly two thirds of the lines of (a).

**3. Windows 7.** Verified from the Electron blog "Farewell, Windows 7/8/8.1":
"Electron 22 will be the last Electron major version to support Windows versions
older than 10. Windows 7/8/8.1 will not be supported in Electron 23 and later
major releases." and "May 30 2023: Electron 22 reaches the end of its support
cycle." Also verified there: "Electron 22, which contains Chromium 108, will
thus be the last supported version", and Electron 23 "will contain Chromium
110", so Chromium 109 is the last on Windows 7; that Electron 22 bundles Node 16
is memory. The web-app variant has no Windows 7 story of its own: Node's
`BUILDING.md` line 118, verified: "| Windows | x64 | >= Windows 10/Server 2016
| Tier 1 |", and the browser would be Chrome 109. So (b) on Windows 7 is an
engine and a runtime that stopped receiving fixes in 2023, holding the money.

**4. The money write.** `better-sqlite3` transactions are synchronous; its
`docs/api.md` line 102, verified: "Transaction functions do not work with async
functions. Technically speaking, async functions always return after the first
`await`, which means the transaction will already be committed before any async
code executes." So the sale is one `db.transaction(() => {...})` and every
plugin hook inside it has to be synchronous; a plugin that awaits anything has
silently left the transaction, and only a rule stops it, not the runtime.

**5. Guarantees.** Checked centimes do not survive as a type: JavaScript's
`number` is an `f64`, exact below 2^53 (which `docs/architecture.md` accepts on
the wire), and no arithmetic overflows into an error; `BigInt` cannot overflow
and throws on mixing with `number`, a runtime check; `totals.ts` holds the line
by fixtures, not by the compiler. The exhaustive match survives: a TypeScript
`switch` with a `never` default is a compile error on a missing case, so the
enum and match at `permissions.rs:39` and `:206` port one for one; a JavaScript
plugin cannot add to a TypeScript union any more than a `.dll` can add to a C#
`switch`, so the same registry question returns. Gapless numbering survives.
`theme.test.ts` is a vitest walk already; the four Rust walks become vitest
walks over the server source or ESLint `no-restricted-imports` rules. A
`require`d plugin runs with the server's full rights, no sandbox, unlike (c).

**6. Pros, cons, the killer.** Pros: the cheapest rewrite, one language on both
sides, the UI and the phone untouched, plugin loading is `require`. Cons: the
money kernel moves into a language with no integer type; native image and
SQLite modules rebuilt per Electron version; Electron's footprint on a till.
The killer: it exists only to reach Windows 7, and the Windows 7 it reaches is
Electron 22, dead since May 2023; on Windows 10 it is a rewrite of 43 000 lines
that gives back exactly what `crates/api` gives today.

## (c) The Rust core hosting WebAssembly modules

**1. Shape.** Nothing is rewritten. `crates/core` gains a host: `wasmtime`
directly or `extism` on top of it, a `modules/` folder read at start, a manifest
per module, and a small set of host functions the module may call. A trade
module is a `.wasm` file, written in Rust compiled to `wasm32` (then it could
depend on the money code, which does no I/O, once that is lifted out of
`crates/core` into a crate without diesel) or in any language with a wasm
toolchain. Extension points are named calls such as `on_sale`.

**2. Price, as additions.** The `wasmtime` dependency (Cranelift and the
runtime; from memory a heavy build on the shared `.cargo-target`), or `extism`,
which brings it in. A host ABI: about a dozen host functions and their JSON
shapes. Module loading, manifests, versioning. Per-module migrations run by the
host, the D5 question unchanged and the largest piece. A second statement of
every walk's intent for code the walks cannot see. And no Windows 7 story: the
host inherits candidate 1's floor (`01-compile-time-domains.md`, section 3: Rust
1.78, Windows 10 for every `*-pc-windows-*` target), under Tauri on WebView2.

**3. Windows 7.** wasmtime, `docs/stability-tiers.md` line 26, verified:
"| Target | `x86_64-pc-windows-msvc` |" in the Tier 1 table;
`docs/stability-platform-support.md` line 71: "OS support at this time
primarily includes Windows, macOS, and Linux." Extism `README.md` lines 47 and
48 under "Supported Targets": "- x86_64-pc-windows-gnu" and
"- x86_64-pc-windows-msvc".
"Windows 7": not found on any of the four pages. It does not matter: the Rust
host does not start on Windows 7 before wasmtime gets a say.

**4. The money write.** The host keeps the closure at `sales.rs:228`. Between
`documents::issue` and `stock::record`, it calls the module's `on_sale` with the
document as JSON and, for the length of that call, exposes host functions bound
to the same `&mut SqliteConnection`: write a row into a module table,
`take_next` a counter (`repos/counters.rs:25`), `audit::record`,
`permissions::require`, read a setting. A module error or trap returns `Err`
from the closure and diesel rolls back every row, the host's and the module's.
That is what a module can do inside the transaction. What it cannot do: open
the shop file itself (the one `Conn` sits behind `Arc<Mutex<Option<Conn>>>` at
`crates/api/src/lib.rs:68`, and a second connection meets the writer lock);
hold the transaction past the call; touch diesel types; add a `Permission`
variant (one enum at `permissions.rs:39`, no runtime registry, by Samir's
decision); create a table the 71 foreign key clauses counted in
`03-data-separated-per-module.md` know, unless the host runs module migrations;
and its centimes are whatever its language offers. Both runtimes' store and
host-function lifetime rules are unverified here, so that plumbing is unpriced.

**5. Guarantees.** For the core, all of them survive untouched: the five walks
still walk `crates/core/src`, `src/routes` and the screens, the match is still
exhaustive, clippy still denies `unwrap`, the counter is still gapless. For
module code, none of them apply: clippy does not see a `.wasm`, the walks do not
read it, `Permission` cannot name what it needs, and checked centimes hold only
if the module is Rust using the money crate, which nothing forces. The host ABI
replaces them: a module does only what a host function lets it, a stronger
sandbox than (a) or (b) and a weaker compile-time story than today.

**6. Pros, cons, the killer.** Pros: the current code stays; the only real
sandbox on the page; modules update without rebuilding Dinar. Cons: the heaviest
dependency in the workspace for no user; JSON at every boundary; the schema
question open. The killer: the plan page's candidate 4 (a trait per
extension point, modules registered at startup, one binary) gives the same
module shape with no wasm boundary and keeps every compile-time guarantee for
module code too. (c) pays for itself only when somebody outside the repo writes
a module without recompiling Dinar; there is no shop, no pilot, no third party.

## Ranking

Everything turns on the constraints block's first job, whether Windows 7 is a
target. If it is dropped, every rewrite loses to no rewrite on price alone: (a)
is about 152 000 lines rebuilt (44 400 shipped Rust, 56 200 test Rust, 51 300
TypeScript), (b) about 99 000, and (c) is not a rewrite but loses to candidate
4 on weight. If Windows 7 stays, only (a) on WPF and .NET Framework 4.8 runs
there on a runtime still serviced, (b) runs there on Electron 22, dead since
May 2023, and (c) does not run there at all. Against the "no rewrite" baseline
the plan page names (candidate 6, fork per trade, "the baseline every other
candidate has to beat", with candidate 1 the cheapest measured): baseline
first; then (c), which costs least and keeps the code, and should still wait
for a third party to exist; then (b), the cheaper rewrite, the one that keeps
the UI and the phone, taken only if Windows 7 is real and a dead engine on the
till is accepted, which it should not be; (a) last, the largest price on the
page, worth it only if Windows 7 is real and the shop refuses an unpatched
engine, and even then a pinned rustc 1.77 (candidate 1, section 3) is costed
first. In one line: no rewrite; if Windows 7 forces the question, price the
rustc pin, then WPF on Framework 4.8, and never Electron 22.
