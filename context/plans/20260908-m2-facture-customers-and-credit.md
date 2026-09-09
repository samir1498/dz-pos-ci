---
title: 'M2 facture customers and credit'
slug: 'm2-facture-customers-and-credit'
status: 'active'
category: 'milestone'
created: 20260908
tldr: 'Customers, debt ledger, numbered facture with words and stamp, statement; stub'
priority: 50
tasks:
  - id: 'T2'
    desc: 'Customers service, routes (GET/POST/PUT /customers, GET /customers/{id}/ledger) and screen from the mockup (list with debt, limit and status tags; fiche drawer; ledger with running balance), opening debt as the first ledger row in the same transaction, audit log on every write, three languages and RTL, bindings regenerated, e2e per language'
    status: 'done'
  - id: 'T4'
    desc: 'Facture at the till: the one-tap ticket/facture switch of features.md §3, the buyer block snapshotted from the customer at issue, facture_requires_party_ids enforced in documents::issue on both halves (seller RC+NIS plus NIF/AI when set; company buyer RC+NIS; consumer buyer name+address) before take_next; after T3, never in the same wave (both touch sales.rs and till.tsx)'
    status: 'done'
  - id: 'T5'
    desc: 'facture_a4 template on the ticket_80mm pattern (print/facture.rs, askama, the paper dictionary grows), amount_in_words on net_to_pay, balance block, stamp and signature blocks, goldens ×3 under fixtures/print/facture_a4 with every amount parsed back, A5 as a Paper parameter toggling @page size with a test that the two renders differ only there; Arabic disclosed unreviewed (R6); GET /sales/{id}/facture?lang= only after T4 (needs T1 only to build the Document by hand)'
    status: 'done'
  - id: 'T8'
    desc: 'The customer debt slip on 80 mm (balance and last movements, goldens ×3), the combined dz-review Pass 2 over T3, T4, T6 and T7 (money, numbering, deletion), features.md §3 and §4 updated to what M2 shipped, the checkpoint PR that closes M2'
    status: 'pending'
  - id: 'T1'
    desc: 'Migration: customers (identifiers, party_kind company|consumer, credit_limit_centimes null = no limit and 0 = no credit, warn_threshold_centimes null = no warning, opening debt as the first ledger row), append-only debt_ledger (opening, sale, payment, avoir, adjustment; one of debit/credit zero), debt_allocations; documents rebuilt outside diesel''s transaction (foreign_keys OFF, twelve-step copy with ids, foreign_key_check, ON) with customer_id FK RESTRICT, kind admitting quittance (variant with series and prefix, nothing issues it), ref_document_id FK, the buyer block with buyer_party_kind, the balance triple; models, repos, customers and debt services with audit; previous-version test keeps every child row and id; no yearly reset (R8)'
    status: 'done'
  - id: 'T3'
    desc: 'Credit sale at the till (after T2''s customer routes): sales::issue takes customer_id, refuses credit without one, warns at the threshold and blocks when the balance after the sale exceeds the limit (0 = no credit; owner override audited until M4), writes document, stock movements and the debt row in one transaction, snapshots the balance triple (old_balance before, total_debt after, remaining_debt = this document''s unpaid part) and writes its definition into features.md §3; till gets the customer picker and the limit banner; a blocked credit sale burns no number; ticket_80mm gains a -credit golden ×3 with the balance amounts cross-checked; dz-review lenses before merge'
    status: 'done'
  - id: 'T6'
    desc: 'Avoir in its own series referencing its facture, with its own lines (partial allowed, the total across avoirs never above the facture), stock back as return movements and the debt reversed as a ledger row in one transaction, any allocation excess becoming customer credit (the only way a customer holds a credit balance); cancelling a facture keeps its number, marks it annulée and, when it carried debt, issues a whole-document avoir in the same transaction; proforma in its own series, burns only its own number, moves no stock and no debt; every action audited; after T4, T5 and T7 (T7 owns debt.rs first)'
    status: 'in-progress'
  - id: 'T7'
    desc: 'Payments (after T2 and T3): POST /customers/{id}/payments writes the payment row and its allocations in one transaction, settles oldest-first, a partial payment leaves the remainder on the oldest, a payment above the outstanding debt is refused, the sum of allocations on a document never exceeds its net_to_pay (checked in the transaction), audited; statement_a4 (opening balance, movements, closing balance for a date range) with Key::ALL grown, goldens ×3 and words on the total; the stamped receipt for a later cash settlement waits for the comptable (R8)'
    status: 'done'
  - id: 'T9'
    desc: 'The avoir and proforma cases of facture_a4 (T5 titles all three kinds in one template): the avoir''s reference line and its own lines, the proforma wording, the annulée reprint of a cancelled facture, goldens ×3 for each with every amount parsed back; split into a second template only if the avoir needs more than a title and a reference; after T5 and T6''s model, same wave as T6'
    status: 'in-progress'
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
