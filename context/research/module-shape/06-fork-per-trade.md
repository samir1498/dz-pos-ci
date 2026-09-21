# Candidate 6: fork per trade

D7 brainstorm page, judged against the constraints block in
`context/plans/20260921-whether-dinar-becomes-a-core-and-modules.md`. Written
2026-09-21 from `main` at 9001c3e; every number says how it was counted. This
is the baseline: not a module system, a second copy of the repo with the shop
cut out of it, and the shared parts lifted out only once two real trades exist.
The four sibling pages (candidates 1 to 4) are taken as read and cited where
this page prices itself against them.

### 1. The shape

Two repositories, the second made by `git clone` of the first and not by a
fresh tree, so the two share ancestry. That choice is the whole mechanism of
the running cost in section 3: a fix committed in one repo applies to the
other by `git cherry-pick <sha>` for every path that kept its name, and
`git log --cherry-pick --right-only retail/main...clinic/main -- <shared
paths>` lists the fixes one side has and the other lacks. Without shared
ancestry each port is a hand diff.

The one-repo variant, two product trees under one root sharing a justfile,
was priced and dropped. It is candidate 1 with the kernel crate left out:
two cargo workspaces on a box whose rule is one shared build folder and one
cargo run at a time (`justfile:37`, the `claim` recipe), so `just gates`
runs twice as long while every walk and gate table still exists twice. It
buys one PR carrying both edits and one CI run, nothing else a fork wants.

The more important thing about the shape is when it happens. No second trade
has a name (plan page, line 99), so the fork's day-one form today is that
nothing happens. The copy is a decision that can be taken the morning a
second trade arrives, and it costs zero until that morning. That is why it is
the baseline: every other candidate pays now for a module nobody has
specified, and this one defers the whole cost until the specification exists.

### 2. The day-one cost, counted

Take a clinic as the worked example, since it is the one Anouar named and
the one D3 is written around. What the clinic fork deletes on its first day,
counted by listing each folder at 9001c3e and classifying by file name with
candidate 1's kept lists (users, sessions, permissions, audit, settings,
preferences, pairing, backup, support bundle, clock, shops):

Core services: 24 of 36 files, 12 055 of 16 703 lines (`wc -l` per file,
kept files summed and subtracted). Repos: 17 of 25 files, 4 370 of 5 583
lines; `counters` and `jobs` go with retail as candidate 1 found. Models:
14 of 21 files, 2 496 of 3 279 lines. Print templates: 8 of 16 files, 2 301
of 5 110 lines. The seeder: `crates/seed` (415 lines) and `services/seed.rs`
(1 127 lines), both shop-shaped by their own header. Core tests: 42 of 59
`.rs` files under `crates/core/tests` plus the 5 proptest regression files.
Migrations: 10 of the 19 folders touch no kernel table and 4 touch both
(`init` creates `categories` and `products` beside `shops`, `users`,
`settings` and `counters`; `documents` creates `audit_log`; `000008` creates
`jobs`; `000009` alters `counters`), counted by the table names each
`up.sql` creates, alters, indexes or inserts into. Since no shop file exists
on any machine (plan page, "no users yet"), the fork can squash the 19 into
one fresh `init` of the 10 kernel tables, and then
`crates/core/tests/migration.rs` (4 122 lines, 55 `#[test]`) is rewritten
rather than trimmed, since candidate 3 counted 268 of its table literals as
retail.

API: 12 of 20 route files, 11 of 20 DTO files, 16 of 31 test files, 59 of
the 90 `.route(` lines in `router.rs` and 42 of the 67 `Gate {` rows in
`gates/table.rs`, both classified by the first path segment (kernel
segments: health, pairing, auth, audit-log, backups, build-info, clock,
settings, support-bundle, users). Generated DTOs: 82 of 122 by candidate 1's
count. Desktop: 32 of 45 route files, about 16 500 of the 19 566 lines under
`apps/desktop/src/routes` by the per-file listing (the five `-till`,
`-customers`, `-documents`, `-products`, `-suppliers` folders and the
top-level shop screens); 6 of 44 components; 9 of the 12 nav items at
`apps/desktop/src/components/AppShell.tsx:112` to `:169`; the redirect to
`/till` in `routes/index.tsx`; at least 376 of the 818 keys in
`i18n/en.json` (a prefix regex over till, sale, product, customer and the
like, so a lower bound). Mobile: the phone is a till, so
`apps/mobile/features/till` and `app/(signed-in)/till.tsx` go, about 600 of
4 002 lines; pairing, sign-in and session stay. `packages/shared`: 12 of 19
client files and about 20 of 31 schema files, by name. Fixtures: all 112
files under `fixtures/print`; the 8 under `fixtures/money` stay. In Rust
overall, 16 849 of 41 718 non-blank lines under `crates/*/src` and
`src-tauri/src` stay (`grep -c .`), the plan page's quarter measured again.

