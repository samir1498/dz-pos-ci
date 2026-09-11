---
type: 'now'
updated: '2026-09-11'
---
## Active

**Resumed 2026-09-11** after the 2026-09-10 23:01 pause ("see ya next week
time to rest"); the loop restarts today as 20-minute ticks working M4 and
the landing page. This morning: M4's plan gained its ten tasks
(`context/plans/20260908-m4-team.md`, users and PINs through the closing
sweep) and a new `landing-page` plan was written, replacing D5 of the
design plan; the organisation's GitHub Actions budget is still capped, but
`just ci` now pushes a branch to the mirror `samir1498/dz-pos-ci` (merged to
main as PR #22) so gates run on Samir's own minutes without waiting on
Anouar. Still open: Anouar's GitHub billing fix itself, which the mirror works
around but does not close.

The demo host was open and is now answered, on 2026-09-11, with the question
that had been marked unverified: Cloudflare is not ruled out for the app
itself, and it is the best fit. Cloudflare Containers reached general
availability in April 2026, and their disk is ephemeral: a container sleeps
after ten minutes of inactivity by default and wakes with a fresh disk from
its image. For a shop's real data that would be disqualifying. For a demo it
is the behaviour to want, because the seeded SQLite file ships inside the
image, every visitor gets the same shop with its thirty days of trading, and
nothing a visitor does survives them. `just seed` is deterministic and
repeatable, which is what makes that work. The cost is a cold start for the
first visitor after a quiet spell, which a Rust binary opening a SQLite file
pays quickly. The other argument for it is that the landing page is already
on Cloudflare Pages, so it is one account and one domain rather than two.
Not deployed: putting the app on a public address is Samir's call, not the
session's.

The landing page is finished, all six tasks on main: the site (PR #25), the
copy with every fiscal claim carrying the `docs/features.md` row it rests on
(#26), the product shots composed from the committed e2e screenshots with
three new French saves the suite had never taken (#27), the sections (#28),
the load budget (#29: fonts split per route, Lighthouse 100/100/100/100 on
French and English and 99 on Arabic, a test refusing an unsized image or a
route over 678 KiB), and the publish preparation (#30: canonical and
alternate links, the sitemap, the sharing card drawn from `@dzpos/design`,
Cloudflare Web Analytics written in and switched off, and `just
landing-deploy` refusing to run without `DZPOS_LANDING_PUBLISH=1`). The page
is built and not published. Three things block it and all three are Samir's
and Anouar's: the product's name is a placeholder, there is no price, and no
native speaker has read the Arabic. The three built routes are shot and on
the boss site at https://dinar-reports.pages.dev/landing/ so Anouar can
answer without a checkout. Left for later: the hero ships a 1344-pixel file
for a 342-pixel render on a phone, which needs width descriptors in the shot
pipeline.

**M4 is on main since 2026-09-11 18:49 (PR #33, cf5236a).** Ten tasks, each
reviewed and merged on its own, then a whole-milestone review and a second
round over the fixes that review produced. The branch is gone; the mirror
check was green on its head. What a person gets:

Migration 000011 adds the five sign-in columns to the `users` table that has
existed since the first migration, with argon2id hashing, the users service
writing an audit row per operation, and a wait that doubles from thirty
seconds to a quarter of an hour after five wrong tries. Thirteen permissions
sit in one `can(role, permission)` table with a typed refusal carrying the
permission, and the discount threshold rides the regime fiscal's dated
history; a manager answers like an owner except on the staff list and the
audit log. Migration 000012 adds sessions, which replace the seeded owner id
on every route: 32 random bytes stored as a digest, the desktop's own header
or an httpOnly cookie, the launch token untouched and still first. The
session middleware looks the route up in one table naming the permission it
wants and refuses before the handler runs, so no handler names a permission
twice, and a write on a route the table does not name is refused rather than
waved through.

A person signs in at a PIN pad if they are at the till and with a name and a
password everywhere else, their name sits in the topbar with a way out, and
after the idle time a lock covers the app without unmounting it, so a
half-rung basket is still there when they come back. The cover is a real
cover: a click, a tab, a barcode scan and the app's own attempts to put the
cursor back in the search box all fail to reach through it, the keyboard
shortcut that pays refuses while it is up, and the app underneath stops
asking the server for anything, so the session finally times out on the
server as well as on the glass.

A cashier cannot see what the shop paid. The purchase, dashboard, expense,
cash and supplier routes refuse them outright; the product list, which they
need to ring a sale up,
hands back its cost and wholesale fields empty instead of being closed. That
second half is what the closing review found missing: the two permissions
about seeing cost and seeing reports were written down, tested, and enforced
by nothing, so a cashier with a session could read every product's cost off
the API while the screen politely hid the column.

A cashier refused a credit block or a discount above the threshold is
refused by name, and so is a cashier who tries the other way round it,
typing a price lower than the one on the product's card; the log carries the
card price beside what was actually charged. A refused credit sale leaves a
row now whichever way it was refused, including when the till sent the
override flag, which is the deliberate attempt and the one the log most
needs. The threshold itself has a route and the settings page reads it back.

The owner has an audit log screen over the rows every service has been
writing since M1, read a page at a time from the file rather than whole.
Settings has a staff list: add someone, rename, change a role, reset a PIN,
switch someone off and on again, each refused to a cashier by the server
rather than by a hidden button, with the last owner protected. Resetting a
PIN or switching somebody off now ends the sessions they were holding, and
somebody changing their own PIN keeps the screen they are standing at. A
refused request, an export and a restore each leave a row.

A brand-new shop can be claimed: one route takes a first PIN with no
session, refuses the moment any credential exists anywhere in the shop, and
hands back a live session.

The browser suite now drives a real cashier, which it never did: every one
of its earlier specs signed in as the owner, so deleting every permission
check in the API would have left all of them passing. Seven specs put a
cashier through the refusals this milestone exists to produce, and they tell
the refusals apart rather than accepting that something was refused. Proven
the only way that counts: deleting the gate on typing a price at the till
turns exactly one of the seven red and leaves the other six green.

Writing that suite found the last thing the milestone was getting wrong.
Ten screens each kept their own copy of the table that turns a server error
code into a sentence, and six of them had no entry for a refused permission,
so a cashier refused anywhere but the staff screen was shown "something went
wrong" instead of being told what they may not do. There is one table now,
and a screen that genuinely means something different for one code passes
that one line and inherits the rest. The word for a refusal was also spelled
twice in all three dictionaries with the wrong one winning, and the role
names twice as well; neither was visible to the translation test, because it
read the parsed object and JSON keeps the last of a repeated key in silence.
It reads the files as text now.

**M5, the first release, started on 2026-09-11 at 19:30**
(`context/plans/20260908-m5-first-release-v1.md`, ten tasks). It is the last
milestone before a shop can run on this, and it is different from the four
before it: most of what it builds cannot be demonstrated from this machine.
An installer needs Windows, a signature needs a certificate nobody has
bought, and an update needs a previous release to update from. So every task
says what can be proven here and what cannot, and a task that says it is
done and means it compiles is not done.

Four of the ten are on main as of 2026-09-11 22:40. The copy of the shop
file taken before a new build migrates it. The version, git short hash and
build date embedded at build time, shown in About and heading the log file
beside the shop file. The webview's content security policy with the launch
token out of the page global, which was the only item in the milestone that
was a hole rather than machinery. And the release rule, where a tag on
`main` is the only thing that publishes and a manual run can only build the
installer and keep it as a download.

Two things the reviews caught before those last two merged, both of the same
shape, a guard that could be talked past. The release workflow pasted the
tag's own name into a shell command, and a git tag may carry a dollar sign,
a backtick, a quote and a semicolon, so a tag named the right way would have
run its own commands inside the job that decides whether a release is
allowed. And the pre-upgrade copy left a half-written file behind for good
when it was interrupted, because the retry after it used a different name.

In flight: the support bundle, written and open for review, waiting on the
browser suite because it adds a button to the settings screen and the
committed pictures of that screen are stale until a local run regenerates
them.

Also on main since 2026-09-11 23:44: `docs/release-checklist.md`, one page
saying what has to be true before v1.0 and who holds each item, and a
section on the public page saying what a shop with staff gets.

Blocked since 2026-09-11 22:30 and not by anything in the repository. The
build machine's Windows disk is at one and a half gigabytes free. Its WSL
disk image is 141 gigabytes holding 44 gigabytes of files, and an image
never shrinks on its own, so about a hundred gigabytes are recoverable only
by compacting it from the Windows side (`wsl --shutdown`, then
`wsl --manage <distro> --set-sparse true`). Nothing inside can do it. The
image itself is not growing, which was checked by writing half a gigabyte
inside the distro and watching the byte count stay put. Anything that does
not need the browser suite is proven on the mirror instead, which is how
three pieces landed after the disk ran out.

Two tasks the reviews added on 2026-09-11. A Windows release build has no
console, so a refusal to start says nothing at all to the shopkeeper: the
window never opens. That was already true of a failed migration and is now
easier to reach, because the app refuses to start when it cannot copy the
shop file before an upgrade. And the two kinds of copy that sit beside the
shop file, the one before a restore and the one before an upgrade, are a
file swap by hand: the restore route takes the name of a daily copy and no
other kind.

Not in the milestone, though `docs/roadmap.md` said so until 2026-09-11: the
bon de livraison. `docs/features.md` § Later parks it for a fiscal reason,
not an effort one, and the two pages now point at each other.

Five release gates are Samir's and Anouar's rather than mine, and two have
lead times that start when they say so: the product's final name, which is
baked into the installer, and the Windows code-signing certificate, without
which every customer sees a warning.

Open for Samir on M4: the wrong-try counter is one per person and covers the
PIN and the password together, so a fumbled password locks that person's
till PIN; the regime fiscal sits inside the settings permission a manager
holds; and a quotation is not gated, so a cashier can print a proforma
promising a price and a discount they could not charge at the till. Watch
for later: a refused request is now a write to the file and nothing prunes
the audit table, which is fine over loopback and a real question the day a
phone on the shop LAN can reach the API.

Earlier on 2026-09-11: the mirror workflow reached main (PR #22) and its two
heavy jobs, Windows and coverage, were cut back to a push to main (PR #24)
after the first two branch checks paid for both, because `just ci` starts a
run by hand and a hand-started run is not a pull request. The stale sweeps
through the code, the context store and the research notes are on main
(PR #23 and two context commits).

Roadmap `dz-pos-to-first-shop` (`just ctx roadmap show dz-pos-to-first-shop`
from the repo root, full text `docs/roadmap.md`). M0 closed on 2026-09-09
(PR #14). M1 merged to main on 2026-09-09 (PR #15) except T6, the real
printer, which waits for Samir at the laptop. M2 (customers, credit,
facture at the till, credit notes, cancellation, proforma, payments,
statement, debt slip) merged to main on 2026-09-10 at 03:36 (PR #19,
79732ca): nine tasks run as a loop on `m2/2026-09-09`, each with three
review lenses and a fix round, then a combined review of the whole diff.
M3 (stock in, expenses, reports) runs as a loop on `m3/2026-09-10` since
2026-09-10 06:33: ten tasks in its plan, the yearly reset of every
document series first (Samir, 2026-09-10: the common practice, the year
in the number), then suppliers, purchases with partial receipt, expenses,
the stock re-derive, the dashboard, Excel. The two M2 refactors (T10 the
avoir's Remaining table plus the kind rules as CHECKs, T11 the zod client
and the till split, both approved by Samir on 2026-09-10) run first on the
same branch. `context/loops/20260910-m3-loop.md` holds the waves and
ports. By 16:30 on 2026-09-10 every M3 task but the dashboard screen and
the closing sweep is on the milestone branch (yearly series, tables,
suppliers, expenses and cash, stock recount, purchases, Excel and labels,
the seeder with the thirty-day series); the afternoon also fixed the
shared build folder (claim step and lock in the justfile, four jobs, one
cargo run at a time). Samir saw the dev server (tmux `dz-dev`,
http://100.101.196.30:5173) and found the screens unstyled:
`context/plans/20260910-design-system-and-branding.md` (four themes on
shadcn/ui with a data-theme switcher, a bilingual logo the language switch
swaps, vendored Plex and JetBrains Mono, Lucide, the kit and the screen
rewrite, a Claude Design project already seeded; Astro landing later). D1
to D3 are merged: by 19:15 every screen sits on the kit (the bare-element
allowlist is down to the theme switch and a fields helper), the dashboard
with its thirty-day chart is on the milestone and in the sidebar, and the
boss site has a gallery of every e2e screenshot (screens/). The closing sweep is
merged (docs, comments, the whole-milestone review and its two import
fixes) and the milestone is on main since 20:18 (PR #20, merged on the
local gates at Samir's word because GitHub Actions cannot start a job:
the organisation's billing failed at 16:12, every run since fails in five
seconds; Anouar fixes it in Billing and plans). M4 (roles, PIN,
permissions, audit log) gained its ten tasks on 2026-09-11; D4 (the Claude
Design bundle lags the repo) is still open, and D5 (the Astro landing) is
superseded by its own `landing-page` plan.
The brand pages are on the boss site under /design/. 2026-09-10 midday: the
WSL disk reached 125 GB from per-worktree cargo targets and the box crashed
five times; the disk rules in CLAUDE.md and the machines process page came
out of it (one shared build folder, one build at a time, worktree teardown
on merge, one session per conversation). A stale-info sweep ran the same
morning. The ledger on the WSL box (`~/.dz-night/ledger.md`)
holds the ticks.

Status site for Anouar: https://dinar-reports.pages.dev/ (progress reports
and reference pages; generator in `~/.dz-night/report/` on the WSL box).

Run `just status` from the repo root for the ladder and the plans.
Run `just ctx show <slug>` for plan details.

## Done recently
- 2026-09-11: stale-info sweep after the M4 checkpoint (roles, PINs,
  permissions, sessions, audit log; PR #33). Two context items fixed on
  `main` (70fa841, folded into PR #35's squash when the worktree fetched
  local main before origin caught up): the CI mirror branch note in
  `now.md` and the repos page still said `ci/lean-and-mirror` was not
  merged (it reached main as PR #22), and `now.md` named only the purchase
  and dashboard routes as closed to a cashier outright where five are.
  Nine more items fixed on `chore/stale-sweep-20260911-m4` (PR #35,
  4b69007): `docs/roadmap.md`'s M4 heading had no closed tag; `README.md`'s
  Screens list was missing `audit` and undercounted `/kit`;
  `docs/architecture.md`'s error-code table was missing five codes
  (`session_required`, `auth_refused`, `locked_out`, `forbidden`,
  `ungated_write`) and said six optional fields where M4 made it eight;
  `crates/api/src/routes/suppliers.rs::adjust` still explained a
  seeded-owner stand-in the route stopped needing at M4 T1;
  `crates/api/src/dto.rs`'s `ThemeChoiceDto` doc and
  `crates/core/src/repos/preferences.rs::clear` still called a cleared
  theme "follow the machine" instead of Comptoir, the default (`just
  types` regenerated `ThemeChoiceDto.ts` to match); `crates/api/src/gates.rs`
  still said eleven reads and that permissions were not yet applied, where
  the closing review gated five more for sixteen and T3 applied them
  the same day; the `justfile`, `README.md` and `CLAUDE.md` claim-comments
  did not say a bare `cargo` needs `CARGO_TARGET_DIR` exported, not only
  `just claim`; `apps/desktop/e2e/README.md`'s Files list was missing
  `till-cashier.spec.ts` and misordered the till files; `docs/features.md`
  §5 credited `see_cost_and_margin` with gating the dashboard (it is
  `see_reports`) and left five audit-log categories out of its list. Three
  new `.claude/stale-homes.md` rows (desktop screens list, gated-read
  count, API error codes). Checked and found correct, not stale:
  `context/roadmaps/` M4 entry, `context/plans/20260908-m4-team.md`,
  `docs/features.md` §5/§8 elsewhere, `apps/desktop/e2e/README.md`'s
  screenshot section, `context/processes/20260908-machines-and-heavy-jobs.md`,
  `.github/workflows/ci.yml`, `crates/core/src/lib.rs`'s mod list. Not
  checked: the boss site (`~/.dz-night/report/`), out of this sweep's
  file list.
- 2026-09-11: stale-info sweep of `context/` after the M3 checkpoint and the design-system merge: seven pages fixed on `main` (4229651, plus a same-day follow-up fixing an unescaped apostrophe that commit left in the design plan's YAML) — a task-count drift (M3's plan and now.md still said "nine tasks" after T0 grew it to ten), the roadmap's M4 entry still `planned` with no note of its ten tasks, the design plan's D2 still describing the theme select falling back to the machine's preference (superseded by Comptoir as the default), D4/D5 still calling the Astro landing "a later plan" instead of naming the `landing-page` plan that replaces D5, the repos page's CI row still describing every-PR Windows/coverage runs on GitHub instead of the mirror workaround, the M3 loop's 16:20 entry still saying "two jobs", and a 2026-09-08 reference recommending a since-superseded per-worktree `CARGO_TARGET_DIR`. Left unverified: whether Cloudflare (as opposed to a container host) was ruled out for the app's own demo deploy. Not touched (another agent's in-flight work, or out of scope): `.claude/stale-homes.md` needs a new CI row; `docs/`, code and `research/`.
- 2026-09-10 06:50: stale-info sweep of the repo and the boss site after the M2 merge: 12 prose items (roadmap headings and the merge sentence, the numbering paragraph, a process page subject, four boss reference pages), 2 justfile comments; context fixed on `main`, docs and justfile through `m3/stale-docs` into the milestone branch, the site redeployed; three new rows in `.claude/stale-homes.md`.
- 2026-09-10: M2 merged to main (PR #19, 79732ca). Rulings taken during the build are in docs/features.md §2 to §4; the open questions for the comptable are on the status site's M2 checkpoint page; the M4 plan carries the permissions M2 leaves open (override, correction, cancel, avoir, closing a fiche).
- 2026-09-09: M1 T3 (documents, gapless numbering, stock ledger, audit log, POST /sales; 8985d9d), T7 (three languages, RTL; 33ad612), T8 (daily backup, restore in place; dcf6f99), T4 (till screen as home; 415ade3) and T5 (80 mm ticket rendered by the core, nine goldens; 9cc9410) merged into `m1/2026-09-09` by the loop, each after dz-review lenses and a fix round. Docs sweep merged (d03ab61); code sweep on `m1/sweep-code` in flight. Checkpoint PR waits for `/dz-pr m1/2026-09-09`. T6 blocked on the printer.
- 2026-09-09: M1 T2 merged into `m1/2026-09-09` (settings screen: store block, dated régime on the shop's UTC+1 calendar; the review moved the seller snapshot and back-dating into T3). Stale-info sweep of the repo: context fixed on main, docs on `m1/stale-docs`; the generic `stale-check` skill is user-level, `.claude/stale-homes.md` holds where each fact lives.
- 2026-09-09: M1 T0 (launch token on every API route, transport-and-auth section in architecture.md) and T1 (products CRUD with edit, PUT /products/{id}) merged into `m1/2026-09-09`; checkpoint PR to main waits for `/dz-pr m1/2026-09-09`.
- 2026-09-09: PR #14 merged, M0 closed. Review of the branch found and fixed: a near-total discount printing a negative TVA base (share now capped at the group HT), the API panicking on i64::MIN, raw SQLite text reachable on the wire, the mockup writing the amount in words differently from the core. Coverage in CI scoped to the two crates with tests.

## Pending

## Not started

## M0 ladder (closed; kept as the record of what M0 was)

M0 ran step by step, Samir reviewing between steps. M1 runs as the loop
described under Active; its tasks and statuses are in the M1 plan, not
here.

1. [x] Stamp, TVA, rounding, facture mentions, numbering, IFU, words read from primary sources; all in `research/legal-fiscal/2026-09-08-fiscal-sources-and-findings.md` and the Source column of `docs/features.md` (R1, R2, R3, R4, R5 done; R6 partial; R3's citations are in `research/legal-fiscal/2026-09-08-facture-and-ticket.md` and the party identifiers row of `docs/features.md`)
2. [x] `Money` newtype and `pct` in `crates/core`, first fixtures, first proptest (money plan T1)
3. [x] TVA grouping per rate and global discount spread; fixtures shared with `design/shared/money.js` via vitest (T2, T5)
4. [x] Stamp duty per step 1 (T3, T8)
5. [x] Amount in words fr/ar/en with golden files (T4, R6)
6. [x] First migration (`shop_id`, products with a price model for both regimes, settings with the dated régime fiscal, one seeded owner user), `crates/api` with GET/POST /products, `packages/shared` with the ts-rs types, and the products screen reading it, shown on the laptop; the `webapp-testing` and `frontend-design` trial (R9) runs on that screen

## From Anouar, 2026-09-08 (Discord), to lock after the steps

- Product name proposal: "Dinar".
- GitHub organisation created; Samir invited. Repo transfer + bundle
  identifier change when the name is final.
- Anouar worked on a similar product before: the law and the calculations
  are not simple; an accountant may be needed as the project grows (R8).

## Decisions from Samir, 2026-09-08 (evening)

- E2E: ObserveOne is the e2e tool for this product; outside skills only
  if they prove better.
- Tauri dev loop: web UI on the WSL box (today `just api` then `just dev`); the laptop
  and the native window only when a feature is ready for a full-app pass.
- Versioning from the first release: semver + git short hash + build date,
  all three embedded in the binary and shown in About.
- Payment modes v1: cash, credit (customer ledger), card on a TPE with no
  integration; transfer and cheque later.
- Milestone order (later that evening): a cash sale with a printed ticket
  ships before customers and debt; sequence and exit criteria, no dates.
  `docs/roadmap.md`.
