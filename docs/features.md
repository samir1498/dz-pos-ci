# Features — what dz-pos does

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
starts.

1. Products, suppliers, purchases into stock, sales, expenses, daily backup,
   dashboard — the inventory baseline.
2. Customers with a debt ledger and credit limit.
3. Invoice model: the fiscal document with every legal field.
4. Print engine: thermal 80mm and A4/A5, three languages, golden-file tested.
5. Users and roles.
6. LAN mode: one desktop serves, phones and second tills pair by QR.
7. Cloud mode — only after Anouar decides (open decision 1).

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
global discount, payment mode (cash, card, cheque, transfer, credit), amount
tendered, change. Anonymous sale is allowed; a credit sale requires a
customer. Saving a sale moves stock out and, if credit, adds to the
customer's debt. Every sale is a fiscal document (see §3) even when it is a
simple ticket.

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

**Debt ledger.** Append-only per party (customer or supplier): document
reference, amount owed added, amount paid, running balance. A payment can
settle several documents oldest-first. The till warns at the threshold and
blocks at the limit; blocking is overridable by a user with the permission.

**Statement.** Printable per-party statement for a date range: opening
balance, movements, closing balance.

## 3. Invoice model (v1)

One document type with a `kind`: `ticket` (till receipt), `facture`,
`proforma`, `bon_de_livraison`, `avoir` (credit note), `bon_de_reception`.
All kinds share the same lines and totals; only numbering, legal blocks and
stock effect differ.

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
| `payment_mode` | cash, card, cheque, transfer, credit |

Numbering: per kind, per year, gapless, assigned at issue and never reused;
a cancelled facture gets an avoir, it is not deleted.

### Fiscal rules — current assumptions

Each row names the fixture that pins it and the source that decides it.
"assumption" means nobody has read the law for it yet; a source cites code,
article and edition. Primary sources and findings live in
`research/legal-fiscal/`. **Confirm each with a comptable before the
first release.**

| Rule | Assumption | Fixture | Source |
|---|---|---|---|
| Money representation | integer centimes; no float anywhere in core | `money_no_float` | design choice, not law |
| Rounding | half away from zero, applied once per TVA rate on the subtotal, not per line | `tva_rounding_once_per_rate` | assumption; CTCA 2026 art. 80bis points to CIDTA art. 324, not yet read |
| TVA rates | 19 % standard, 9 % reduced, 0 % exempt; rate per product, defaulted from category | `tva_rates_table` | CTCA 2026 art. 21 (19 %), art. 23 (9 %, list by tariff line) |
| Droit de timbre | **superseded, do not implement:** `clamp(net × 1 %, 5, 2 500)` was Lumina's pre-2025 rule. Current: per tranche of 100 DA or fraction, 1 DA up to 30 000, 1,5 DA to 100 000, 2 DA above, minimum 5 DA, amounts of 300 DA or less out of scope; electronic payments exempt. Marginal vs whole-amount banding and the 300 DA edge are open | `stamp_progressive_tranches` (replaces `stamp_cash_only_clamped`) | Code du timbre 2026 art. 100-I, art. 258 quinquies (LF 2025) |
| Amount in words | French, Arabic and English generators, dinars and centimes | `words_{fr,ar,en}_golden` | décret 05-468: total TTC "en chiffres et en lettres"; Arabic wording not yet sourced |
| Party identifiers | RC, NIF, NIS, AI printed for both parties on `facture`; optional on `ticket` | `facture_requires_party_ids` | décret 05-468 (FAQ list read; decree text pending); ticket rules in loi 04-02 pending |
| Numbering | gapless per kind per year | `numbering_gapless` | assumption; "numéro d'ordre" in 05-468, which text requires gapless is task R5 |

## 4. Printing (v1)

- Templates: `ticket_80mm`, `facture_a4`, `facture_a5`, `proforma_a4`,
  `bon_de_livraison_a4`, `avoir_a4`, `statement_a4`, `barcode_label`. Each
  in ar/fr/en. HTML rendered by the core, not the UI, so desktop and any
  server print the same bytes.
- Golden-file test for every template × language against fixed fixtures.
  A template change is a reviewed golden diff.
- Thermal: ESC/POS over USB or Bluetooth from the desktop; from the phone
  via the desktop in LAN mode. "Any printer" means the OS print dialog for
  A4/A5 and raw ESC/POS for 80mm.

## 5. Users and roles (v1)

Owner, manager, cashier. Login by PIN on the till, password elsewhere.
Permissions: sell, give discount above X %, override credit block, see cost
prices and margins, edit products, edit settings, see reports. Every
document records the user. Audit log of sensitive actions (price change,
discount override, delete, settings change) — an ISO-27001 control we get
for nearly free by writing it now.

## 6. LAN mode (v1, after 1–5)

Exactly one desktop is the server; it advertises via mDNS and shows a QR
(host, port, short-lived pairing token). A phone or a second till scans it
and is a client from then on. Clients never own stock; a client that loses
the server queues writes and replays them, server answer wins. Windows
Firewall is the known trap: detect the blocked listener on first run and
show one instruction.

## 7. Cloud mode (open)

Same core binary hosted, one SQLite file per shop, account login. Not
started until decision 1. Nothing in 1–6 may assume it does not exist:
every query is scoped by `shop_id`, every client talks HTTP.

## Later, agreed

- Batches/lots with expiry, product variants, bundles, promotions.
- AI invoice scanning on the phone (the one modern thing the competitor has).
- Phone-only offline via the Rust core compiled into the RN app (uniffi),
  never a TypeScript reimplementation of the calculations.

## Open decisions

1. **SaaS with an account, offline licence, or both.** Anouar. Changes
   pricing, hosting and step 7 only.
2. **Product name and GitHub org.** Anouar; name research continues. Changes
   the bundle identifier once, before the first release.
3. **TVA per product or one global rate.** Recommendation: per product with
   a global default — the competitor's single global rate is a known
   complaint and the cost of a column now is nothing.
4. **A real printed facture** from any Algerian shop, to check the field
   list and layout against paper before the golden files are frozen.