What the kept part has to change to compile without the deleted part, by
grepping every kept service, repo and model for `services::`, `repos::` and
`models::` paths into a deleted file, comments and `#[cfg(test)]` stripped:
zero hits. No kept service reaches a retail one; the ring test's zero at
`crates/core/tests/services_go_through_services.rs:260` is why. The
entanglement is in six places. `crates/core/src/error.rs`: 6 of 22 variants
are shop words (DuplicateBarcode, PaymentAboveDebt, CreditLimit, PartyIds,
Unstamped, UnpricedReversal), and `crates/api/src/error.rs` maps them.
`models/sql_types.rs`: 9 of 10 `text_enum!` invocations.
`services/permissions.rs`: 10 of 15 variants by candidate 1's list; the
clinic writes its own and the match at `:206` stays exhaustive. `schema.rs`:
21 of 31 `table!` blocks. The print engine: `print/mod.rs` (3 code lines
naming `Document`, 2 `use` of `models::document`), `print/escpos.rs` (7
lines, 1 `use`), `print/strings.rs` (6 lines); the other five engine files
name no model, matching candidate 1 section 5. Two raw counts:
`services/backup.rs:527` and `:528` count `products` and `documents` to
decide a file is a Dinar file, and `services/support_bundle.rs:107` onward
counts products, documents and customers for `counts.txt`. Then the
assembly files: `router.rs`, `gates/table.rs`, `gates/tests.rs`,
`dto/mod.rs`, `routes/mod.rs`, `daily.rs:25` (the nightly stock recount),
the two `settings.rs` (3 of 8 settings routes are retail, candidate 4), and
`crates/core/tests/common/` (2 093 lines, whose helpers build customers for
kept tests too), and the 22 pins in `scripts/file-sizes.json` (16 name retail
files; `strings.rs` and `migration.rs` shrink). About 20 files, no service.

Identifiers, or the two products collide on one PC: `identifier`
`com.dinar.app` (`apps/desktop/src-tauri/tauri.conf.json:5`); the updater
endpoint at `:34`, `Dinar-dz/dz-pos/releases/latest/download/latest.json`,
the dangerous one, since a clinic build left on it updates itself into a
shop till; `_dzpos._tcp.local.` (`crates/api/src/mdns.rs:10`); the shop file
`dirs::data_dir()/dzpos/dzpos.db` (`src-tauri/src/lib.rs:451`); the five
crate and package names. Around them a second mirror (`just ci` force-pushes
to `samir1498/dz-pos-ci`, `justfile:368`), a second `ORG_RELEASE_TOKEN`
target (`release.yml:358`, the release is created on `Dinar-dz/dz-pos` by
API call), a second signing key (pubkey `UNSET`, `tauri.conf.json:32`), and
copies of 690 lines of workflow, 551 of justfile, 374 of `.github/scripts`.

### 3. The running cost: the double-maintenance rate

The repo's first commit is 2026-09-07, so the last thirty days are its whole
history: 1 162 commits, 649 of them touching code (the other 513 touch only
`context/`, `docs/`, `research/`, `design/` or a Markdown file). Every file
in every commit was classified by path as shared (section 2's kept lists,
`money/`, the engine, `src-tauri`, the desktop shell), retail, mixed (the
enums, `schema.rs`, `router.rs`, `gates/table.rs`, the i18n JSON,
`routeTree.gen.ts`), or infra (`.github`, `justfile`, `scripts`, manifests).

The count the brief asked for: 212 commits touched a shared or infra file
and no retail file, against 431 that touched retail (138 retail only, 24
retail plus a mixed file, 269 retail plus a shared file). Shared-only to
retail-touching is 212 to 431, or 0.49. The broader figure is the one a
fork pays: 481 of 649 code commits, 74 percent, touched at least one shared
or infra file, and each of those is a cherry-pick into the other repo, with
a second `just gates`, a second mirror run and, per tag, a second release.
The 269 entangled commits are an upper bound, because `crates/api/src/lib.rs`
(62 touches), `packages/shared/src/client.ts` (50) and
`packages/shared/src/index.ts` (42) are aggregators every feature edits, and
the retail half of such a commit ports as nothing.

The rate is not falling yet. The eight days 14 to 21 September, the current
loop (shifts, cash refunds, the review round), hold 80 code commits: 26
shared-only, 49 retail plus shared, 5 retail only. At the PR level, 66
squash commits carry a `(#N)`: 19 shared-only, 44 both, 3 retail only. By
area, the 212 shared-only commits fall on infra 82, backup and bundle 35,
migrations and `db.rs` 23, settings and theme 22, api plumbing and gates 17,
users, sessions and permissions 16, pairing 16, money 16, the Tauri shell
14, the UI kit 11, audit 9, the print engine 3 (a commit may count twice).
The plan page calls this the cheap quarter (line 63), and it is cheap to
build once; the log says it is also where the plumbing churn lives. At 212
over 15 days, 14 a day, a fork made this week diverges by about a hundred
shared commits a week if nobody ports, and costs a hundred cherry-picks a
week if somebody does.

### 4. Lifting the shared parts out later, and candidates 1 and 4

