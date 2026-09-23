---
title: 'A kernel crate, and retail as the first module'
slug: 'a-kernel-crate-and-retail-as-the-first-module'
status: 'done'
category: 'feature'
created: 20260922
tldr: 'The split the seven-way comparison recommended, written as tasks. crates/core becomes a kernel crate and a retail crate; the shop half of the twelve files the boundary test pins moves out with it; the api and desktop stay one crate each with the retail rows behind a feature. Nothing a shop can see changes, and the same binary is built with retail switched on. The plug points are not in this plan: they get drawn from two real modules, not one, so they wait for the doctor package. Anouar settled the last open question on 2026-09-22, build time and a package we prepare per customer, so nothing here loads code at runtime.'
priority: 85
tasks:
  - id: 'S1'
    desc: 'This page: the split as tasks, the order, and what stays behind'
    status: 'done'
  - id: 'S2'
    desc: 'The two crates exist and the workspace builds: kernel and retail, retail depends on kernel, nothing moved yet'
    status: 'done'
  - id: 'S3'
    desc: 'The 63 source files move, the 35 import edges rename, the five helpers widen'
    status: 'done'
  - id: 'S4'
    desc: 'The twelve pinned files give up their shop half: the error enum, the column enums, the print strings, the two raw counts'
    status: 'done'
  - id: 'S5'
    desc: 'The api crate: 42 gate rows and 59 routes behind the retail feature, the nightly recount with them'
    status: 'done'
  - id: 'S6'
    desc: 'The tests move with their services, and the boundary walk is rewritten against two crates'
    status: 'done'
  - id: 'S7'
    desc: 'A build with retail off compiles, runs its migrations and signs a user in'
    status: 'done'
---
# A kernel crate, and retail as the first module

Samir, 2026-09-22: go with the split, make the core smaller, and build
towards a known point once the clinic research names one.

Anouar settled the shape the same morning. At 08:52: "module bringing its
own screens, permissions and tables, without ever touching the core's, this
is what we want." At 09:03, asked whether that happens at runtime: "no not
in runtime." At 09:49: "we need to add the modules for them, so a doctor
wants a software, we prepare the package for him and send it." So a
customer's package is built by us with their modules compiled in. Nothing
in this plan loads anything while the program runs.

## Why this order and not another

The comparison of 2026-09-21
(`context/research/20260921-module-shape-the-seven-compared.md`) kept three
shapes and put them in an order: change nothing, then the crate move, then
the plug points, and only ever in that order. Two things have happened
since, and both say the first two steps can start now.

The boundary is a rule since `5015e4e`: a walk over the twenty-one shared
files fails the build when any of them names a shop word outside a pinned
list of twelve files. That list is the work item of S4, and it is also the
proof that S3 finished honestly, because a file that moves its shop half out
has to leave the list or shorten its row.

The paper test measured the tear, which is what the comparison said the
split was waiting for. Nineteen tears
(`context/research/20260922-the-paper-test-a-consultation-in-the-document-model.md`),
every one of them in `documents`, `document_lines`, `document_tva`,
`stock_movements`, `customers`, `debt_ledger` or the stamp and TVA rules.
None in users, sessions, permissions, audit, settings or preferences. The
line the crates draw is the line the tear list found, so the move is not a
guess about where a second trade breaks.

The plug points stay out. Candidate 4's own page says every trait method
has one implementer on the day it is written, which makes the hooks a guess
until the doctor package exists to draw them from. S2 to S7 create no trait
and no registry.

## What the shape is when this plan closes

`crates/kernel`: users, sessions, permissions, audit, settings,
preferences, pairing, backup, support_bundle, clock, shops, the money
module, the error enum, `Role`, and the three print files the whole engine
shares. Eleven services, seven repos, six models.

`crates/retail`: sales, purchases, avoir, proforma, documents, cancellation,
cash, cash_refunds, shifts, stock, pricing, products, categories, customers,
debt, suppliers, supplier_debt, expenses, dashboard, export, import, seed,
and the eight print files that draw paper. Twenty-four services, seventeen
repos, fourteen models, and `counters`, `jobs` and `models/job.rs`, which no
kernel service reaches.

`crates/api` stays one crate. Its 42 retail gate rows and 59 retail routes
sit behind `#[cfg(feature = "retail")]` in the same two files, because
`route_gates.rs` reads the router with `include_str!` and would never see a
second one, and a table assembled at startup is the runtime registry Samir
ruled out on the 21st. The desktop stays one app; its 30 retail screens do
not move.

`default = ["retail"]`, so a shop's build is the product as it is today and
nothing a shop can see changes.

## The tasks

**S2: the crates exist, empty.** Two new members in the workspace
`Cargo.toml`, retail depending on kernel, kernel depending on neither.
Nothing moves in this task; it exists so the move in S3 is a move and not a
move plus a build fight. Done when `just clippy` and `just test` are green
with two crates that hold nothing. Size S.

