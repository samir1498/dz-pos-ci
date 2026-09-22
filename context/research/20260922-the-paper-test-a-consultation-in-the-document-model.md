---
title: 'The paper test: a consultation in the document model'
date: 2026-09-22
status: 'draft'
tldr: 'Walking the généraliste consultation from the cabinet-day draft through `documents`, `document_lines`, the payment mode enum, the stamp rule and the customer fiche turns up nineteen tears, most of them "no meaning" (a column asks for a fact a consultation has none of) rather than "wrong rule" or "no home". Two of the three papers, the ordonnance and the feuille de soins, cannot be a document in this file at all; only the reçu is even a candidate, and its own fiscal status is unsourced. The tiers payant case adds several more: a fund that owes the cabinet has no table anywhere in this schema, not even a wrong one. Every tear sits in the retail half of the code (documents, document_lines, stock_movements, customers, debt_ledger, the stamp and TVA rules); nothing in the shared quarter (users, sessions, permissions, audit, settings, preferences) tears at all, so on this one test the kernel line the plan is trying to draw holds exactly where D1 said the cheap quarter would.'
---

This is written 2026-09-22 against main at commit `134a788`, from the
desk-research draft at
`context/research/20260922-a-doctors-cabinet-day-on-paper.md`, which
Samir's own research may replace; if it does, this tear list would be
redone against whatever it corrects. What follows takes the one
consultation that draft's last section names for D3: a patient walks
into a médecin généraliste's cabinet in Algiers with no appointment, is
seen the same afternoon, and pays 1 500 DA in cash at the desk, leaving
with three papers, an ordonnance, a feuille de soins and a reçu. The
cabinet owes nobody anything from the visit; the one debt running
outward is CASNOS, once a year, unrelated to it. The tiers payant case
from the draft's "What is owed" section is the harder second case: a
fund settles with a conventioned médecin directly, and the cabinet is
owed by a party never in the room.

## The consultation as a ticket

Dinar's document model is `documents` (`crates/core/src/schema.rs:114`)
with its lines in `document_lines` (`schema.rs:163`); a 1 500 DA cash
ticket is the nearest existing shape, so the columns are walked as if
this consultation were rung on the till. `documents.kind`
(`schema.rs:118`, `DocumentKind`,
`crates/core/src/models/sql_types.rs:110`) has exactly seven values:
`ticket`, `facture`, `proforma`, `bon_de_livraison`, `avoir`,
`bon_de_reception`, `quittance`, none naming a fee note for a service
with no goods behind it; `ticket` is the closest reading, every other
kind a shop-specific fit. `series`/`number` (`schema.rs:119-120`), the
gapless per-kind, per-year counter
(`crates/core/src/repos/counters.rs:30-52`, taken inside
`crates/core/src/services/documents.rs:262`),
could seat a reçu with no schema change, at the cost of sharing one
sequence with a shop's cash tickets. `payment_mode` (`schema.rs:124`,
`crates/core/src/money/stamp.rs:30`) fits `Cash` exactly; `regime`
(`schema.rs:123`, `crates/core/src/models/document.rs:196-201`)
resolves to `réel` because médecins are excluded from IFU since 2022,
by coincidence rather than a choice a doctor makes. `seller_*`
(`schema.rs:125-131`) hold RC, NIF, NIS, AI, address, phone, a
commerçant's identity; whether a médecin even carries an RC is
unsourced, and a médecin's real identifier (Ordre national des
médecins registration) has no column at all. `customer_id`/`buyer_*`
(`schema.rs:132-139`) stay `None` on a walk-in, which works, though the
columns say "buyer" where the fact is "patient".
`total_ht_centimes`, `tva_centimes`, `total_ttc_centimes`
(`schema.rs:141-145`) presume a taxable base and a TVA rate; whether a
consultation carries TVA at all is unresolved (draft: 9% under CTCA
art. 23 versus exonéré outright), exactly the comptable question D3
exists to expose. `stamp_centimes` (`schema.rs:146`,
`crates/core/src/money/stamp.rs:36-63`) would run mechanically on
1 500 DA cash and print a real stamp above the 5 DA floor, but Code du
timbre art. 100-I taxes "une opération facturée", and the draft could
not establish a médecin's reçu as a facture in that sense: the formula
runs cleanly on a premise that is itself in question, a wrong-rule tear
rather than a missing one. `net_to_pay_centimes` (`schema.rs:147`)
inherits both unresolved figures. `tendered_centimes`/`change_centimes`
(`schema.rs:148-149`) fit a cash desk exactly. `old_balance_centimes`,
`remaining_debt_centimes`, `total_debt_centimes` (`schema.rs:150-152`)
are `None` together (`crates/core/src/models/document.rs:479-496`)
since there is no customer here; tiers payant revisits this below. In
`document_lines`, `product_id` (`schema.rs:169`, nullable) is the
one column a consultation leaves empty, so the `document_lines ->
products` join (`schema.rs:508`) never fires. `qty_milli`
(`schema.rs:172`) has no natural value, since a consultation is not
sold in thousandths of a unit; any figure written is a fiction the
format tolerates. `name` (`schema.rs:170`) could snapshot "consultation
généraliste", standing in for an act rather than a good, and
`unit_price_centimes`/`line_total_centimes` (`schema.rs:173,176`)
would hold 150 000, arithmetically fine but hiding that the fee was
never "priced" the way a shelf price is. `rate_bps` (`schema.rs:175`)
inherits the unresolved TVA question. The sharpest tear sits behind
the line: a sale line and a stock
movement travel together, and `stock_movements` (`schema.rs:192`) requires
a non-nullable `product_id` (`schema.rs:196`), and
`crates/core/src/services/sales.rs:228-1061` writes one `stock::record`
call per priced line (`sales.rs:434-441`) inside the document's own
transaction. A consultation moves no product, so there is no movement
to write; the file's own claim that "a sale becomes a fiscal document,
its stock leaves the ledger, and both commit together" (`sales.rs:1-7`)
describes an act this consultation can only half perform. `documents`
and `document_lines` can be coerced into holding the numbers; the
transaction written around them cannot.

