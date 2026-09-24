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
  - id: 'T8'
    desc: 'Wish (Samir): password and PIN fields get our own show/hide eye button. Browsers disagree today (Edge draws a native reveal, Chrome/Brave/Firefox draw none), so hide the native one (::-ms-reveal) and ship one eye everywhere, first setup and sign-in included.'
    status: 'pending'
  - id: 'T9'
    desc: 'Right after first setup the owner lands on the till with "Ouverture de la caisse" asking for the drawer float, in a shop with no products yet. Cause: routes/index.tsx:25 homeRoute sends every role to /till when retail is built, and routes/-till/session.tsx asks anyone without an open shift on each till mount. Owner (and manager) home should be the dashboard, or onboarding on an empty shop; the till prompt belongs to whoever sells.'
    status: 'pending'
  - id: 'T10'
    desc: 'Wish (Samir): basic onboarding for a new shop, a spotlight tour plus a first-steps checklist (add products or import the sheet, add a cashier and PIN, pair a phone, open the till, ring a first sale). Needs its own plan. Check open source before building: driver.js (MIT, no deps), react-joyride, shepherd.js (its license needs a look for a commercial app).'
    status: 'pending'
  - id: 'T11'
    desc: 'Samir hates the till layout: the cart column stacks customer, document type, cart, discount, totals, payment, amount, numpad, Valider and Encaisser in one tall column, so on a 1080p screen the numpad and Encaisser sit below the fold and every sale needs scrolling. Needs a new till design with no scrolling (numpad and pay always visible), done as a design pass first (mockups on the reports site''s design section, which today holds only the logo page), not a patch.'
    status: 'pending'
  - id: 'T12'
    desc: 'Later, outside this plan (Samir): merge the research and reports of ObserveOne and Dinar into one place, to clear old reports and duplicated unfinished ideas. Needs its own plan; the Dinar side today is ~/.dz-night/report (dinar-reports) plus context/research.'
    status: 'pending'
  - id: 'T13'
    desc: 'Wish, low priority (Samir dislikes pack sizes typed into product names, though it is normal retail practice since each size is its own barcode/SKU): an optional "contenance" field (number + unit, e.g. 1,5 L, 250 g) appended to the displayed name and usable for a price per litre/kilo on shelf labels. Names stay as they are until then.'
    status: 'pending'
  - id: 'T14'
    desc: 'Money inputs type right-aligned (MoneyInput.tsx:86, text-end, used on 14 screens) while every other field types left, which Samir finds disorienting. Make money inputs start-aligned like the others; keep end alignment only in tables and totals where digits stack in a column. Keep dir="ltr" on the amount.'
    status: 'pending'
  - id: 'T15'
    desc: 'Wording: "float" (drawer cash) clashes with the never-a-float-for-money rule. Replace it with "opening cash" in docs/features.md (8), code and test comments (crates/api gates table, dto/till.rs, routes/till.rs, tests), keeping historical plan slugs as they are. UI and identifiers already say opening cash / opening_cash_centimes.'
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
