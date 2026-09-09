---
title: 'M1 sale and ticket on one desktop'
slug: 'm1-sale-and-ticket-on-one-desktop'
status: 'active'
category: 'milestone'
created: 20260908
tldr: 'Products, till, cash sale, 80mm ticket on a real printer; stub until M0 closes'
priority: 60
tasks: []
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