## The three papers

**The reçu**, the paper for the 1 500 DA itself, is the only candidate
for `documents` in this file's sense, since it is the only one of the
three that names a total. Whether it may actually be numbered into a
series is unresolved rather than refused: the draft found médecins
classed a profession libérale non commerciale in every fiscal source
read, with nothing saying whether that carries them into or out of loi
04-02's ticket-or-facture duty. If the answer is "no duty at all", a
reçu is a plain receipt with none of the apparatus
`docs/features.md:550-759` builds for a ticket or a facture: the model
can hold its total but not say whether it may be numbered like a ticket.

**The ordonnance** cannot be a document here at all: no price, no
total, no TVA line, nothing `total_ht_centimes` or
`net_to_pay_centimes` can describe. **The feuille de soins** is a claim
form the patient carries to CNAS, not a paper the cabinet keeps in its
own numbered series. Even granting it a price, which it does not
carry, it is issued into a fund's claims process the cabinet has no
ledger relationship with; `documents.customer_id` and `buyer_*`
describe a buyer the cabinet deals with directly, while a feuille de
soins is a claim the cabinet stamps, hands over, and loses all further
interest in.

Only the reçu is a document in `documents`' sense, and even it stalls
on an unanswered fiscal question; the other two have, correctly,
nothing to tear against, because they were never candidates.

## The payer that is not the person in front of you

Tiers payant, from the draft's "What is owed" section: a fund settles
directly with a conventioned médecin, the patient pays nothing or a
reduced part, and the sourced figure is 400 DA per généraliste
consultation under the conventionnement du médecin traitant scheme.
This is the harder case because the payer is neither the person at the
desk nor, in Dinar's schema, anyone the model has room to name.

`customers` (`schema.rs:222`) holds `name`, `party_kind`
(`crates/core/src/models/sql_types.rs:142-146`), `phone`, `address`,
RC/NIF/NIS/AI, `credit_limit_centimes`, `warn_threshold_centimes`,
`notes`, `active`. A fund could be typed in as a `company`-kind fiche
with a name and no RC, the way any company fiche starts
(`docs/features.md:441`), but nothing fits what a fund actually is:
`credit_limit_centimes` caps what the shop will sell on credit, and a
fund does not buy on credit, it owes the cabinet for services already
rendered, the reverse of what the row is built for; `party_kind` is
`company` or `consumer` with no third value for a public fund paying
after the fact.

`debt_ledger` (`schema.rs:243`) is worse: its `kind`
(`crates/core/src/models/sql_types.rs:152-159`, `Opening`, `Sale`,
`Payment`, `Avoir`, `Adjustment`) is every variant a movement on a debt
the customer owes the shop, while tiers payant needs the opposite
direction, a receivable the cabinet holds against the fund, created by
seeing a covered patient rather than by a `sale`.
`debt_ledger.customer_id` (`schema.rs:247`, non-null into `customers`)
means even a fund forced into a `customers` row could only appear as a
debtor, never as a party the cabinet is owed by, and `supplier_ledger`
(`schema.rs:353`), the mirror direction for what the shop owes a
supplier, does not fit either: a fund owing the cabinet is a third
direction the schema has no table for, a missing home rather than a
wrong column. The 400 DA tariff itself has no field: not the patient's
price, which may differ or be zero under tiers payant, and not a debt
row, since the debt runs to a party the schema cannot name.

## The tear list

