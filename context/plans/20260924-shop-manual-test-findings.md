---
title: 'Shop manual test findings'
slug: 'shop-manual-test-findings'
status: 'active'
category: 'other'
created: 20260924
tldr: 'Dump for everything Samir''s manual shop test turns up (2026-09-24 onward); log only, fix later in grouped PRs'
tasks:
  - id: 'T1'
    desc: 'First setup: "Obligatoire." shows under "Votre nom" on first load, before the owner typed anything. Cause: FirstSetupScreen.tsx:25 derives nameError from the empty field with no touched/submitted gate. Show it only after blur or a submit attempt.'
    status: 'pending'
  - id: 'T2'
    desc: 'Wish (Samir): a floating theme picker like ObserveOne''s (~/observeone/projects/ObserveOne-frontend/src/components/ThemeSwitcher.tsx), reachable on every screen including first setup and sign-in.'
    status: 'pending'
  - id: 'T3'
    desc: 'Wish (Samir): a floating language picker (fr/en/ar) on every screen. First setup has none today, so an Arabic-speaking owner meets the first screen in French only.'
    status: 'pending'
  - id: 'T4'
    desc: 'Screenshots (automated run 2026-09-24): full-page shots of dashboard, till-credit-ar and customer-account-ar draw the sticky header and sidebar mid-page; the sidebar also stops short on tall pages (till, documents). Likely a capture artifact; confirm on a real screen.'
    status: 'pending'
  - id: 'T5'
    desc: 'Screenshots: the print preview frame on documents-avoir.png is blank. Check whether the preview renders by hand, or the shot is taken too early.'
    status: 'pending'
  - id: 'T6'
    desc: 'Documents: facture FA-2026-000003 shows Net 3 000, "Reste sur ce document 1 500", "Nouveau solde 3 000" while the customer''s balance is 1 500. Nouveau solde is probably the balance at issue time; the label does not say so. Verify the numbers before calling it a bug.'
    status: 'pending'
  - id: 'T7'
    desc: 'Till (Arabic): "لا يوجد زبون مطابق" (no matching customer) shows under a customer who is already selected (till-credit-ar.png).'
    status: 'pending'
acceptance: []
---
# Shop manual test findings

Samir's by-hand test of the shop module, started 2026-09-24 on the WSL box
(http://100.101.196.30:5173, built app, fresh database
`.dev/manual-0924.db`). Every finding lands here as a task first; nothing is
fixed during the walk. Fixes go out later, grouped into PRs by screen.

Also logged here: what the automated run's screenshots showed on the same
day (T4 to T7), so the manual walk can confirm or kill each one.

## How to add
One task per finding: where (screen, step), what was seen, the cause if
known with `file:line`, and "wish" when it is a request, not a defect.
