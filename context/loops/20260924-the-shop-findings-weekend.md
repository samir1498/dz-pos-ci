---
title: 'The shop findings weekend'
slug: 'the-shop-findings-weekend'
status: 'active'
category: 'loops'
created: 20260924
tldr: 'Session A of two. Builds the clear-cut notes from Samir''s hand test of the shop (plan shop-manual-test-findings, T1-T56), one PR per screen group, and draws mockups for the design calls (till layout, onboarding tour) for Monday. Owns every cargo run on the box. Session B runs the automation loop in parallel (20260924-the-shop-automation-and-report).'
roadmap: 'catching-lumina'
---
# The shop findings weekend

Samir, 2026-09-24, 18:41: split the weekend over two Claude sessions, one
per group of plans, each resumable by its own id after a restart. This is
session A, tmux `dz:claude`, id in `~/.dz-night/sessions.tsv`.

The plan it executes is
`context/plans/20260924-shop-manual-test-findings.md`. Every task there is a
note Samir made while walking the shop by hand; the desc says what he saw
and, where one was settled, the decision.

## The two sessions

| Session | tmux | Loop file | Owns |
|---|---|---|---|
| A (this) | `dz:claude` | this file | the findings plan, every cargo run |
| B | `dz:auto` | `20260924-the-shop-automation-and-report.md` | the automated QA plan round 1, the emulator phone test, the Arabic pass, the report site |

Both commit `context/` to main: `git pull --rebase` before every context
commit. Only A writes `context/progress/now.md`. B never runs cargo; when
it needs a gates run on its branch it sends A a message (ListAgents,
SendMessage) and A runs `just gates` there when no builder holds
`.cargo-target/.owner`.

## Budget

Anouar shares the Claude plan. Sonnet builders (dz-builder, and
dz-money-builder on Opus for schema or money), at most four agents across
both sessions, so A keeps to three and B to one. One Fable review of the
whole loop at the end, not per PR (dz-review lenses on Sonnet per PR).
If usage passes 90 percent, stop spawning and finish what is in flight.

## Groups, one branch and one PR each, in this order

1. Printing: T25 (Imprimer must print, not only preview).
2. Setup and the shop's identity: T1, T9, T24, T28, T37, T38, T55; then
   T56 (logo upload) and T47 step 1 (supplier paper attached to a purchase)
   together, one files design. T56/T47 touch the schema: money builder.
3. The till, the parts that do not wait on the layout: T16, T17, T18, T19,
   T23, T29, T50.
4. The cart survives and parks: T39, T40 (decided: in the database), T41.
   Schema: money builder.
5. Customers and debt: T6, T7, T30, T32, T33, T34 (decided: opening debt
   paid first).
6. Purchases and suppliers: T13, T35, T44, T45, T46, T49. T45 is a new
   gapless series: money builder.
7. Documents and the audit journal: T31, T42.
8. Backups: T51, T52.
9. Phone pairing: T53, T54.
10. Small UI: T2, T3, T8, T14, T15.

## Mockups only, for Samir on Monday

T11 with T20, T21 and T22 (the till layout, function keys and visible
shortcuts; `context/research/20260924-how-lumina-takes-a-payment.md` is the
input), T10 with T27 (the onboarding tour), T43 (lists on TanStack Table).
Use the `dz-mockup` skill; nothing ships.

## Out of this loop

T12 (merge the two report sites) and T26 (per-page OG images) belong to the
reports site: B takes T26, T12 waits for Samir. T4, T5, T36, T48 are B's.
T47 step 2 (OCR) is an idea.

## Guardrails

Every cargo command through `just`; logs under `~/.dz-night/logs` with an
exit line; commit and push after every step; never `pkill -f`, `git stash`,
`git add -A`, prettier. Worktrees torn down with `just worktree-rm` as each
branch merges. `df -h /mnt/c` before a build; at 20 GB free, clean first.
Leave the laptop alone. Ticks every ten minutes while builders run.

## Where it stands

2026-09-24 18:55: loop started. Builders on group 1 (printing, worktree `printing`, branch fix/imprimer-prints) and group 10 (small UI, worktree `small-ui`, branch fix/small-ui-findings, no cargo). Session B started in tmux dz:auto.

2026-09-24 19:20: small UI PR #177 open, needs `just gates` before merge (cargo busy). It moved the theme and language switchers out of the top bar into one floating corner widget, so there is one of each: tell Samir. Group 3 builder (shop identity) running; group 5 (customers and debt, money builder) started.

2026-09-24 19:28: gates queue once printing frees .cargo-target: (1) small UI #177 `just gates`; (2) B's qa/first-day-e2e (worktree first-day) `just gates` then `just e2e-first-day`, reply exit lines to session B. B's first-day spec has test.fail guards for T31 and T34: the branch merging second drops them.

2026-09-24 19:45: printing merged (#178, gates green): Imprimer opens the print dialog for ticket and facture. Follow-up T57 logged (avoir/proforma print button, thermal printer setting).

2026-09-24 20:36: small UI merged (#177, gates green on rerun after a lint fix). First-day gates (B) rerunning on 846921b0.

2026-09-24 21:20: B's first-day e2e green (gates + e2e-first-day on 846921b0), B merges #180. Queued for when a slot frees: B's qa/screenshot-artifacts (worktree shots, bed5e0c2) `just gates` + `just e2e`, don't commit the rewritten PNGs, reply exit lines to B. Mockup agent started on the till layout (worktree till-mockups). Tell B when the box is quiet enough for its ~5 min Arabic emulator run.
