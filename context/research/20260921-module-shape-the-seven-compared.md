---
title: 'Module shape: the seven candidates compared'
date: 2026-09-21
status: 'done'
tldr: 'Seven shapes for a core plus trade modules were each argued on its own page under context/research/module-shape/ against the one constraints block on the decision plan. Two facts decide most of it: the Rust toolchain in use needs Windows 10, so Windows 7 is out for every shape that keeps Rust; and under WAL a SQLite transaction across two files or two processes is not one commit, so every shape with a second writer or a second file breaks the money rule. What is left is candidate 1 (crates), candidate 4 (in-process traits) and candidate 6 (change nothing until a second trade has a name). The recommendation is the sequence they imply: draw the boundary as a rule now (D4), change no structure until D3 names a tear list, then the crate move first and traits only when two real modules exist to extract them from. D6 stays open and waits on D2, D3 and D5.'
---

# Module shape: the seven candidates compared

Seven pages, one per candidate, each written by its own agent on the sharper
model against the constraints block of
`context/plans/20260921-whether-dinar-becomes-a-core-and-modules.md` (D7),
reading the repo at `06308cd` to `97cc5c9`. This page decides between them.
It does not write D6: the plan says D6 waits on D2, D3 and D5, and the last
section says what D6 needs from these seven.

The pages: `module-shape/01-compile-time-domains.md`,
`02-local-microservices-one-database.md`, `03-data-separated-per-module.md`,
`04-in-process-modules-on-the-current-stack.md`,
`05-rewrite-on-a-plugin-stack.md`, `06-fork-per-trade.md`,
`07-the-odoo-shape.md`.

### Three facts the verdict rests on, each read again here

The Rust floor. The workspace builds with rustc 1.98.1 (`rustc --version`
on the WSL box, 2026-09-21; `.github/workflows/ci-restricted.yml:21` pins
`stable`). Page 01 §3 fetched the Rust 1.78 release notes: from 1.78 the
minimum for every `*-pc-windows-*` target is Windows 10, and the
`x86_64-win7-windows-msvc` target is tier 3 with no prebuilt standard
library. So the binary does not start on Windows 7 today, whatever shape
the code has. This removes Windows 7 for candidates 1 to 4 and for the
WebAssembly host in 5.

WAL and atomicity. `crates/core/src/db.rs:93` opens the file with
`PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout =
5000;`. Page 03 quoted SQLite's ATTACH page: transactions across attached
databases are atomic only when the journal mode is not WAL. Page 02 counted
39 `.transaction(` sites in `crates/core/src/services`, all deferred
begins, and showed that a second writer turns each into a rollback or a
five-second stall. So one file, one process, one connection is what keeps a
sale one commit, which is constraint 3 of the block.

The foreign keys. Page 03 counted, from the last `CREATE TABLE` of each of
the 31 tables, 71 foreign keys: 32 trade-to-core (21 to `shops`, 11 to
`users`), 26 trade-to-trade, 13 core-to-core, 0 core-to-trade. That zero is
what makes a kernel carvable at all: nothing the shared quarter owns points
at a retail row. It also says where the shop still lives inside the kernel
(page 01 §6 and page 06 §2 list the same six places: the error enum, the
permission enum, nine of ten column enums, `print::number`, the shared
migration list, and two raw counts in backup and the support bundle).

### The decision tree

First question: is Windows 7 a target. Every page answered it. If yes, the
only shape that runs there on a serviced runtime is candidate 5a, WPF on
.NET Framework 4.8 (page 05 §3: .NET 6 shipped with Windows 7 SP1 and
dropped it, 8 has no row; Electron stopped at 22 in May 2023; Node needs
Windows 10; WinUI 3 needs 1809), at the price page 05 measured: about
152 000 lines rebuilt (44 438 shipped Rust, 56 246 of tests in 90
binaries, 45 screens, 818 i18n keys times three, 120 fixtures, a 7-screen
phone app). If no, every rewrite loses to no rewrite on price alone and
the Rust shapes stay on the table. Samir's expectation of 2026-09-21 is a
Windows 10 machine, possibly an older one; nobody has a shop yet, so the
first real shop confirms it. This page proceeds on Windows 10.

