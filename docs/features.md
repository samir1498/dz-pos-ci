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
8. Cloud mode — only after Anouar decides (open decision 1).

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
| `payment_mode` | cash, card, credit in v1; cheque and transfer are parked (the column admits them) |

Numbering: per kind, gapless, assigned at issue and never reused; a
cancelled facture keeps its number and is marked "facture annulée"; an
avoir is its own kind with its own series. A yearly reset of the series is
common practice but not in the decree; confirm with the comptable (R8)
before it becomes a setting.

### Fiscal rules — current assumptions

Each row names the fixture that pins it and the source that decides it.
"assumption" means nobody has read the law for it yet; a source cites code,
article and edition. Primary sources and findings live in
`research/legal-fiscal/`. **Confirm each with a comptable before the
first release.**

| Rule | Assumption | Fixture | Source |
|---|---|---|---|
| Money representation | integer centimes; no float anywhere in core; rates are integer basis points, 0 to 10 000 (1900 = 19 %), a rate above one whole is refused | `money_no_float`, invalid rates in `tva_rounding_once_per_rate` | design choice, not law |
| Rounding | integer centimes; TVA per rate group on the group's HT subtotal, rounded once, half away from zero, to the centime. A design choice: no text prescribes facture rounding | `tva_rounding_once_per_rate` | CTCA 2026 art. 80bis → CIDTA 2026 art. 324 governs the tax return (base to the lower dinar / ten dinars, duty to the nearest 10 centimes), not the document. Comptable to confirm facture practice |
| TVA rates | 19 % standard, 9 % reduced, 0 % exempt; rate per product, defaulted from category | `tva_rates_table` | CTCA 2026 art. 21 (19 %), art. 23 (9 %, list by tariff line) |
| Régime fiscal | shop-level, dated setting `ifu` or `réel`. IFU: single price per product, no TVA rate, no HT/TTC, no TVA line on any document; réel: the TVA rows above. Documents keep the regime they were issued under | `regime_ifu_prints_no_tva` | CIDTA 2026 art. 282 ter (8 M DA threshold), 282 sexies (rates); CTCA 2026 art. 2-12 (out of TVA scope), art. 64 (must not mention TVA) |
| Droit de timbre | cash only (electronic exempt); nothing at 300 DA or less; tranches = ceil(amount / 100 DA); 1 DA per tranche up to 30 000 DA, 1,5 DA up to 100 000 DA, 2 DA above, the whole amount at its band's rate (no progressivity); minimum 5 DA; no cap. Open: half-dinar on odd tranches at 1,5 | `stamp_progressive_tranches` (replaces `stamp_cash_only_clamped`) | Code du timbre 2026 art. 100-I and 258 quinquies; DGI circular 14/MF/DGI/LF.2025 (examples 1–3) |
| Amount in words | French, Arabic and English generators, dinars and centimes | `words_{fr,ar,en}_golden` | décret 05-468: total TTC "en chiffres et en lettres"; Arabic wording not yet sourced |
| Party identifiers | `facture`: seller RC + NIS (+ NIF, AI as on every facture in circulation), buyer RC + NIS, or name + address when the buyer is a consumer; stamp and signature blocks; `ticket`: seller identity only | `facture_requires_party_ids` | décret 05-468 art. 3 and 4; NIF/AI from tax texts, article to cite (R3) |
| Numbering | one uninterrupted chronological series per document kind; a cancelled document keeps its number and is marked "facture annulée"; numbers never reused | `numbering_gapless` | décret 05-468 art. 10 |

## 4. Printing (v1)

- Templates: `ticket_80mm`, `facture_a4`, `facture_a5`, `proforma_a4`,
  `avoir_a4`, `statement_a4`, `barcode_label`. Each in ar/fr/en. HTML
  rendered by the core, not the UI, so desktop and any server print the
  same bytes. `bon_de_livraison_a4` is parked with the facture
  récapitulative (see Later); the `kind` stays in the model.
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
