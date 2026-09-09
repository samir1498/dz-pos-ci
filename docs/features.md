# Features: what dz-pos does

Normative. If a screen or a rule is not here, it is not in scope. The
competitor teardown, the Algerian facture field list and the market notes
live in `research/` (competitor material under
`research/competitors/2026-09-07-lumina-and-market/`); this file says
what *we* build, and links there for *why*.

Status legend: **v1** ships in the first release; **later** is agreed but
not scheduled; **open** waits on a decision listed at the bottom.

## Scope

- One shop, one SQLite file, desktop first. The phone is a thin client of
  the desktop (or of a hosted server later). Every table carries `shop_id`
  from day one even though v1 only ever has one value.
- Languages: Arabic (RTL), French, English on every screen and every
  printed document, independently selectable (UI language ≠ print language).
- Out of v1: cloud mode, phone-only offline, product variants/colours,
  bundles, promotions, AI invoice scanning, e-invoicing. Each is either
  parked in the architecture notes or listed under open decisions.

## Build order

Each step is usable on its own and ships with its tests before the next
starts. `docs/roadmap.md` holds the milestones, what each one demos and
what blocks it; this list is the order in one glance.

1. Money core: centimes, TVA, stamp, amount in words, the first migration,
   `crates/api` with products.
2. A cash sale with a printed 80mm ticket: products, till, the ticket
   series, stock ledger, the user on every row and the audit log, settings
   with the régime fiscal, three languages, daily backup.
3. Customers with a debt ledger and credit limit, and the invoice model
   with every legal field: facture, avoir, proforma, statement.
4. Stock in: suppliers, purchases, expenses, dashboard, Excel.
5. Users and roles.
6. First release: installer, updater, signing, versioned migrations.
7. LAN mode: one desktop serves, phones and second tills pair by QR.
8. Cloud mode, only after Anouar decides (open decision 1).

The sections below keep their original numbering; it names the area, not
the order.

## 1. Inventory baseline (v1)

**Product.** Name, barcode (unique, optional, auto-generated numeric if
blank), category, unit of measure (piece, kg, litre, box), cost price,
selling price, wholesale price (optional), quantity on hand, low-stock
threshold, TVA rate (see open decision 3), active flag. Batches/lots: later.

**Supplier.** Name (unique), phone, address, RC, NIF, NIS, AI, opening debt
(money owed before the software existed), notes.

**Purchase.** Supplier, supplier's document number, date, lines (product,
quantity, unit cost), transport and extra costs, amount paid now, due date
for the rest. Saving a purchase moves stock in and adds the unpaid part to
the supplier's debt. A purchase can be received in parts.

**Sale (till).** Lines (product, quantity, unit price, line discount),
global discount, payment mode (cash; credit on the customer's ledger; card
on a TPE with no integration, decided 2026-09-08), amount tendered,
change. Anonymous sale is allowed; a credit sale requires a customer.
Saving a sale moves stock out and, if credit, adds to the customer's debt.
Every sale is a fiscal document (see §3) even when it is a simple ticket.
The till screen adds one rule the spec does not name: a product whose unit
is piece or box is sold in whole units, so a typed "1,5" is refused on the
line before the sale is posted (kg and litre take it). The core does not
enforce this; a half box is not something a receipt can say, and nothing
else was catching it.

**Credit at the till.** A credit sale names a customer; without one it is
refused on the `customer_id` field, because the ledger is per customer and
money owed by nobody has nowhere to sit. The customer has to be active and
of the shop. A closed fiche is named on no document at all, cash and card
included: closing it says the shop has stopped trading with that customer,
and a paid ticket made out to them would say otherwise. What the customer's credit limit is tested against is the
balance the sale would leave behind, `old_balance + net_to_pay`, not the
basket: a customer 100,00 under their limit cannot buy 200,00 on credit
however small each basket is. A null credit limit is no limit at all and a
zero one is no credit at all, which are two different answers. Passing the
limit refuses the sale with the code `credit_limit`, which carries the
balance after and the limit so the till can say by how much; the refusal
happens before the document number is taken, so a blocked sale burns none.
A null warn threshold is no warning; reaching one is enough to warn, and a
warned sale still goes through, so the answer carries `near_limit` beside
the document rather than refusing it.

The owner can pass the block by sending the sale again with `override`.
That sale is written like any other and an audit row records it
(`sale.credit_override`, with the balance, the limit and the document it
produced). Until M4 there are no roles in the app, so the API accepts an
override from anyone; the log is what carries the accountability in the
meantime.