Second question: does a second trade exist with a tear list. D3 is the
task that produces one, and D2 the research it needs; neither is done.
While the answer is no, the baseline holds: candidate 6 not taken, which
costs nothing (page 06 §7: the shared quarter changes fourteen times a
day, so a copy taken now is a copy of something still being written), plus
D4's rule, which names the six places above and fails a build that adds a
seventh. When the answer is yes, the order is candidate 1's crate move
first (page 01 §2: 63 files move, 35 import edges rename, 11 files edit,
48 of 59 core tests go with them), then candidate 4's traits drawn from
the two real implementers, never invented ahead of the second one. That is
candidate 4's own killer (page 04 §8: every trait method has one
implementer on the day it is written) turned into the sequencing rule, and
page 06 §8 reaches the same order from the fork's side.

### The candidates against the constraints

The columns are the block's constraints in its order: one offline PC and a
phone, Windows (10; 7 in the note), money atomic, adding a module cannot
break one, less complex wins, permissions one list. Each cell names the
page section that carries it.

| Candidate | Offline PC | Windows 10 / 7 | Money atomic | Cannot break | Less complex | One permission list |
|---|---|---|---|---|---|---|
| 1 crates | pass | pass / fail (01 §3) | pass, unchanged (01 §4) | repos and cycles only (01 §5) | pass, a folder move | pass |
| 2 local processes | pass, but a proxy for the phone (02 §1) | pass / fail | fail: second writer, outbox instead (02 §4) | inverted: a hung module stalls a sale (02 §8) | fail: supervisor, proxy, registry | pass, copied per binary |
| 3a table prefix | pass | pass / fail | pass, a rename (03 a) | unchanged | pass | pass |
| 3b file per module | pass | pass / fail | fail under WAL (03 b §5) | conditional: 32 constraints replaced by triggers | fail: N files to back up and restore | pass |
| 3c a schema, so Postgres | fail: a server (03 c) | pass / fail | pass | pass | fail: 664 connection mentions in 99 files rewritten | pass |
| 4 in-process traits | pass | pass / fail | pass, the hook runs inside the closure (04 §4) | conditional: a compose test replaces the crate wall (04 §7) | conditional: a trait with one implementer (04 §8) | pass |
| 5a WPF on 4.8 | pass | pass / pass | pass | conditional: runtime plugins, no walks port (05 §5) | fail: the rebuild | conditional |
| 5b Electron or web | pass | pass / fail (dead engine or Node) | conditional: sync transactions only (05 b) | fail: JS plugins, f64 money | fail | conditional |
| 5c Rust host, wasm | pass | pass / fail | pass, a trap rolls back (05 c) | conditional | fail: the host is candidate 4 plus a runtime | pass |
| 6 fork | pass | pass / fail | pass | met trivially (06 §6) | pass today, fail as the copies drift (06 §3) | pass |
| 7 Odoo shape | fail: Postgres and Python (07 §6) | fail / fail | conditional: a dynamic ORM (07 §5) | fail by design (07 §8) | fail | fail: runtime permissions |

### Eliminated, one line each

- Candidate 2: a module becomes a second writer on the till's file, so a
  slow module transaction stalls or fails a sale through the lock, the exact
  failure Anouar asked to be protected from and one that cannot happen today.
- Candidate 3b: under WAL a sale that writes `counters` and `audit_log` in
  the core file and its lines in the module file is two commits; of the ten
  tables a sale writes, two are core, and 23 of 36 services call
  `audit::record`.
- Candidate 3c: a database server on the shop PC fails the first constraint
  and touches 664 `SqliteConnection` mentions in 99 files.
