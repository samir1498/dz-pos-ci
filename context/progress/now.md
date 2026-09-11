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
`just ci` (branch `ci/lean-and-mirror`, not yet merged to main) now pushes a
branch to the mirror `samir1498/dz-pos-ci` so gates run on Samir's own
minutes without waiting on Anouar. Still open: the host for the demo
deploy (a container on Koyeb or Render, or Cloudflare Containers — the
box-side tunnel pieces are stopped; unverified whether Cloudflare was
ruled out for the app itself, as opposed to the landing page's static
Cloudflare Pages), and Anouar's GitHub billing fix itself, which the
mirror works around but does not close.

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

1. [x] Stamp, TVA, rounding, facture mentions, numbering, IFU, words read from primary sources; all in `research/legal-fiscal/2026-09-08-fiscal-sources-and-findings.md` and the Source column of `docs/features.md` (R1, R2, R4, R5 done; R3, R6 partial)
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