A cash or a card sale may also name a customer. The document then carries
the buyer block and the balance triple with a `remaining_debt` of nothing,
and no ledger movement is written: nothing is owed. The buyer block is
snapshotted onto every document that names a customer, a ticket included,
so the facture switch has nothing to backfill.

**Stock movements.** Append-only ledger: every change to quantity on hand
is a row with type (purchase, sale, adjustment, return, opening), quantity,
unit cost, reference to the source document, user, timestamp. Quantity on
hand is derived from this ledger and cached on the product; a nightly job
re-derives it and reports drift.

**Expense.** Category (seeded: rent, electricity, water, salaries,
transport, maintenance, other), amount, date, note.

**Dashboard.** Today's and this month's sales, gross margin (sales minus
cost of goods sold from the ledger), expenses, cash position, low-stock
list, top products, outstanding customer debt, outstanding supplier debt.

**Backup.** Automatic daily copy of the SQLite file, keep 30, restore from
the settings screen. Export products, sales, customers, suppliers to Excel;
import products from Excel with a downloadable template.

## 2. Customers and debt (v1)

**Customer.** Name, phone, address, RC, NIF, NIS, AI (all optional for a
private person, required for a company the moment an invoice is issued to
them), credit limit, warning threshold, opening debt, notes.

The stored fiche differs from that list in four places (migration
`2026-09-09-000002_customers`). It carries a `party_kind`, `company` or
`consumer`, and an `active` flag the way a product does. The name is
required, because the buyer block prints it and the list is ordered by it.
The opening debt is not a column: it is the first `opening` row of the debt
ledger, so the balance has one source and correcting it later is an
`adjustment` movement a comptable can read. A null credit limit is no limit
and a zero one is no credit at all; a null warning threshold is no warning.

`party_kind` is a field on the fiche, never inferred from whether an RC was
typed in. Loi 04-02 art. 10 decides ticket against facture by who the buyer
is, and `facture_requires_party_ids` asks a different set of fields of a
company than of a consumer, so an inference would flip the rule the moment
somebody clears a field.

The customers screen opens a company fiche with neither RC nor NIF: the
identifiers are asked for when a facture is issued to it, not to open its
record. It offers no delete (the ledger holds the fiche); a shop that has
stopped dealing with somebody clears `active`. The list shows one tag per
customer, and when two apply the order is over the limit, then no credit at
all (a zero limit), then the warning threshold.

**Debt ledger.** Append-only per party (customer or supplier): document
reference, amount owed added, amount paid, running balance. A payment can
settle several documents oldest-first. The till warns at the threshold and
blocks at the limit; blocking is overridable by a user with the permission.

One row raises the debt or lowers it, never both and never neither, and the
balance is the sum of the table rather than a stored number. It may go below
zero: a customer who overpays is owed money and the statement prints it.

**Statement.** Printable per-party statement for a date range: opening
balance, movements, closing balance.

## 3. Invoice model (v1)

One document type with a `kind`: `ticket` (till receipt), `facture`,
`proforma`, `bon_de_livraison`, `avoir` (credit note), `bon_de_reception`.
All kinds share the same lines and totals; only numbering, legal blocks and
stock effect differ. The column admits a seventh value, `quittance`, the
stamped receipt for a payment on account; nothing issues one until the
comptable says whether such a payment needs its own numbered document (R8),
and the value is admitted now because adding a kind once there are documents
means rebuilding the table.

**Seller block** (from store settings): name, RC, NIF, NIS, AI, address,
phone, fax, email.

**Buyer block** (from the customer at issue time, snapshotted): name, RC,
NIF, NIS, AI, address.

**Lines:** product snapshot (name, barcode), quantity, unit price HT, line
discount, TVA rate, line total HT.

**Totals**, all in centimes:

| Field | Definition |
|---|---|
| `total_ht` | Σ line totals HT after line discounts |
| `discount` | global discount applied to `total_ht` |
| `subtotal_ht` | `total_ht − discount` |
| `tva` | Σ per rate: `round(subtotal_ht_at_rate × rate)` |
| `total_ttc` | `subtotal_ht + tva` |
| `stamp` | droit de timbre, see below |
| `net_to_pay` | `total_ttc + stamp` |
| `amount_in_words` | `net_to_pay` written out in the print language |
| `old_balance`, `remaining_debt`, `total_debt` | from the ledger at issue time |
| `payment_mode` | cash, card, credit in v1; cheque and transfer are parked (the column admits them) |

