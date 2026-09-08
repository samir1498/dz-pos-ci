---
title: 'M2 facture customers and credit'
slug: 'm2-facture-customers-and-credit'
status: 'paused'
category: 'milestone'
created: 20260908
tldr: 'Customers, debt ledger, numbered facture with words and stamp, statement; stub'
priority: 50
tasks: []
acceptance: []
---
# M2: facture, customers and credit

Stub. Tasks get added when M1 closes. The milestone text is
`docs/roadmap.md` § M2.

Demo that closes it: a company customer buys on credit and gets a numbered
A4 facture with the amount in words and the stamp; pays part of it later;
the statement shows the balance.

In: customers, the append-only debt ledger with oldest-first settlement and
credit limit warn/block, facture A4 and A5, avoir, proforma, gapless
numbering per kind per year, `statement_a4`.

Fixtures: `numbering_gapless`, `facture_requires_party_ids`, the three words
golden files.

Blocked by: a real printed facture (open decision 4), the native review of
the Arabic words file (R6), the accountant's answers (R8).

