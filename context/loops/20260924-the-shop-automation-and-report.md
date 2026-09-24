---
title: 'The shop automation and report'
slug: 'the-shop-automation-and-report'
status: 'active'
category: 'loops'
created: 20260924
tldr: 'Session B of two. Turns Samir''s 2026-09-24 hand test into automation: round 1 of the automated QA plan (a first-day end-to-end test), the phone pairing and phone sale on the Windows Android emulator with Maestro, an Arabic pass, and keeps the reports site current. Never runs cargo. Session A (20260924-the-shop-findings-weekend) builds the fixes.'
roadmap: 'catching-lumina'
---
# The shop automation and report

Samir, 2026-09-24, 18:26 and 18:41: automate the hand test with an emulator,
then split the weekend over two sessions. This is session B, tmux `dz:auto`,
id in `~/.dz-night/sessions.tsv`. Session A's file,
`20260924-the-shop-findings-weekend.md`, has the shared rules (budget,
who writes what); read it first.

## What to automate

The hand test is in `context/plans/20260924-shop-manual-test-findings.md`
(the intro and the data verified in each step) and the report at
`~/.dz-night/report/src/legacy/reports__2026-09-24-shop-test-day.html`.

1. Round 1 of `context/plans/20260924-automated-qa-rounds-for-the-shop.md`:
   a first-day Playwright test in `apps/desktop` that walks setup, three
   products, open the till, cash sale, credit customer with limit and a
   forced sale, debt payment, facture, avoir, supplier purchase and two
   payments, close with a count, backup and restore, asserting each figure
   the hand test checked (debt 1 819,85, supplier 23 836,00, drawer
   6 288,00, stock). Where a finding makes a step fail today, mark that
   step `test.fixme` naming the finding id, so A's fix turns it green.
2. The phone: `apps/mobile/maestro/pair-and-sell.yaml` and
   `scripts/demo-phone.sh` already pair Expo Go on an emulator and ring a
   sale. Emulator `dinar`, SDK on the Windows side:
   `/mnt/c/Users/Administrator/AppData/Local/Android/Sdk` (`adb.exe`,
   `emulator/emulator.exe`); `scripts/maestro-windows.sh` drives Maestro
   there. Pairing needs the API in LAN mode and the launch token in
   `EXPO_PUBLIC_API_TOKEN` (findings T53, T54).
3. An Arabic pass: the same first-day test under the `ar` project, and the
   Maestro flow once in Arabic if the phone has an Arabic locale.
4. T4 and T5 (screenshot capture artifacts, blank print preview in a
   screenshot), T26 (per-page OG images on the reports site), and add the
   new clips to the report as they land.

## Never cargo

A owns every cargo run. Run the API from a copied binary:
`~/.dz-night/bin/dzpos-api` (copied from `.cargo-target/debug/dzpos-api`
at the start; copy again only when A says main moved the API) on port 4417
with its own shop file `.dev/qa.db`, web preview on 5273, Metro on 8091.
Leave `dz-dev` (4317, 4318, 5173, 8081) alone: it is Samir's.
When a branch needs `just gates` or `just e2e`, message session A.

## Budget and guardrails

One agent at a time from this session. Same guardrails as A's file. Work on
a branch in a worktree (`just worktree <name> <branch>`), one PR per unit: the
first-day test, the phone flow, the Arabic pass, the report changes (the
reports site is its own repo, `Dinar-dz/dinar-reports`, committed and
pushed after every change). `git pull --rebase` before any context commit
to main; never write `context/progress/now.md`.

## Where it stands

2026-09-24 18:50: loop file written, session not started.

2026-09-24 19:10: started. Worktree `first-day` on `qa/first-day-e2e`:
`apps/desktop/playwright.first-day.config.ts` (own database, no
globalSetup so the test walks first setup, ports 4321/5176,
`DZPOS_E2E_API_BIN` runs the copied API instead of cargo) and `just
e2e-first-day`. One dz-builder writes `e2e/first-day/first-day.spec.ts`
from the hand-test database's figures. Desktop walk ends at debt 1 819,85,
supplier 23 836,00, drawer 6 288,00, stock Eau 69 / Café 116 / Sucre 49
(the report's 67/116/48 include the phone sale after the close).