The balance triple, in the words that decide it. `old_balance` is what the
customer owed before this document. `total_debt` is what they owe after it.
`remaining_debt` is this document's own unpaid part at issue: `net_to_pay`
on a credit sale, zero on a cash or a card one. All three are read from the
ledger when the document is issued and stored on it, so a reprint six
months later prints the balance the customer was handed and never a sum of
today's ledger. A document that names no customer stores none of the three.

`remaining_debt` is the one of the three that moves after the document is
issued, and it moves for both of the things that lower what a customer owes:
a payment, and a correction downwards. Both settle the customer's documents
oldest first, so Σ `remaining_debt` over a customer's issued documents is
never more than the ledger balance, and a document never goes on asking for
an amount the ledger says is no longer owed.

Because the limit is tested on `total_debt` and not on the basket, a
customer whose balance is negative may buy on credit up to what the shop
already holds for them, whatever the limit says. A deposit or an avoir
leaves `old_balance` below zero; a 200,00 basket against a 300,00 credit
balance leaves `total_debt` at -100,00, which is past no limit, not even a
limit of zero. That is the same rule read from the other side, not an
exception to it: what the block asks is what the customer will owe.

**Facture or ticket at the till.** Loi 04-02 du 23 juin 2004 art. 10, as
rewritten whole by loi 10-06 du 15 août 2010 art. 3, decides it, and it
decides it by who the buyer is, not by an amount. There is no threshold in
the text.

- A sale to a consumer is a `ticket`. « Les ventes de biens ou les
  prestations de services faites au consommateur doivent faire l'objet d'un
  ticket de caisse ou d'un bon justifiant la transaction. » That is the
  till's default and it needs nothing from the customer.
- A sale to an agent économique carrying on an activity listed in art. 2
  (production, distribution, services, artisanat, pêche, agriculture) is a
  `facture`, and the buyer is under a matching duty to ask for one.
- Any buyer who asks turns the sale into a `facture`: « Toutefois, la
  facture ou le document en tenant lieu doit être délivré si le client en
  fait la demande » (art. 10 al. 3; décret 05-468 art. 2 repeats it from the
  facture side).
- The document is due « dès la réalisation de la vente », so the choice is
  made at the till, before the sale is saved, and never by a later reprint.

A till cannot tell a consumer from a professional on its own, so the
operator decides: the sale screen carries a one-tap switch from ticket to
facture that pulls in the buyer block. A facture to a consumer needs only
« ses nom, prénom(s) et adresse » (décret 05-468 art. 3-2, last alinéa); a
facture to a trader needs the party identifiers of the row below. The till
sends the choice as `kind` on the sale (`ticket` or `facture`, ticket by
default), and a facture is refused unless both halves carry what the row
below asks: the shop's own RC and NIS from its settings, and from the fiche
either the buyer's RC and NIS or, for a consumer, a name and an address. The
refusal names the side and the identifiers that are missing, and it happens
before a number is taken, so neither series gaps over it. A facture whose
`net_to_pay` is zero — a basket a discount emptied, an exchange settled line
for line — is issued today like any other and takes its number, because no
text read so far says a document has to be worth something; confirm with the
comptable (R8) before a shop leans on it.

Numbering: per kind, gapless, assigned at issue and never reused; a
cancelled facture keeps its number and is marked "facture annulée"; an
avoir is its own kind with its own series. A yearly reset of the series is
common practice but not in the decree; confirm with the comptable (R8)
before it becomes a setting.

### Fiscal rules: current assumptions

Each row names the fixture that pins it and the source that decides it.
"assumption" means nobody has read the law for it yet; a source cites code,
article and edition. Primary sources and findings live in
`research/legal-fiscal/`. **Confirm each with a comptable before the
first release.**