**S3: the move.** 63 source files from `crates/core/src` into
`crates/retail/src`, counted by folder on 2026-09-21 (services 24 of 35,
repos 17 of 24, models 14 of 20, print 8 of 16). 35 import edges from a
retail service into a kernel one rename from `crate::services::x` to
`dzpos_kernel::services::x`; audit is 15 of them, clock 12, shops 3,
permissions 2, settings 2, users 1. Edges the other way: zero, which is what
makes this a move rather than a redesign. Three of the five `pub(crate)`
helpers in `services/mod.rs` widen to `pub` (`role_of`, `optional_field`,
`bounded_field`); `user_names` and `end_sessions_of` have kernel callers
only and stay as they are. `lib.rs`'s `pub(crate) mod repos` needs no change
and is what stops retail reaching a kernel repo. Done when the workspace
builds and no behaviour changed. Size L.

**S4: the twelve pinned files give up their shop half.** This is the "make
the core smaller" half of Samir's sentence, and the boundary walk is its
score. Per file, in the order they are cheapest: `models/sql_types.rs`
splits, `Role` staying in the kernel and the other nine column enums going
to retail. `error.rs` keeps eighteen variants and retail takes its own error
type for the six shop ones, which means `crates/api/src/error.rs` maps two
enums instead of one; the alternative, a kernel that keeps naming them, is
the thing this task exists to stop. `print/strings.rs` splits its word list.
The two raw counts in `backup.rs` and `support_bundle.rs` ask retail for a
count instead of naming `products` and `documents` themselves. `audit.rs`'s
action tags move to the module that records them.
`permissions.rs` is the one to decide rather than do: the enum stays one
list by Samir's ruling of the 21st, so its ten shop variants stay in the
kernel and the walk's row stays, with the reason rewritten to say it is a
decision and not a leftover. The money kernel's `discount` and `price` are
the second decision: shop-only or shared. Do not settle either in passing;
they are the two rows the walk carries so that a person has to answer them.
Done when the walk's allow list is shorter and every remaining row says
decision rather than leftover. Size M.

**S5: the api crate.** 42 gate rows and 59 routes behind the feature, and
`daily.rs`'s nightly stock recount with them. The gates table stays one
`const &[Gate]` and `gate_for` is unchanged. The 122 generated DTOs do not
move: `just types` runs one export test into one folder, and a shop's
TypeScript client carries the types of trades it was not built with, the
way a shop build carries permission variants it never uses. Size M.

**S6: the tests.** 48 of the 59 core test files name a retail service, a
print module or a retail model and move with them; 11 stay. The
`crates/api/tests` 31 files stay and the retail ones run under the feature.
Then the boundary walk itself is rewritten: with two crates, the rule it
enforces is partly cargo's, because a kernel that cannot see retail cannot
import it. What the walk still has to say is the part cargo does not, the
naming: a kernel file may not spell a shop word even in a constant or an
error message. Size M.

**S7: the proof.** A build with `--no-default-features` compiles, opens a
database, runs the migrations and signs a user in. That is the whole claim
of the split: the kernel is a program without a shop. Under Samir's D5
answer the migration folder stays one and lives in the kernel, so a kernel
build creates every trade's tables and uses a tenth of them; that is
accepted and written down here rather than discovered later. Size S.

## What this plan does not do

- No trait, no `Module` type, no registry, no list of modules anywhere. The
  plug points wait for the doctor package, which is when there are two
  implementers to draw them from.
- No schema change and no migration. The tables keep their names and their
  columns; only which crate declares the `table!` block moves.
- Nothing a shop can see. If a screen, a total, a printed paper or a
  permission behaves differently at the end of S7, something in the move was
  a rewrite and has to come back out.
- Not the doctor module. What that module would contain was researched on
  2026-09-22 (`context/research/20260922-what-clinic-software-provides.md`),
  which names the target this split is built towards, not its content. Three
  questions it leaves for this plan to answer: whether a patient is a row in
  the module's own table or a differently typed `customers` row (its finding
  is that Anouar's "same module with different strings" holds for the
  identity columns and fails at the money ones); how the shared role gate
  learns a module's own permission names when each package is assembled
  separately; and how a module's migrations sequence against the kernel's so
  a package still passes `crates/api/tests/upgrade_from_a_previous_version.rs`.
  The last one lands on S7 and the second on S5.

## The gate before S2 starts

The shared Claude plan was at 95 percent on 2026-09-22 and Anouar needs
headroom for his own work, so this plan is written and waits. S3 is a
builder-heavy day and S4 is the careful one. Samir says when the budget has
room.

## Samir, 2026-09-23 09:20: no full ports and adapters

Asked whether the restructure should move to ports and adapters with
dependency injection, plus DDD-lite. Ruled: ports only at the plug points,
where a module tells the shared part what it brings (permissions, routes,
migrations, audit actions, what backup counts), drawn after the doctor
module exists so each trait comes from two real modules. The crates are
the DDD-lite bounded contexts; regrouping each crate's files by domain
(the DTO file first) is a follow-up after the split merges. No trait in
front of every repo and no DI through every service: the app is offline
SQLite with no second database to swap in, and the tests already run
against a real file.
