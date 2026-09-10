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
    desc: 'Yearly reset of every document series, the common practice in Algeria (Samir, 2026-09-10): the counter row is keyed by series and year taken from the shop clock (`facture:2026`), the first number of a year is 1, the printed and stored number carries the year (`FA-2026-000001`; documents gain a `series_year` column, unique with kind and number per shop), the number is still taken inside the issuing transaction so nothing gaps within a year; a cross-year test drives the clock over 31 December; docs/features.md §4 Numbering row updated and the R8 comptable question narrowed to confirming the practice; existing rows keep their year from created_at in the migration'
    status: 'pending'
  - id: 'T1'
    desc: 'Migration 2026-09-10-000008: suppliers (name unique per shop, phone, address, RC, NIF, NIS, AI, notes, active), supplier_ledger (append-only, kinds opening/purchase/payment/adjustment, one direction per row, payment_mode iff payment, the same CHECKs as debt_ledger), purchases (supplier, supplier document number, date, due date, transport and extra costs, paid now, status ordered/partially_received/received), purchase_lines (product, qty milli, unit cost centimes, qty received milli), expense_categories seeded fr/en/ar keys (rent, electricity, water, salaries, transport, maintenance, other), expenses (category, amount, date, note, user); a purchase is its own table and not a document kind: the shop is the buyer, the number is the supplier''s, no series of ours is taken; bon_de_reception stays a document kind for the receipt slip (T3); previous-version test with real M2 rows; models and repos'
    status: 'pending'
  - id: 'T2'
    desc: 'Suppliers: routes and screen on the customers pattern (list, fiche route `/suppliers/$id`, opening debt as a ledger row, payment to a supplier, adjustment with reason, close with reason when debt is open), the balance from the ledger, audit `supplier.*`; the error payload gains nothing new (party_side already says which side); i18n ×3; e2e'
    status: 'pending'
  - id: 'T3'
    desc: 'Purchases: service `purchases::save` writes the purchase, its lines and the supplier_ledger `purchase` row for the unpaid part in one transaction (paid now becomes a `payment` row in the same transaction), `purchases::receive` takes a partial or whole receipt: one bon_de_reception document out of its own series naming the purchase, stock movements `purchase` at the landed unit cost (extra costs spread over the lines by value, rounded down, remainder on the last line, dz-money), product cost price updated to the last landed cost (the margin reads the movement''s cost, not the product''s); a receipt above the ordered quantity refused; screen: purchase list, new purchase, receive; e2e'
    status: 'pending'
  - id: 'T4'
    desc: 'Expenses: routes, screen (list by month, add, categories seeded), audit; the cash position rule written once in the core: cash in from cash sales and cash payments minus cash refunds, supplier cash payments and expenses, per day and per month, from the ledgers and not from a stored number'
    status: 'pending'
  - id: 'T5'
    desc: 'Stock re-derive with drift report: `stock::rederive(shop)` recomputes quantity on hand per product from the movements ledger, compares with the cached column, writes an audit row per drift and returns the report; run at app start once a day (settings store the last run day) and from a settings button; the drift list shown in settings; a test that forges a cached quantity and sees the drift reported and the cache corrected'
    status: 'pending'
  - id: 'T6'
    desc: 'Dashboard: one core query set behind `GET /dashboard?day=` reading the shop clock: today and this month sales (ttc, count), gross margin (sales ht minus cost of goods sold from the sale movements'' unit cost), expenses, cash position (T4 rule), low-stock list, top ten products by quantity and by margin, outstanding customer debt and supplier debt (sum of each ledger); screen as the index route replacing the placeholder; every number integer centimes, no f64; property test: margin plus cost equals sales ht to the centime'
    status: 'pending'
  - id: 'T7'
    desc: 'Excel: build-vs-buy checked in the brief (rust_xlsxwriter for writing, calamine for reading, both pure Rust, no new C dependency); export products, sales, customers, suppliers from the settings screen through the sandboxed download; product import from a downloadable template with a dry run that lists refusals per row before anything is written; barcode_label print (EAN-13 on a label roll, sizes decided in the brief) from the product drawer'
    status: 'pending'
  - id: 'T8'
    desc: 'Closing sweep: docs/features.md §1 and a new §5 (suppliers, purchases, expenses, dashboard) in the present tense, architecture.md data model paragraph, the combined dz-review over main..m3, the M4 carry-ins updated (supplier payment permission, purchase and expense permissions), the boss page, the checkpoint PR'
    status: 'pending'
acceptance: []
---
# M3: stock in, expenses, reports

Tasks added 2026-09-10 when M2 closed; the milestone text is `docs/roadmap.md` § M3.
Two refactors from the M2 plan (T10 avoir Remaining table and kind CHECKs, T11 zod
client and till split) run first on the same milestone branch `m3/2026-09-10`.

Design choice taken, to be challenged by the plan lens: suppliers get their own
table and their own ledger (`supplier_ledger`), purchases their own table; the
`documents` table and `debt_ledger` stay customer-keyed. The alternative, one
`parties` table with a role and one party-keyed ledger, was rejected because every
M2 query, index, CHECK and screen is customer-keyed and a supplier never buys at the
till; two mirrored ledgers cost one repeated service, not a rewrite.

Demo that closes it: a purchase from a supplier lands stock and supplier
debt; the dashboard shows today's sales, gross margin, cash position, low
stock and both debts.

In: suppliers, purchases with partial receipt and extra costs,
`bon_de_reception`, expenses with seeded categories, the nightly re-derive
of quantity on hand with a drift report, dashboard and reports, Excel
export and product import.