| Rule | Assumption | Fixture | Source |
|---|---|---|---|
| Money representation | integer centimes; no float anywhere in core; rates are integer basis points, 0 to 10 000 (1900 = 19 %), a rate above one whole is refused | `money_no_float`, invalid rates in `tva_rounding_once_per_rate` | design choice, not law |
| Rounding | integer centimes; TVA per rate group on the group's HT subtotal, rounded once, half away from zero, to the centime. A design choice: no text prescribes facture rounding | `tva_rounding_once_per_rate` | CTCA 2026 art. 80bis → CIDTA 2026 art. 324 governs the tax return (base to the lower dinar / ten dinars, duty to the nearest 10 centimes), not the document. Comptable to confirm facture practice |
| Line quantity and line total | a quantity is an integer number of thousandths of the unit (1500 is 1,5 kg), so a product sold by weight or by volume never needs a float. The line gross is `unit_price × qty_milli / 1000` rounded to the centime, half away from zero, once per line and before the line discount is taken off it. An assumption: no text says how a weighed line is rounded. Confirm with the comptable (R8) | `line_total_fractional_qty` | design choice, not law |
| Global discount spread | the global discount is allocated to the rate groups in proportion to each group's HT subtotal; each share is rounded down to the centime and the centimes left over go to the group with the largest HT subtotal, the lower rate winning a tie, never past that group's own HT (what it cannot take rolls to the next largest). The group bases always sum back to `total_ht − discount` and none is negative, so no centime is invented or lost. An assumption: no text says how a global discount splits across rates. Confirm with the comptable (R8) | `discount_spread_largest_remainder` (the `totals_cases` array in `tva_rounding_once_per_rate`) | design choice, not law |
| TVA rates | 19 % standard, 9 % reduced, 0 % exempt; rate per product, defaulted from category | `tva_rates_table` | CTCA 2026 art. 21 (19 %), art. 23 (9 %, list by tariff line) |
| Régime fiscal | shop-level, dated setting `ifu` or `réel`. IFU: single price per product, no TVA rate, no HT/TTC, no TVA line on any document; réel: the TVA rows above. Documents keep the regime they were issued under | `regime_ifu_prints_no_tva` | CIDTA 2026 art. 282 ter (8 M DA threshold), 282 sexies (rates); CTCA 2026 art. 2-12 (out of TVA scope), art. 64 (must not mention TVA) |
| Droit de timbre | cash only (electronic exempt); nothing at 300 DA or less; tranches = ceil(amount / 100 DA); 1 DA per tranche up to 30 000 DA, 1,5 DA up to 100 000 DA, 2 DA above, the whole amount at its band's rate (no progressivity); minimum 5 DA; no cap. The base is `total_ttc`, the amount before the stamp itself: the text says "amount" and names no base, so this is an assumption to confirm with the comptable (R8). Open: half-dinar on odd tranches at 1,5. It is exact in centimes and only matters if the tax must be paid in whole dinars | `stamp_progressive_tranches` (replaces `stamp_cash_only_clamped`) | Code du timbre 2026 art. 100-I and 258 quinquies; DGI circular 14/MF/DGI/LF.2025 (examples 1–3) |
| Amount in words | French, Arabic and English generators, dinars and centimes | `words_{fr,ar,en}_golden` | décret 05-468: total TTC "en chiffres et en lettres"; Arabic wording not yet sourced |
| Printed wording | the words a document prints live in the core, three languages per key. The Arabic is unreviewed by a native speaker, exactly like `words_ar`, and the Arabic goldens say so in their own header comment | `fixtures/print/ticket_80mm/ar.html`, `ar-ifu.html`, `fixtures/print/facture_a4/ar.html`, `ar-cash.html`, `ar-ifu.html` | none yet; R6 covers both this and the amount in words |
| Party identifiers | `facture`: seller RC + NIS (+ NIF, AI as on every facture in circulation), buyer RC + NIS, or name + address when the buyer is a consumer; stamp and signature blocks; `ticket`: seller identity only | `facture_requires_party_ids` | décret 05-468 art. 3 and 4; NIF/AI from tax texts, article to cite (R3) |
| Numbering | one uninterrupted chronological series per document kind; a cancelled document keeps its number and is marked "facture annulée"; numbers never reused | `numbering_gapless` | décret 05-468 art. 10 |

## 4. Printing (v1)

- Templates: `ticket_80mm`, `facture_a4`, `statement_a4`, `barcode_label`.
  Each in ar/fr/en. HTML rendered by the core, not the UI, so desktop and
  any server print the same bytes. `facture_a4` is one template for the
  three kinds that share a facture's blocks: it titles itself facture,
  avoir or proforma, and A5 is a `Paper` argument that changes the `@page`
  size line and nothing else, so there is no separate `facture_a5`,
  `proforma_a4` or `avoir_a4` file to keep in step.
  `bon_de_livraison_a4` is parked with the facture récapitulative (see
  Later); the `kind` stays in the model.
