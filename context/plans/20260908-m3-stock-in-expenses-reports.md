---
title: 'M3 stock in expenses reports'
slug: 'm3-stock-in-expenses-reports'
status: 'active'
category: 'milestone'
created: 20260908
tldr: 'Yearly series reset, suppliers, purchases with partial receipt, expenses, stock re-derive, dashboard, Excel; nine tasks'
priority: 40
tasks:
  - id: 'T0'
    desc: 'Yearly reset of every document series, the common practice in Algeria (Samir, 2026-09-10; the comptable confirms the practice, R8): the counter key carries the year from the shop clock (`doc_facture:2026`, so `counters::take_next` takes an owned key and `Exhausted` names it as a String), the first number of a year is 1; `documents` gains `series_year INTEGER NOT NULL DEFAULT 0` by additive ADD COLUMN in migration 2026-09-10-000009 (after T1''s 000008 and T10''s 000007), backfilled from `issued_at` (the shop clock, never `created_at` which is UTC) and the existing UNIQUE (shop_id, series, number) still holds because the series string carries the year; the printed and stored number is `FA-2026-000001` through `number_of(kind, year, number)` so the statement and the debt slip print the right year for a prior-year facture; every print golden regenerated and reviewed as a diff; the four series assertions in documents_service.rs and the raw seeders keep compiling through the DEFAULT; a cross-year test drives the clock over 31 December at 23:30 UTC; docs/features.md §4 Numbering row updated; done 2026-09-10 08:36, merged into m3/2026-09-10'
    status: 'done'
  - id: 'T1'
    desc: 'Migration 2026-09-10-000008 with `shop_id` on every table, expense_categories included (rule 3): suppliers (name unique per shop, phone, address, RC, NIF, NIS, AI, notes, active), supplier_ledger (append-only, kinds opening/purchase/payment/return/adjustment, one direction per row, payment_mode iff payment, created_at from the shop clock, the same CHECKs and index shape as debt_ledger), supplier_allocations mirroring debt_allocations so what is still owed on one purchase is answerable, purchases (supplier, supplier document number unique per supplier where set, date, due date, transport and extra costs, status ordered/partially_received/received/cancelled/closed_short), purchase_lines (product, qty ordered milli, unit cost centimes, landed unit cost centimes fixed at save, qty received milli), purchase_receipts (its own number out of a `reception:<year>` counter, date, user, lines received; the bon de réception the roadmap names, stored and listed, never a row of `documents`, whose NOT NULL regime, payment_mode and customer key have no honest value for a purchase), expense_categories seeded with fr/en/ar keys (rent, electricity, water, salaries, transport, maintenance, other), expenses (category, amount, date, note, user), jobs (name, last_run_day) for T5; previous-version test with real M2 rows; models and repos; done 2026-09-10 08:22, merged into m3/2026-09-10'
    status: 'done'
  - id: 'T2'
    desc: 'Suppliers: routes and screen on the customers pattern (list, fiche route `/suppliers/$id`, opening debt as a ledger row, payment to a supplier settling purchases oldest-first through supplier_allocations with the same settle function shape as debt.rs, adjustment with reason, close with reason when debt is open), the balance from the ledger, audit `supplier.*`; the error payload needs no new field (the existing optionals carry the figures; `party_side` stays seller/buyer and is not reused); i18n ×3; e2e; done 2026-09-10 09:46, merged into m3/2026-09-10; a conflict error code (409) was added for the unique name'
    status: 'done'
  - id: 'T3'
    desc: 'Purchases: `purchases::save` writes the purchase and its lines with the landed unit cost fixed once (extra costs spread over the ordered lines by value, rounded down, remainder on the last ordered line, dz-money) and, when paid now, a supplier_ledger `payment` row; `purchases::receive` takes a partial or whole receipt in one transaction: a purchase_receipts row numbered from its counter, stock movements `purchase` at the line''s landed unit cost, the supplier_ledger `purchase` row for the value received (so debt follows goods and a purchase closed short owes nothing more), product cost price set to the last landed cost (the margin reads the movement''s cost, not the product''s), status moved; a receipt above the ordered quantity refused; save with ''received in full'' creates the receipt in the same transaction (the common case, and what features.md §1 describes); cancel a purchase with no receipt, close short after a partial one; return to the supplier: stock `return` movement and ledger `return` row at the landed cost; screen: purchase list, new purchase, receive, return; docs/features.md §1 Purchase paragraph reconciled in this task; e2e; done 2026-09-10 11:28, merged into m3/2026-09-10; the last delivery of a line takes the remainder so receipts never exceed the line; money with an order settles oldest-first'
    status: 'done'
  - id: 'T4'
    desc: 'Expenses: routes, screen (list by month, add, categories seeded), audit; the cash position rule written once in the core: cash in from cash sales and cash payments minus cash refunds, supplier cash payments and expenses, per day and per month, from the ledgers and not from a stored number; done 2026-09-10 10:06, merged into m3/2026-09-10; ruling: cash sales count net_to_pay (the stamp moved into the drawer), the stamp is its own figure'
    status: 'done'
  - id: 'T5'
    desc: 'Stock re-derive with drift report: `stock::rederive(shop)` recomputes quantity on hand per product from the movements ledger, compares with the cached column, writes an audit row per drift and returns the report; run at app start once a day (the `jobs` table holds the last run day) and from a settings button; the drift list shown in settings; a test that forges a cached quantity and sees the drift reported and the cache corrected; done 2026-09-10 11:17, merged into m3/2026-09-10'
    status: 'done'
  - id: 'T6'
    desc: 'Dashboard: one core query set behind `GET /dashboard?day=` reading the shop clock, every SUM coalesced to zero so an empty day answers zeros: today and this month sales (total_ttc and count over issued documents, cancelled excluded), gross margin (total_ht minus cost of goods sold from the sale movements'' unit_cost, reversals included), expenses, cash position (T4 rule), low-stock list, top ten products by quantity and by margin, outstanding customer and supplier debt (sum of each ledger); first, the reversal fix with a test: an avoir and a cancellation write their stock movement at the unit cost of the movement they reverse, not the product''s current cost (avoir.rs and documents.rs read `p.cost` today), else the margin drifts after every purchase; screen at `/dashboard` with a nav link, the till stays the launch screen (index.tsx redirects to it on purpose); every number integer centimes, no f64; property test: margin plus cost equals sales ht to the centime over a random history with reversals and cost changes; core and API merged 2026-09-10 14:41 (the reversal fix, the dashboard read, the property test); the screen is built on the kit after D3; ruling: margin after the whole-document remise, an assumption for the comptable (R8)'
    status: 'done'
  - id: 'T7'
    desc: 'Excel: build-vs-buy checked in the brief (rust_xlsxwriter for writing, calamine for reading, both pure Rust, no new C dependency); export products, sales, customers, suppliers from the settings screen through the sandboxed download; product import from a downloadable template with a dry run that lists refusals per row before anything is written; barcode_label print (EAN-13 on a label roll, sizes decided in the brief) from the product drawer; done 2026-09-10 14:55, merged into m3/2026-09-10 (rust_xlsxwriter, calamine, barcoders; a third decimal in an imported price is refused; a code that is not thirteen digits gets no label)'
    status: 'done'
  - id: 'T8'
    desc: 'Closing sweep: docs/features.md §1 and a new §5 (suppliers, purchases, expenses, dashboard) in the present tense, architecture.md data model paragraph, the combined dz-review over main..m3, the M4 carry-ins updated (supplier payment permission, purchase and expense permissions), the boss page, the checkpoint PR'
    status: 'pending'
  - id: 'T9'
    desc: 'A dev seeder (Samir, 2026-09-10 15:03: the shop starts empty, so screens and screenshots look bare): `just seed` fills a fresh shop file through the services with a realistic Algerian corner shop and thirty days of history, deterministic and idempotent; and the thirty-day series for the dashboard (per day and per week: sales, cost, margin, expenses, cash in) behind GET /dashboard/series, drawn as a chart in the screen wave'
    status: 'done'
