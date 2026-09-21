# Candidate 3: modules with their data separated per module

D7 brainstorm page, judged against the constraints block in
`context/plans/20260921-whether-dinar-becomes-a-core-and-modules.md`. Written
2026-09-21 from `main` at ad34eb6; every number says how it was counted. Three
cases: (a) one file, a table prefix per module; (b) one SQLite file per module
`ATTACH`ed to the core's; (c) a schema per module, which means leaving SQLite.
"The sibling" is `context/research/module-shape/02-local-microservices-one-database.md`,
which owns the two-process analysis.

## The split every case starts from

31 tables (`grep -c 'diesel::table!' crates/core/src/schema.rs`; replaying every
`CREATE`, `DROP` and `RENAME TO` across the 19 folders and 1 843 lines of
`up.sql` under `crates/core/migrations` gives the same 31). Core, meaning a
clinic would carry them unchanged: `shops`, `users`, `sessions`, `audit_log`,
`settings`, `preferences`, `counters`, `jobs`, `pairing_tokens`,
`paired_devices`, 10 tables; trade, the other 21. Two are arguable, `counters`
(a module could number its own documents) and `customers` (the fiche the plan
page calls the same table for a patient); moving either changes no verdict.

Foreign keys, counted as `REFERENCES` clauses in each surviving table's last
`CREATE TABLE`: 71. 32 run trade to core, 21 of them to `shops` (rule 3 of
`docs/architecture.md`) and 11 to `users` (`shifts` names it twice); 26 trade
to trade; 13 core to core; 0 core to trade. That zero is the fact every case
leans on: the core never needs a module's row, so a missing module cannot fail
a core write. In diesel the same edges are 67 `joinable!` lines (`schema.rs`).

No SQL join crosses that line. `crates/core/src/repos/*.rs` holds 4
`inner_join`/`left_join` calls (`grep -c`): `repos/dashboard.rs:129` and `:173`
join `document_lines` and `stock_movements` to `documents`, and
`repos/cash_refunds.rs:138` joins `debt_ledger`, all trade to trade;
`repos/sessions.rs:50` joins `users`, core to core. The audit screen names a
user in Rust: `repos/audit.rs` has no join and `services/audit.rs:443` builds a
`HashMap` from `user_names` (`services/mod.rs:60`). The dashboard service
(`services/dashboard.rs`, 525 lines) calls `cash`, `debt`, `supplier_debt`,
`expenses` and five `repo::` functions (`grep -o`) and sums in Rust.

A sale, from the sibling's section 4 and the closure at
`crates/core/src/services/sales.rs:228`, writes ten tables, two of them core
(`counters`, `audit_log`), and 23 of the 36 service files call `audit::record`
(`grep -c`): with core and trade in two files, every sale and almost every
write crosses both.

## Case (a): one file, a table prefix per module

**1. Shape.** One shop file; the 21 trade tables become `retail_products` and
so on. `crates/core/migrations` splits into a core and a `retail` folder, each
with its own `embed_migrations!` (`crates/core/src/db.rs:7`) run on the same
connection (`db.rs:101`). Both write one `__diesel_schema_migrations` table
(diesel 2.3.13, `src/migration/setup_migration_table.sql`) keyed by version
string, so a module folder needs its own version prefix.

**2. Cost.** 21 `ALTER TABLE ... RENAME TO` statements, carried by SQLite
through foreign keys and the 34 `CREATE INDEX` lines. Then every spelling of a
table name: 21 of the 31 `table!` blocks in `schema.rs` (593 lines), a share of
the 57 `table_name =` attributes in `crates/core/src/models` (`grep -c`), 6 raw
SQL literals in shipped code (`services/backup.rs:527-528`,
`services/support_bundle.rs:127-129`, `repos/stock.rs:168`), and 430 literal
table names across 45 files under `crates/core/tests` and `crates/api/tests`
(`grep -o` on `FROM|INTO|UPDATE|TABLE <name>`), 268 of them in
`crates/core/tests/migration.rs` (55 `#[test]`, `grep -c`). Nothing else moves.

**3. Foreign keys.** Unchanged: all 71 clauses stay enforced by `PRAGMA
foreign_keys=ON` (`db.rs:93`, asserted at `migration.rs:2631`).

**4. Dashboard and audit.** Unchanged under new names; one audit table.

**5. Money atomicity.** Unchanged: one connection behind one mutex
(`crates/api/src/lib.rs:68`), 39 `.transaction(` lines in 20 service files
(sibling section 2), one journal. A two-process (a) inherits the sibling's
sections 3 and 4 whole.

**6. Backup, restore, leak scan.** One file, so `VACUUM INTO` (`backup.rs:416`),
the eight-step restore (`lib.rs:177-188`), the prune (`crates/api/src/daily.rs`)
and the bundle (`support_bundle.rs:162`) stay; five count literals get a prefix.

**7. Windows 7.** Nothing changes; the sibling's section 5 is the answer.