- Candidate 5, all three: on Windows 10 every rewrite loses to no rewrite on
  price; 5a is the only shape with a Windows 7 story and costs the whole
  product; 5c is candidate 4 with a runtime boundary the walks cannot see.
- Candidate 7: Odoo's mechanism is built to permit exactly what Anouar asked
  to prevent (a module adds a column to another's table and overrides its
  methods at runtime); the parts of it that carry safety (a registry,
  module routes, gate rows, per-module migrations, side tables with foreign
  keys) are what candidate 4 already gives, and Anouar's answer names
  nothing that needs the rest.
- Candidate 3a: kept, as a table-naming rule inside candidate 1, not as a
  separation.

### What each of the three that remain is for

Candidate 6 not taken is the present: it is what the repo is doing tonight,
and it is right while the shared quarter moves fourteen times a day and no
second trade has a name. Its own page says when it stops being right: a
second trade within the year, or the shared-only commit rate near zero for
weeks, or a second trade that reuses none of `documents`, `counters` and
the fiscal rules, or a second owner.

Candidate 1 is the first move once a tear list exists: a kernel crate and a
retail crate, `pub(crate) mod repos` and cargo's no-cycle rule replacing the
two core walks, the api and desktop walks unchanged, `default = ["retail"]`
so today's product does not change. Its page is honest that the kernel
names the shop on day one in six places, so the crate wall protects repos
and cycles only, and D4's rule gives the rest without creating a crate.

Candidate 4 is the second move, and only with two implementers in hand: a
`Module` trait with a method per extension point (routes as data, gate
slices, migrations, numbering, error status, chores, sale hooks), an
explicit module list at the two places a router is built, audit actions and
the two key/value tables as extension points at no cost, one bundle on the
desktop with a module folder mounted into the file router. Its page names
the hook contract that keeps a sale one transaction (same connection, no
I/O, no total change, `Err` unwinds) and the one spike it could not verify
(two route folders at one prefix). It also names the fork on `documents`
that both it and candidate 1 stall on until D3 says whether a clinic's
paper is a document in this file's sense.

The earlier read-only architecture review of 2026-09-21 reached "nothing
structural to dz-pos" from the code alone; seven independent arguments
landed on the same place with the sequencing added. That is evidence for
the recommendation, not a decision.

### Recommendation

Change no structure now. Do D4: the source-walk rule that names the six
places the shop lives inside the shared quarter and refuses a seventh.
Decide between the crate move and the traits only when D3 has a tear list,
and in that order. Confirm Windows 10 with the first real shop; if Windows 7
turns out to matter, the only honest answer is candidate 5a and the plan
page's cost section is the place to price it, not this one.

### What D6 needs from these seven

- The Windows 7 answer from a real shop; every Rust shape depends on it.
- D3's tear list on `documents`: candidates 1 and 4 both stall there.
- Candidate 4's desktop routing spike (two folders at one prefix, flat
  settings nesting), unverified.
- The claims each page flagged as from memory rather than fetched, to
  verify before they carry weight: Windows refusing a rename over an open
  file (02); `SQLITE_MAX_ATTACHED` defaulting to 10, insert failure on a
  foreign key to an absent parent table, `VACUUM schema INTO`, PostgreSQL
  10 as the last Windows 7 major (03); .NET 6's end of life in November
  2024, Framework 4.8 servicing, the severity of CS8509, Electron 22's Node
  16, wasmtime and extism store lifetimes (05); the .NET Framework line in
  the Windows 7 story (07 inherits 05).
- Three corrections carried back to the plan page tonight: the import
  rings are cut (`RINGS_STILL_OPEN` is empty at
  `crates/core/tests/services_go_through_services.rs:260`, T11 done);
  `crates/core/src/schema.rs` is hand-written by its own header, not
  generated; the DTOs number 122 (`crates/api/tests/export_bindings.rs`),
  not 111.
