---
title: 'A clean context, then the till finished'
slug: 'a-clean-context-then-the-till-finished'
status: 'active'
category: 'loops'
created: 20260921
tldr: 'The pass before the replan. First the context store, the code comments and the status site stop claiming things that moved, because the replan reads those pages and a replan built on a stale page is worse than no replan. Then the till shifts plan finishes: the open and close screens, a cash refund that leaves the drawer, and the closing sweep. Then Arabic on a cheap thermal head, then one whole-loop review on the three lenses at fable and a standup. The modular replan is what comes after this file closes, and nothing here is written as though it had already happened.'
roadmap: 'catching-lumina'
---
# A clean context, then the till finished

Samir, 2026-09-21: pause, see what we built, what is left and what is stale,
fix the stale, finish what is left, and only then replan Dinar as modular.

The loop before this one is
`context/loops/20260920-closing-the-lumina-gaps-loop.md`. Its nine rulings,
its guardrails, its `ctx:` trailer shape and its "who does what" hold here
unchanged and are not repeated. Its Phase 0 and Phase 1 are done. Its Phase 2
is half done and finishes here, its Phase 3 is carried here whole, and its
filler list stays where it is. That file stays the reference for any detail
this one does not restate; when Phase D below closes, its status becomes
done and this file is the live one.

## Why a sweep comes before the work, not after

The replan will be written by reading `context/`, `docs/features.md` and the
roadmap. Every page that lies about what exists produces a plan for a product
we do not have. This morning the progress page said the shift service was
building while it was merged, and the status site still described the gate
table and the shared client as single files months after both became folders.
Those are cheap to fix today and expensive to discover inside a replan.

The sweep is also the last moment the loop's own numbers are worth trusting.
After the replan the milestones themselves move, and a count fixed now is a
count fixed in a page that is about to be rewritten.

## What the modular question is, and what it is not

Anouar proposed on 2026-09-21 that Dinar keep a shared core and grow modules
per trade, doctors first, the way Odoo does. Samir's answer was that it is
close to a new app and that he will research what a doctor's day needs before
anything is designed.

What that means for this file: **nothing here treats the pivot as decided.**
No plan is void, no retail task is dropped, and no page gets rewritten as
though Dinar were already multi-trade. A page that talks as though the pivot
has happened is the stale page, not the other way round.

Two measurements worth carrying into the replan, both taken 2026-09-21 rather
than guessed. The repo is 44 793 lines of code across 366 files, duplication
1.9%, comment density 26.9%, and about fifty lines of the whole thing are
genuinely dead. And of the core services by line, roughly a quarter is
generic (users, sessions, permissions, audit, settings, preferences, pairing,
backup), a little over half is a shop and nothing else (sales, purchases,
avoir, both ledgers, stock, pricing, till, dashboard), and the rest is
schema-shaped and rewritten per product anyway. The shareable quarter is the
cheap quarter. That is the fact the replan has to start from.

## Phase A: the context stops lying

Three read-only sweeps ran on 2026-09-21 against a written fact list, one on
the prose (`README`, `CLAUDE.md`, `docs/`, all of `context/`, the skills and
the agents), one on the code comments and the tooling (`justfile`, workflows,
`scripts/`, doc comments), and one on the status site at
`~/.dz-night/report/`. Their findings become the task list here.

Routing, from `.claude/stale-homes.md`: a `context/` page is committed
straight to main; a doc, a README, a skill or a code comment goes through a
branch and a pull request with the usual gates; the status site is fixed in
its own repo and deployed with its own script.

Two rules the sweep holds to, both learned here:
- a dated record is history. A handoff, a research note or a
  "decisions on <date>" list keeps its body and gains one line at the top
  saying when it was written and where the current state is.
- a fact fixed in more than one place gets a home and the other copies become
  pointers, with the row added to `.claude/stale-homes.md` before the fix is
  committed.

Done: every finding either fixed or written down as unverifiable and why, the
fixes committed in one commit per routing rule so a reader can see what
changed for which reason, and `context/progress/now.md` describing today.

## Phase B: the till shifts plan finishes

`context/plans/20260921-till-shifts-a-float-and-a-count.md` owns the detail.
T1 through T4 are done: the table, the models and repos, the service that
counts one cashier's own cash over a window pinned at both ends, and the four
routes with the two permissions. What is left is T5, T6, T7, in that order,
one task at a time.

T5 is the open popup at the first sign-in of the day and the close modal that
shows the expected figure before the cashier types what is in the drawer.
T6 is a refund that leaves the drawer, and the shift figure on the dashboard
and expenses screens. T7 is the closing sweep: the shift list screen, the
end-to-end run with a real cashier, and the docs.

One thing T5 inherits and must not work around: a cashier does not hold
`see_reports`, so their own figures come from `GET /till/shifts/open` before
the close and from the close call's own answer after it, never from the gated
read. A screen that needs the gated read has the wrong shape.

## Phase C: Arabic on a cheap thermal head

Carried whole from the previous loop's Phase 3. Nothing about it changed and
it is not restated here; read it there.

## Phase D: one review of everything, then the standup

The three lenses run once over everything this loop and the last one merged,
with `model: "fable"` passed to the Agent tool, which overrides each agent
file's own model. One sharp review of the whole pass rather than a dozen
small ones, which is Samir's rule from 2026-09-17. The lenses find; this
session proves every finding by reading the code or by mutation before any
of it is acted on.

Then `dz-standup` for Anouar, in plain words, with no task codes in the body.

## The gate to the replan

This file closes when Phase D's standup is published. The replan is a
separate plan page and a separate conversation, and it starts from the
research Samir is doing on what a doctor's day actually needs, not from an
architecture preference. The question it has to answer first is not how to
split the code. It is which parts of a shop's day and a clinic's day are the
same transaction, and the code follows that answer.

## Guardrails, added to the ones the last loop lists

- The sweep reports; this session fixes. A lens that both finds a stale line
  and rewrites it has marked its own paper.
- No page is rewritten to describe the modular product until the replan
  exists. Writing the plan into the pages first is how the pages become
  stale again on the day the plan changes.