**8. Tests.** All five source walks survive untouched, none reading a table
name: `crates/api/tests/route_gates.rs`, `one_handler_decides.rs`,
`crates/core/tests/services_go_through_services.rs`, `repos_own_the_queries.rs`
(its allow lists at lines 27 and 37 are file names), `scripts/file-sizes.mjs`.
The match at `crates/core/src/services/permissions.rs:205` is untouched.

**9. Pros, cons, the killer.** Pro: a rename, every guarantee kept. Con: it
separates nothing SQLite enforces; a retail service can still `UPDATE users`,
and what stops that today is a Rust walk (`services_go_through_services.rs:182`,
a per-service repo allow list). The killer: it is not a data separation, it is
candidate 1 or 4 plus a naming rule and a second migration folder.

## Case (b): one SQLite file per module, ATTACHed to the core's

**1. Shape.** `dinar.sqlite` for the 10 core tables, `retail.sqlite` for the 21
trade tables. `db::open_unmigrated` (`db.rs:84`) opens the core file then runs
`ATTACH DATABASE 'retail.sqlite' AS retail`. The 21 trade `table!` blocks become
`retail.products (id) {...}`, which diesel parses
(`diesel_table_macro_syntax-0.3.0/src/lib.rs:8,45,109`, tested at
`diesel-2.3.13/src/macros/mod.rs:521`) and emits as `"retail"."products"`, so
the repos need no edit. The harness's bookkeeping table is created unqualified
and lands in `main`, so the module file is migrated on its own connection, then
attached; `journal_mode` is per file, so `PRAGMA retail.journal_mode` joins
`db.rs:93`; `SQLITE_MAX_ATTACHED` defaults to 10 (from memory).

**2. Cost.** The 32 trade-to-core `REFERENCES` clauses are deleted, not left:
with `foreign_keys=ON` a child whose parent table is not in its own database
fails the insert (SQLite behaviour from memory, to verify on a file built that
way): a rewrite of 21 `CREATE TABLE`s, or a rebuild migration in the shape of
`2026-09-10-000007`. The 26 trade-to-trade and 13 core-to-core clauses stay.
`schema.rs`: 21 prefixes; the 67 `joinable!` stand because one connection sees
both schemas. `db.rs`: an ATTACH, a second migrate path, per-file pragmas.
Tests: the 22 files opening a temp file (`grep -l 'db::open|testdb'`) build two.

**3. Foreign keys.** SQLite enforces none across attached files. In place of the
32: the 21 `shops` edges are covered by rule 3's filters, 361 `shop_id` mentions
across the 25 repo files (`grep -c`, summed), and by one file being one shop
(`docs/architecture.md`, Data); the 11 `users` edges become a read inside the
transaction, which the services do anyway to resolve the actor; `ON DELETE` is
gone, and the one replacement SQLite offers is a `TEMP` trigger, which
`lang_createtrigger.html` section 2.1 lets reach any attached database where an
ordinary trigger may not, recreated at every open, firing for this process
only. Users are switched off, never deleted (`docs/architecture.md`, the
last-owner rule), so that edge matters less.

**4. Dashboard and audit.** The two joins at `repos/dashboard.rs:129,173` run
inside `retail`; the Rust fan-out reads trade services only; the audit screen
resolves names in Rust and is unchanged, and a cross-file join would work
anyway, `schema-name.table-name` being ordinary SQL across attached files
(`lang_attach.html`, Details). The bundle's `sqlite_master` walk
(`support_bundle.rs:162`) reads `main` only and needs a second pass.

**5. Money atomicity.** This is where (b) turns. `lang_attach.html`, fetched
2026-09-21: transactions across attached databases are atomic only when the
journal mode is not WAL; under WAL each file commits on its own, and a crash in
the middle of a COMMIT touching two files can leave one updated and the other
not. `db.rs:93` sets WAL. A sale writes `retail` and `main` (`counters`,
`audit_log`), so a power cut can leave a numbered sale whose counter did not
advance, the gapless-numbering failure `docs/features.md` forbids. Three ways
out, each a choice: leave WAL for the rollback journal, which one connection
behind one mutex (`lib.rs:68`) gets no concurrent reader out of anyway, at the
price of retiring `db::checkpoint` (`db.rs:51`) and restore steps 5 and 8
(`lib.rs:182-188`) and changing every write's durability; or move `counters`
and `audit_log` into the module file so all ten tables a sale writes sit in
`retail`, at the price of an audit screen reading a `UNION` across N logs and
12 of the 23 `audit::record` callers writing a second log; or keep WAL and
accept the split, which the constraints block says loses. The process model
this needs: the module in the core's process on the core's one connection,
candidate 1 or 4's shape with two files under it. Two processes is the
sibling's sections 3 and 4: no shared transaction at all.

**6. Backup, restore, leak scan.** `VACUUM INTO` (`backup.rs:416`) copies
`main`; the attached file needs `VACUUM retail INTO` (syntax from memory). Two
copies at two instants, consistent only because `with_conn` (`lib.rs:382`)
holds the mutex across both. `backup::verify` (`backup.rs:484`) counts
`products` and `documents` at `:527-528`, which the core copy lacks. Restore
(`lib.rs:224-301`) renames one staged file over one shop file; with two it is
two renames, and a crash between them leaves a new core beside an old retail
with no constraint left to notice. The sibling's section 7 covers
checkpoint-busy and the rename, per file. The prune (`backup.rs:105-113`) and
`shop_file_bytes` (`support_bundle.rs:130`) each assume one file. The leak scan
(`crates/api/tests/support_bundle.rs:140`) is indifferent to file count.

