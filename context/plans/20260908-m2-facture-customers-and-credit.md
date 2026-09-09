---
title: 'M2 facture customers and credit'
slug: 'm2-facture-customers-and-credit'
status: 'active'
category: 'milestone'
created: 20260908
tldr: 'Customers, debt ledger, numbered facture with words and stamp, statement; stub'
priority: 50
tasks:
  - id: 'T1'
    desc: 'Migration 3: customers (name, phone, address, RC/NIF/NIS/AI, party_kind company|consumer, credit_limit_centimes, warn_threshold_centimes, opening debt, notes, shop_id, active), append-only debt_ledger (party, document, debit/credit centimes, kind incl. opening, user, created_at), debt_allocations (which document a payment settled); documents gain the buyer block, ref_document_id and the old_balance/remaining_debt/total_debt snapshot; customer_belongs_to_shop check in the service since SQLite cannot add the FK; previous-version test from an M1 file; no yearly series reset (R8)'
    status: 'pending'
  - id: 'T2'
    desc: 'Customers service, routes (GET/POST/PUT /customers, GET /customers/{id}/ledger) and screen from the mockup (list with debt, limit and status tags; fiche drawer; ledger with running balance), opening debt as the first ledger row in the same transaction, audit log on every write, three languages and RTL, bindings regenerated, e2e per language'
    status: 'pending'
  - id: 'T3'
    desc: 'Credit sale at the till: sales::issue takes customer_id, refuses credit without one, warns at the threshold and blocks at the limit (owner override until M4), writes document, stock movements and the debt row in one transaction, snapshots the balance triple on the document; till gets the customer picker and the limit banner; a blocked credit sale burns no number (numbering_gapless); ticket_80mm gains a -credit golden ×3 with the balance amounts cross-checked; dz-review lenses before merge'
    status: 'pending'
  - id: 'T4'
    desc: 'Facture at the till: the one-tap ticket/facture switch of features.md §3, the buyer block snapshotted from the customer at issue, facture_requires_party_ids enforced in documents::issue on both halves (seller RC+NIS plus NIF/AI when set; company buyer RC+NIS; consumer buyer name+address) before take_next; after T3, never in the same wave (both touch sales.rs and till.tsx)'
    status: 'pending'
  - id: 'T5'
    desc: 'facture_a4 template on the ticket_80mm pattern (print/facture.rs, askama, the paper dictionary grows), amount_in_words on net_to_pay, balance block, stamp and signature blocks, goldens ×3 under fixtures/print/facture_a4 with every amount parsed back, A5 as a Paper parameter toggling @page size with a test that the two renders differ only there; Arabic disclosed unreviewed (R6); GET /sales/{id}/facture?lang= only after T4 (needs T1 only to build the Document by hand)'
    status: 'pending'
  - id: 'T6'
    desc: 'Avoir in its own series referencing its facture (ref_document_id), stock back as a return movement and the debt reversed as a ledger row in one transaction; a facture can be cancelled, keeps its number and reprints as "facture annulée" (golden set ×3); proforma in its own series, burns only its own number, moves no stock and no debt; after T4 and T5'
    status: 'pending'
  - id: 'T7'
    desc: 'Payments: POST /customers/{id}/payments settles oldest-first across documents through debt_allocations, a partial payment leaves the remainder on the oldest, over-payment refused; statement_a4 (opening balance, movements, closing balance for a date range) with goldens ×3 and words on the total; the stamped receipt for a later cash settlement waits for the comptable (R8) and is not in this task; after T2 and T3'
    status: 'pending'
  - id: 'T8'
    desc: 'The customer debt slip on 80 mm (balance and last movements, goldens ×3), the combined dz-review Pass 2 over T3, T4, T6 and T7 (money, numbering, deletion), features.md §3 and §4 updated to what M2 shipped, the checkpoint PR that closes M2'
    status: 'pending'
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