acceptance: []
---
# M3: stock in, expenses, reports

Tasks added 2026-09-10 when M2 closed; the milestone text is `docs/roadmap.md` § M3.
Two refactors from the M2 plan (T10 avoir Remaining table and kind CHECKs, T11 zod
client and till split) run first on the same milestone branch `m3/2026-09-10`.

Design choice taken and held after the plan lens (2026-09-10 06:45): suppliers get their own
table and their own ledger (`supplier_ledger`), purchases their own table; the
`documents` table and `debt_ledger` stay customer-keyed. The alternative, one
`parties` table with a role and one party-keyed ledger, was rejected because every
M2 query, index, CHECK and screen is customer-keyed and a supplier never buys at the
till; two mirrored ledgers cost one repeated service, not a rewrite. The lens
moved the bon de réception out of `documents` (its NOT NULL regime, payment
mode and customer key have no value for a purchase) into `purchase_receipts`,
added `supplier_allocations`, put supplier debt on receipt rather than on save,
fixed the migration order (T1 000008, T0 000009) and found that reversals
reverse stock at the current cost, which T6 fixes before the margin is read.

Demo that closes it: a purchase from a supplier lands stock and supplier
debt; the dashboard shows today's sales, gross margin, cash position, low
stock and both debts.

In: suppliers, purchases with partial receipt and extra costs,
`bon_de_reception`, expenses with seeded categories, the nightly re-derive
of quantity on hand with a drift report, dashboard and reports, Excel
export and product import.

