---
title: 'Automated QA rounds for the shop'
slug: 'automated-qa-rounds-for-the-shop'
status: 'paused'
category: 'other'
created: 20260924
tldr: 'Replace hand walks like 2026-09-24''s with three automated rounds: a first-day e2e, model-based fuzzing of the money rules, and an AI QA agent; four more kinds listed for later'
tasks: []
acceptance: []
---
# Automated QA rounds for the shop

Samir, 2026-09-24, during the manual shop walk
(`context/plans/20260924-shop-manual-test-findings.md`, T36): everything that
walk did by hand should run by itself later, including "a QA trying weird
things". Paused until the findings from the walk are fixed; nothing here
starts before Samir says so.

Each round catches a different class of bug, so the three are not
alternatives.

## 1. The first day of a shop, as one e2e

Today's walk as a Playwright test against a fresh database: first setup,
three products, open the till, a cash sale with a ticket, a credit sale,
the over-limit refusal and override, a debt payment, a facture, an avoir,
close the till with a count. It asserts the numbers the walk checked by hand
(570,90, 429,10 change, 1 457,80, 2 373,40, 1 873,40) and the audit rows.
Catches: a regression on the path every shop takes on day one. Cheapest of
the three; the e2e harness exists.

## 2. Model-based fuzzing of the money rules

A state machine generates long random sequences of operations (sale in
cash/card/credit, payment, adjustment, avoir, cancellation, purchase,
supplier payment, shift open/close) against the real services on a temp
SQLite, and after every step checks invariants that must always hold:
a customer's balance equals the sum of their ledger; the sum of documents'
remaining debt equals the balance minus the opening debt left; ticket and
facture numbers are gapless per series and year; stock equals the sum of
movements; a shift's expected cash equals opening cash plus its cash
movements; no amount is ever negative where it cannot be. proptest is
already in `kernel`, `core`, `retail`; `proptest-state-machine` adds the
stateful runner and shrinks a failure to the shortest sequence that breaks
a rule. Catches: money bugs no hand test reaches (order effects, the fifth
payment after a cancellation).

## 3. An AI QA agent round

An agent drives the built app through Playwright with a persona ("a
shopkeeper who does not read labels, types with spaces, double-clicks,
leaves mid-sale, switches to Arabic"), takes screenshots, and files each
finding into a findings plan like the 2026-09-24 one, with the screen, the
step and the screenshot. Run per milestone, not per PR (cost). Catches:
what the other two cannot, confusing UX, a button that looks dead, a
misleading label, which is most of what the 2026-09-24 walk found.

## More kinds, for later

- Mutation testing (`cargo-mutants`): flips operators in the money code and
  reports tests that still pass, i.e. tests that prove nothing. Checks the
  tests, not the app.
- Fault injection: kill the API mid-sale, drop the network during a
  dialog, cut power during a SQLite write, then check nothing half-saved
  (today's network hiccup is this case).
- Visual regression: Playwright `toHaveScreenshot` on key screens in fr and
  ar, so a layout break fails a run.
- Accessibility: `@axe-core/playwright` on each screen (contrast, labels,
  keyboard reach), which also backs the keyboard-first till.

## Order

1 first (cheap, protects day one), then 2 (money), then 3 (UX). The four
extra kinds only when one of the first three shows the need.