- Golden-file test for every template × language against fixed fixtures.
  A template change is a reviewed golden diff. `ticket_80mm` is done:
  one basket sold three ways, three languages each.
  `fixtures/print/ticket_80mm/{fr,en,ar}.html` is réel and cash,
  `{fr,en,ar}-ifu.html` is the IFU, and `{fr,en,ar}-card.html` is réel and
  card, which has no stamp row and neither half of the change. Pinned by
  `crates/core/tests/print_ticket.rs`. `UPDATE_GOLDENS=1` rewrites them and
  fails the run on purpose, and every amount in a golden is parsed back out
  of the file against the document's stored totals, so a golden that drifts
  from the money cannot be accepted by regenerating it.
  `facture_a4` is done on the same mechanics:
  `fixtures/print/facture_a4/{fr,en,ar}.html` is a réel facture on credit
  to a company, with the balance triple and no droit de timbre;
  `{fr,en,ar}-cash.html` is cash to a consumer, with the stamp and no
  balance block; `{fr,en,ar}-ifu.html` is the IFU. Pinned by
  `crates/core/tests/print_facture.rs`, which also reads back the TVA rate
  of every recap row and of every line, the balance triple, the escaping of
  a product name that carries markup, and the words line against
  `amount_in_words` of the stored net.
- The print language is the language the till is being used in, passed by
  the caller on each call (`GET /sales/{id}/ticket?lang=fr|en|ar`). There is
  no separate print-language setting in v1.
- Numbers are Western digits in every language, comma decimal, thousands
  grouped with a narrow no-break space (U+202F), and no currency word on a
  line: `fixtures/money/format_centimes.json` pins the core's formatter and
  the desktop's to each other. A document's printed number is
  `{prefix}-{number:06}`, `TK-000123` for a ticket.
- The words a document prints are the core's own dictionary
  (`crates/core/src/print/strings.rs`), not the desktop's i18n JSON: a
  server with no UI prints the same paper. The Arabic in it is unreviewed by
  a native speaker, like `words_ar` (R6), and the Arabic goldens of both
  templates say so in their own header comment
  (`fixtures/print/ticket_80mm/ar*.html`,
  `fixtures/print/facture_a4/ar*.html`).
- Under the IFU a printed document has no TVA recap, no rate on a line and
  no "HT" on its total row: it must not mention the tax at all
  (`regime_ifu_prints_no_tva`), and "hors taxe" names one. The facture
  drops its "Total TTC" row for the same reason, and prints "Prix unitaire"
  where the réel one prints "Prix unitaire HT".
- Thermal: ESC/POS over USB or Bluetooth from the desktop; from the phone
  via the desktop in LAN mode. "Any printer" means the OS print dialog for
  A4/A5 and raw ESC/POS for 80mm.
- Later: a QR code on the ticket, the shop's logo, and a footer text the
  owner sets. None of the three is in M1.

## 5. Users and roles (v1)

Owner, manager, cashier. Login by PIN on the till, password elsewhere.
Permissions: sell, give discount above X %, override credit block, see cost
prices and margins, edit products, edit settings, see reports. Every
document records the user. Audit log of sensitive actions (price change,
discount override, delete, settings change), an ISO-27001 control we get
for nearly free by writing it now. An owner user exists from the first
migration, so every document, ledger row and audit entry carries a user
from the first sale (build-order step 2); PIN, roles and permissions
arrive in step 5.

## 6. LAN mode (v1, after the desktop milestones)

Exactly one desktop is the server; it advertises via mDNS and shows a QR
(host, port, short-lived pairing token). A phone or a second till scans it
and is a client from then on. Clients never own stock; a client that loses
the server queues writes and replays them, server answer wins. Windows
Firewall is the known trap: detect the blocked listener on first run and
show one instruction.

## 7. Cloud mode (open)

Same core binary hosted, one SQLite file per shop, account login. Not
started until decision 1. Nothing built before it may assume it does not exist:
every query is scoped by `shop_id`, every client talks HTTP.

## Later, agreed

- Batches/lots with expiry, product variants, bundles, promotions.
- AI invoice scanning on the phone (the one modern thing the competitor has).
- Phone-only offline via the Rust core compiled into the RN app (uniffi),
  never a TypeScript reimplementation of the calculations.
- Cheque and transfer as payment modes (decided 2026-09-08: v1 is cash,
  credit, card on a TPE).
- Bon de livraison and facture récapitulative together (décret 05-468
  art. 14–17 allow the first only with the second and a wilaya
  authorisation). The `bon_de_livraison` kind exists in the model; no
  template or screen until then.

## Open decisions

1. **SaaS with an account, offline licence, or both.** Anouar. Changes
   pricing, hosting and step 8 only.
2. **Product name.** Anouar; "Dinar" proposed and the org `Dinar-dz` created
   on 2026-09-08. The name changes the bundle identifier once, before the
   first release.
3. ~~TVA per product or one global rate.~~ **Decided 2026-09-08 (Samir):
   per product, defaulted from the category.** CTCA art. 23 lists the 9 %
   goods by customs tariff line, so the rate is a property of the product.
4. **A real printed facture** from any Algerian shop, to check the field
   list and layout against paper before the golden files are frozen.
