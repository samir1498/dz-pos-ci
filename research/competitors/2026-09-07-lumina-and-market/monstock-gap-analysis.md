# MonStock ↔ Lumina POS — gap analysis

Sources: `github.com/samir1498/MonStock` @ HEAD (8,394 LOC Rust + TSX, 3
migrations) and the extracted Lumina desktop source at `/home/samir/lumina/src`.
Feature claims about Lumina come from its shipped code and templates, not from
the marketing video.

## What MonStock already has

Products (name, barcode, cost, selling price, qty), suppliers, purchase orders
with line items and receive-into-stock, sales as `transactions` +
`transaction_items` carrying both cost and selling price per line, expenses with
categories, a dashboard with profit/loss, barcodes, end-of-day, daily automatic
backups, EN/FR i18n (106 keys), dark/light theme, pagination everywhere. Two
frontends over one GUI-agnostic core, plus a CLI.

That is a working single-shop inventory and sales app. The core is sound and
the layering — `monstock-core` as models/repos/services, GUI on top — is the
right shape to build on.

## The gap, in order of how much it costs to close

### 1. There is no fiscal layer at all

MonStock has no invoice concept. No `invoices` table, no document rendering, no
TVA, no stamp duty, no fiscal identifiers on anyone. `grep -i facture|invoice|
tva|tax` across the repo returns nothing but a stray comment.

Lumina ships the whole thing: RC/NIF/NIS/AI on the store and on every party,
global TVA rate, droit de timbre (1%, floor 5 DZD, ceiling 2500 DZD, cash only),
amount in words, old-balance/remaining-debt ledger, and 50+ print templates —
eleven invoice layouts, thermal 80mm, credit notes, delivery notes, reception
notes, proformas, recap invoices, payment receipts, each in ar/en/fr.

This is the single biggest gap and the reason the market pays them.

### 2. There is no customer

`customers` does not exist in the schema. Sales are anonymous: a transaction has
a timestamp and a total, nothing else. Lumina has customers with credit limits,
warning thresholds, a debt ledger and sales history — and "تتبع الديون" (debt
tracking) is in their headline pitch. Algerian retail runs on credit; a POS that
cannot say who owes what is not sellable there.

Suppliers have the same hole: MonStock's supplier is name/phone/notes. Lumina's
carries four fiscal identifiers plus an opening-debt field for money already owed
before the software was installed.

### 3. There is no user model

No `users` table, no roles, no auth, no PIN. Single implicit operator. Lumina has
users with roles and per-user permissions, and PIN unlock on mobile. Needed the
moment a shop has a cashier who is not the owner.

### 4. No printing

`print` in MonStock is `println!`. No ESC/POS, no thermal, no page layout.
Lumina prints to thermal and A4/A5, and "supports all printers" is one of its
four selling points.

### 5. No multi-device

MonStock is one binary against one SQLite file. Lumina offers single device /
LAN server on :8080 with QR-paired phones / cloud / hybrid.

### 6. Product model is one tier deep

MonStock: name, barcode, cost, price, quantity. Lumina: unit of measure,
wholesale price, suggested retail, margin %, variants and colours with
per-variant pricing, batch/lot tracking, low-stock alerts.

### 7. No Arabic

MonStock ships EN/FR. No `ar.json`, and egui does not do RTL out of the box —
this is a real engineering cost in the desktop frontend, not a translation task.
The Tauri frontend would handle RTL far more cheaply.

## What neither has

- Per-product TVA rates. Lumina's is one global percentage.
- Anything resembling e-invoicing or DGI submission.
- Self-serve purchase. Lumina activation is a form and a phone call.

## What Lumina has that is worth stealing as an idea

**AI invoice scanning.** Photograph a supplier invoice, extract the products and
the supplier automatically ("Extrayez automatiquement les produits et le
fournisseur depuis la photo"). Quota-metered on their side. It is the only
genuinely modern thing in the product, it is mobile-only, and it directly
attacks the worst job in a shop — typing in a delivery. Worth taking seriously.

## Quality gates — where MonStock stands against what Anouar asked for

One test file, `monstock-core/tests/repo_tests.rs`, 26 `#[test]` functions
covering repos only. No service tests, no UI tests, no coverage gate, no Sonar,
no CI quality workflow. If the brief is "start with Sonarqube, high coverage,
deterministic verification", this is close to a standing start — but a
standing start on 8k lines of clean code is still much better than an empty
repo.

## Read

Roughly: MonStock is the inventory half done well, and the commercial and fiscal
half not started. The missing pieces are not exotic — customers with debt, an
invoice with the right fields on it, users, and printing — and Lumina's own
templates are now a specification for the hardest of them.

Not a rewrite. An extension, in this order: customers + debt → invoice data
model → print engine → users/roles → Arabic/RTL → multi-device.