1. `documents.kind` (`crates/core/src/models/sql_types.rs:110-118`): the seven-value enum has no variant for a fee note; `ticket` is the nearest fit but conflates a doctor's fee with a shop's cash sale. Class: no meaning.
2. `document_lines.product_id` (`crates/core/src/schema.rs:169`) and the `document_lines -> products` join (`schema.rs:508`): a consultation line names no product, so the join never fires. Class: no meaning.
3. `document_lines.qty_milli` (`schema.rs:172`): a consultation has no natural quantity in thousandths of a unit; any value written is a fiction the format accepts. Class: no meaning.
4. The per-line stock movement `sales.rs:434-441` writes, against `stock_movements.product_id` non-null (`schema.rs:196`): a consultation moves no stock, so half of what the till's one transaction (`crates/core/src/services/sales.rs:1-7`) is documented to do cannot happen. Class: no meaning.
5. `documents.tva_centimes` and the `document_tva` recap (`schema.rs:141-144,181-190`): whether a consultation carries TVA at 9% or is exonéré is unresolved (cabinet-day draft, "Fiscal rules: TVA"). Class: no home.
6. `document_lines.rate_bps` (`schema.rs:175`): inherits the same unresolved TVA question; the only default is a product's `categories.default_rate_bps` (`crates/core/src/schema.rs:8-15`), and a consultation has no product and no category. Class: no meaning.
7. The stamp formula (`crates/core/src/money/stamp.rs:36-63`) would run and print a non-zero stamp, but whether a médecin's reçu is a "facture" for Code du timbre art. 100-I is unestablished (cabinet-day draft, "Droit de timbre"). Class: wrong rule.
8. `documents.seller_rc/nif/nis/ai` (`schema.rs:126-129`): built for a commerçant's registration; a médecin's professional identifier has no column among them. Class: no home.
9. `documents.regime` (`crates/core/src/models/document.rs:196-201`): resolves to `réel` only because médecins are excluded from IFU, a coincidence of exclusion rather than a choice the column was built to record. Class: wrong rule.
10. The ordonnance has no candidate field anywhere in `documents` or `document_lines`: no total, no TVA, nothing to snapshot. Class: no home.
11. The feuille de soins has no candidate field either: a claim to a fund the cabinet has no ledger relationship with, not a document in the cabinet's own series. Class: no home.
12. The reçu's own fiscal status (numbered document versus plain receipt) is unresolved, so whether it should burn a `counters` row (`crates/core/src/repos/counters.rs:30-52`) at all is undecided. Class: no home.
13. `customers.party_kind` (`crates/core/src/models/sql_types.rs:142-146`): a fund is neither `company` nor `consumer`; no third kind exists for a payer who owes the cabinet rather than being owed by it. Class: no meaning.
14. `customers.credit_limit_centimes` / `warn_threshold_centimes` (`schema.rs:234-235`): built to cap credit sales; meaningless for a fund that never buys anything. Class: no meaning.
15. `debt_ledger.kind` (`crates/core/src/models/sql_types.rs:152-159`): every variant describes what a customer owes the shop; none describes what a fund owes the cabinet. Class: no home.
16. `debt_ledger.customer_id` (`schema.rs:247`, non-null into `customers`): can only record a fund as a debtor, never as the cabinet's own creditor. Class: no home.
17. The tiers payant receivable itself: no table anywhere (`documents`, `customers`, `debt_ledger`, `supplier_ledger`) models a receivable created by seeing a covered patient rather than by a sale. Class: no home.
18. The 400 DA conventioned tariff has no field: not the patient's price (which may differ or be zero) and not a debt row, since the debt runs to a party the schema cannot name. Class: no home.
19. CASNOS, the one outward obligation, is annual and tied to the doctor's declared income, not to any visit; nothing in `documents`, `debt_ledger` or `supplier_ledger` represents a periodic obligation unconnected to a transaction. Class: no home.

## What the tear list says about the boundary

Every one of the nineteen tears sits inside `documents`,
`document_lines`, `document_tva`, `stock_movements`, `customers`,
`debt_ledger`, or the money kernel's stamp and TVA rules: either the
shop-only half of the code or the money kernel Anouar himself calls
real, shared reuse (the plan,
`context/plans/20260921-whether-dinar-becomes-a-core-and-modules.md:56-72`).
None touches `users`, `sessions`, `permissions`, `audit_log`,
`settings` or `preferences`, the four tables T11 already proved cut to
an empty `RINGS_STILL_OPEN` (plan:203-210) and the tables the plan
calls the trade-independent quarter. A cabinet still needs a user
table, a session, a permission check and an audit row for the same
reasons a shop does, and this test found nothing that says otherwise.

That is the finding worth stating plainly, because it is the one D6
asked for: on this one billed consultation, the tear is entirely
retail-side, all of it in the half the plan already measured as "a
shop and nothing else" (plan:58-60). Anouar's claim that customers and
patients are the same module with different strings survives the
fiche's identity columns, name, phone, address, and fails at the first
column that describes money: a credit limit, a debt-ledger direction,
a total that assumes a TVA rate and a stamp rule built for a
commercial sale. The kernel Anouar wants to share is untouched by this
test; the module he wants to add is exactly where every tear lands.