**7. Windows 7.** Nothing changes: ATTACH is in every SQLite the bundled
`libsqlite3-sys 0.37.0` (`Cargo.lock`) ships; the sibling's section 5 applies.

**8. Tests.** In process, all five walks survive as text walks;
`repos_own_the_queries.rs` keeps its lists because the ATTACH lives in `db.rs`,
not a service. As a second crate, `services_go_through_services.rs:480` and
`repos_own_the_queries.rs:46` walk `crates/core/src` only and lose reach
(sibling section 6). The permission match survives. `migration.rs`'s 55
functions each build two files instead of one.

**9. Pros, cons, the killer.** Pros: a module's data is a file a shop can see,
copy or delete; a module absent from a build leaves a file nobody opens; zero
core-to-trade edges means the core cannot break on a missing module. Cons: 32
constraints replaced by Rust reads or `TEMP` triggers; backup, verify and
restore go from one file to N with no atomic swap; every fixture opens two
files. The killer: under WAL a sale is no longer one commit.

## Case (c): a schema per module, leaving SQLite

**1. Shape.** Diesel's backends are `postgres`, `mysql` and `sqlite`, so a
schema per module means PostgreSQL as a Windows service on the till, one
database per shop, schemas `core` and `retail`, the `table!` prefix as in (b).
Everything (b) loses, (c) keeps, for a server on a machine the constraints
block calls a desktop with no server.

**2. Cost.** `SqliteConnection` is named 664 times in 99 files under `crates`
(`grep -c`, `grep -l`) despite the `Conn` alias at `db.rs:11`; 23 `PRAGMA`
lines, 38 `sqlite_master` mentions and 39 `STRICT` suffixes go; `VACUUM INTO`
becomes `pg_dump`; `libsqlite3-sys` with `bundled` (`crates/core/Cargo.toml:18`)
becomes a driver plus an installer that ships a service, sets a password and
opens a port. The one file per shop rule (`docs/architecture.md`, Data) and
everything under it is rewritten.

**3. Foreign keys.** Kept, and stronger: `retail.documents` references
`core.users` and the server enforces it.

**4. Dashboard and audit.** Kept; cross-schema joins if ever needed;
`audit.rs` unchanged.

**5. Money atomicity.** Kept: one transaction across schemas is ordinary
PostgreSQL, held by the server, so a module in its own process could share it;
the sibling's section 3 lock problem becomes row locks.

**6. Backup, restore, leak scan.** All rewritten: a dump the app schedules and
verifies, a reload against a running service, `schema.txt` off
`information_schema`. The 629 lines of `backup.rs` and 375 of
`support_bundle.rs` do not survive as they are.

**7. Windows 7.** The EDB installer page (`postgresql.org/download/windows`,
fetched 2026-09-21) tests 13 through 18 on Windows Server 2016 and later and
lists no Windows 7 row; the last major with a Windows 7 installer was 10, out
of support since November 2022 (from memory, to verify). An unsupported
database, on top of the WebView2 109 question.

**8. Tests.** The five walks and the permission match survive as text. The 22
test files that open a temp SQLite file need a running PostgreSQL, which `just
gates` and `.github/workflows/ci-restricted.yml` do not have; `migration.rs`'s
55 functions are rewritten with the migrations.

**9. Pros, cons, the killer.** Pro: the only case that keeps foreign keys,
joins and atomicity while separating the data, and the only one where a module
process gets a real transaction. Cons: a database service on a shop's till, an
installer that manages it, backup, restore and the bundle rewritten, 99 files
touched. The killer: the constraints block's first line, no server; a till
that cannot ring a sale because a service did not start after a power cut.

## Ranking

Three constraints decide the order: money atomic under a power cut, less
complex, no server. (a) satisfies all three by changing nothing about the file,
and separates nothing, so it ranks first as a naming rule and not as a
candidate. (b) is second: real separation at a countable price, but its default
reading breaks the first constraint (WAL plus two files), its two repairs either
change every write's durability or duplicate two core tables per module, and it
works only in one process on one connection, so it is candidate 1 or 4 with two
files under it. (c) is last: it keeps every guarantee by installing a server on
a till and touching 99 files, fails the second and third constraints outright,
and has no Windows 7 answer. If the session wants data separation, the honest
version is (b) with `counters` and `audit_log` owned per module, priced against
candidate 1 with a module crate, which gets the same boundary from a Rust walk.

## Noticed, belongs elsewhere

- `services_go_through_services.rs:182` is a per-service data boundary today;
  "a module cannot write core tables" comes from there, not from a file layout.
- The WAL-and-ATTACH sentence also rules out multi-file atomic commits for
  candidate 2's shape, on top of its section 4.