A fork today does not close either door. With two real products in hand the
extraction is candidate 1's move done once per repo: the kernel files (the
11 services, 7 repos, 6 models, 5 engine files, `money/`, `db.rs`,
`error.rs`, the kernel migrations) move into a `crates/kernel` in each fork,
the two kernels are diffed, whatever drifted in the meantime is reconciled
once, and the result is one crate in a third repo that both forks name as a
git dependency; the desktop shell and `packages/design` follow as packages.
The cost is candidate 1's 63 moves twice plus the reconciliation, which is
exactly the unported remainder of section 3.

Candidate 4 is where the fork's real value sits. That page's own killer (its
section 8) is that every trait method has one implementer on the day it is
written, and it says a trait extracted from two real modules is cheap. Two
forks are two real implementers. The clinic's counterpart of
`crates/core/src/services/sales.rs:228`, written without a trait in the way,
is the second data point candidate 4 lacks: the diff of the two closures
says what `inside` and `after` take, what the numbering trait needs, and
whether `documents` belongs to the kernel or to each trade, which D3's tear
list otherwise has to guess. The fork is not opposed to candidates 1 and 4;
it is the path to candidate 4's own precondition, paid in cherry-picks.

### 5. Money, Windows 7, the walks and the match

All four are inherited unchanged, because each fork is today's binary with
files removed. The sale stays one `conn.transaction` closure at
`sales.rs:228` on one connection behind one mutex (`crates/api/src/lib.rs:68`)
in the retail fork, and the clinic writes its own closure in its own repo
with nothing to coordinate. Windows 7 is candidate 1 section 3 whole: Rust
1.78 and later need Windows 10 on every `pc-windows` target and WebView2
stopped at 109, so both forks are Windows 10 products built by the same
`release.yml:171` job; a fork changes no toolchain. The five source walks
candidate 1 section 5 names each walk the crate or folder they live in, and
a fork is the same crate with fewer files, so all five run untouched in both
repos with their allow lists cut down. The permission enum stays one global
list per product with its exhaustive match at `permissions.rs:206`; the
clinic deletes the ten shop variants and adds its own, and the compile error
that caught a mistake twice in September fires in both repos independently.

### 6. Anouar's safety ask under a fork

"Adding a module cannot break an existing one" is met trivially: a clinic
module is not in the shop's binary, schema or gate table, so there is
nothing for it to break. The question turns inside out: what does a fix to
shared code cost, and what happens when it lands on one side only. The
concrete failure is a session or lockout fix in
`services/sessions.rs` or `services/users.rs` shipped to shops and not to
clinics, or the reverse, with nobody's test failing anywhere, because each
repo's gates are green on their own. The ISO controls
`context/processes/20260908-security-and-provenance.md` asks every feature
to ship with are then evidenced twice or, on a bad week, once. Candidates 1
and 4 make a shared fix one commit; the fork makes it two and relies on the
`--cherry-pick` listing in section 1 to notice the gap.

### 7. Pros, cons, the one thing that kills it

Pros. Zero files move and zero traits are invented; the day's cost is
deletion, which the compiler checks. No abstraction is designed ahead of the
second trade, so none fits nothing. Each product is the plainest shape, which
is what Samir asked for. Money, the walks and the match are untouched. It
costs nothing until a second trade has a name, and it produces the second
real implementer candidate 4 needs.

Cons. Seventy-four percent of the commits so far would have had to be ported.
Two mirrors, two release targets, two signing keys, two justfiles, two sets
of workflows. Six places still carry the shop into the kept part and get
hand-edited on day one. A drift between the two kernels is invisible to
every test and shows up in an audit or a bug report. The extraction in
section 4 is a three-way merge whose size is set by how long the fork lived.

What kills it. The shared quarter is still under construction. Fourteen
shared-only commits a day, on backup, migrations, sessions, settings and
the Tauri shell, is not a stable kernel to copy; it is a moving one. A fork
taken while that number is high buys two copies of a thing that is still
being written, and the second copy pays for every rewrite of the first. The
fork is the right baseline precisely because its day-one cost is zero when
it is not taken, and the wrong call precisely when it is taken early.

### 8. What would have to be true for the fork to be the right call

Any one of these. First, no second trade within a year: then the fork is
free by construction, because it never happens, and the right work now is
D4's rule and nothing else. Second, the shared-only rate measured the way
section 3 measured it, commits per week touching a kernel path and no
retail path, has fallen near zero for several weeks running, so the copy is
of something finished. Third, the second trade's day is nothing like
retail's: it reuses none of `documents`, `counters`, the TVA and stamp
rules or the fiscal numbering, so the kernel worth sharing is only the
cheap quarter and candidate 1 and 4's `documents` fork never has to be paid.
Fourth, a second owner: a separate person or team for the second product,
for whom a separate repo is the honest boundary. If none of the four holds
when the second trade arrives, candidate 1's move, with candidate 4's traits
drawn from two real cases afterwards, beats the fork on this page's numbers.
One finding belongs to D4: the kept services are clean, and the shop lives
on in the kept part only through the list in section 2, which D4 should name.
