---
title: 'M1 sale and ticket on one desktop'
slug: 'm1-sale-and-ticket-on-one-desktop'
status: 'active'
category: 'milestone'
created: 20260908
tldr: 'Products, till, cash sale, 80mm ticket on a real printer; stub until M0 closes'
priority: 60
tasks:
  - id: 'T0'
    desc: 'Transport and auth for the five links (loopback, phone or second till over LAN, desktop to cloud, phone to cloud, roles over HTTP) written in docs/architecture.md; loopback gets a per-launch bearer token injected by Tauri, required on every route but /health, passed by just api and the e2e; a request without it gets 401 in the envelope'
    status: 'done'
  - id: 'T1'
    desc: 'Products CRUD complete: PUT /products/{id} on the core update (deactivate as a field), edit drawer with every spec field, rate column in the table so the e2e sees the stored rate; checkpoint PR to main'
    status: 'in-progress'
  - id: 'T2'
    desc: 'Settings screen: store block (name, RC, NIF, NIS, AI, address, phone) and the dated régime fiscal control, read and written through the API; the seller block a ticket prints comes from here'
    status: 'pending'
  - id: 'T3'
    desc: 'Document model in core with every kind but only ticket issued, gapless numbering per kind on take_next (no dedupe loop, so the counter test must pin the series), stock movements ledger, the sale service (lines, discounts, totals, stock out) and POST /sales; the facture-or-ticket rule written into features.md §3; dz-review Pass 2 before merge'
    status: 'pending'
  - id: 'T4'
    desc: 'Till screen: product search and barcode entry, lines, line and global discount, tendered and change, cash or card; e2e proves a sale reduces stock and the totals match the fixture; dz-review Pass 2 before merge; checkpoint PR to main'
    status: 'pending'
  - id: 'T5'
    desc: 'ticket_80mm template with golden files in fr, en and ar (Arabic disclosed as unreviewed like words_ar), rendered from the document; IFU ticket prints no TVA line (regime_ifu_prints_no_tva)'
    status: 'pending'
  - id: 'T6'
    desc: 'ESC/POS over USB from the desktop; code and a byte-level golden on the WSL box, the real print needs Samir at the laptop with the thermal printer'
    status: 'blocked'
  - id: 'T7'
    desc: 'Three languages and RTL on every M1 screen (products, settings, till), a Playwright pass per language'
    status: 'pending'
  - id: 'T8'
    desc: 'Daily backup of the SQLite file, keep 30, restore from settings, the backup test opens the copy; checkpoint PR to main closes M1 except T6'
    status: 'pending'
acceptance: []
---
# M1: a sale and a ticket on one desktop

Stub. Tasks get added when M0 closes. The milestone text is
`docs/roadmap.md` § M1.

Demo that closes it: Anouar sells three products at the till, cash or card
on the TPE, and an 80 mm ticket comes out of a real thermal printer. Stock
goes down.

In: products CRUD, the till, the document model with only `ticket` issued,
the stock movements ledger, the store block and the régime fiscal control in
settings, `ticket_80mm` golden files in three languages, ESC/POS over USB,
RTL from the first screen, daily backup with restore.

Fixtures: `regime_ifu_prints_no_tva`, `facture_requires_party_ids` (seller
half). Also writes the facture-or-ticket rule into `docs/features.md` §3.
