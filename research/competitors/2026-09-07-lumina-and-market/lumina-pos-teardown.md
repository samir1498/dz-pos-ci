# Lumina POS — competitor feature map

Reverse-engineered from three artefacts in the vendor's Drive folder: a 4m25s
screen recording, the Android APK, and the Windows installer. The installer was
unpacked (NSIS → `app-64.7z` → `resources/app.asar`), not executed. The
extracted source is the authority for anything below that the video could not
show.

Working files: `/home/samir/lumina` — `frames/`, `sheets/`, `shot_*.jpg`,
`apk/`, `src/` (extracted asar).

## Vendor

| | |
|---|---|
| Product | Lumina POS (لومينا) |
| Company | FikraDevs — `fikradevs.com`, package `com.fikradevs.lumina` |
| Author credit in UI | BOUKERDOUNE MOHAMMED — LUMINA POS |
| Base | عين البيضاء، أم البواقي (Aïn Beïda, Oum El Bouaghi) |
| Contact | lumina@fikradevs.com · 0775151284 · support 09:00–18:00 |
| Version | 1.5.1 in the UI; installer names itself 1.0.0, built 2026-07-27 |
| Stack | Electron desktop (obfuscated JS, sensitive modules compiled to `.jsc` bytecode) + Expo/React Native mobile |

The recording was made on `C:\Users\anoum\…` — Anouar's own machine, not the
vendor's.

## Commercials

- **12,000 DZD one-time**, struck through from 15,000 DZD, countdown timer on
  the activation screen. Roughly €83.
- Explicitly lifetime licence, no subscription: "تدفع مرة واحدة فقط وتمتلك
  البرنامج بالكامل".
- Trial capped at **100 sales**, counter in the top bar.
- Activation is manual: the in-app form collects name, phone, wilaya and
  commune, and a human sends back a code. No card, no self-serve.
- Claims: fastest in Algeria, free updates for life, works offline, supports
  all printers.

## The fiscal layer — this is the useful part

`src/invoice_template_1.html` is the printed facture, and its placeholders are
a complete field list for a compliant Algerian invoice:

**Seller block:** `COMPANY_NAME`, `COMPANY_RC`, `COMPANY_NIF`, `COMPANY_NIS`,
`COMPANY_AI`, `COMPANY_ADDRESS`, `COMPANY_PHONE`, `COMPANY_FAX`,
`COMPANY_EMAIL`

**Buyer block:** `PARTY_NAME`, `PARTY_RC`, `PARTY_NIF`, `PARTY_NIS`,
`PARTY_AI`, `PARTY_ADDRESS`

**Totals:** `TOTAL_HT`, `TVA`, `TAX_TOTAL`, `DISCOUNT`, `SUBTOTAL`,
`STAMP_AMOUNT`, `NET_TO_PAY`, `TOTAL_IN_WORDS`

**Ledger:** `OLD_BALANCE`, `REMAINING_DEBT`, `TOTAL_DEBT`, `PAYMENT_MODE`,
plus signature and stamp blocks.

### Droit de timbre, as they implement it

From `renderer.js`:

```
POS_STAMP_RATE = 0.01        // 1%
POS_STAMP_MIN  = 5           // DZD
POS_STAMP_MAX  = 2500        // DZD
stamp = payment_mode === 'cash' ? max(5, min(2500, amount * 0.01)) : 0
```

Toggleable per store. Non-cash payments get no stamp. Confirm the rate and caps
with a comptable before copying them — this is their reading of the rule, not a
citation.

TVA is a **global percentage** set once in Settings → store info
(`نسبة الضريبة العالمية (TVA %)`), not per-product.

Both the store record and every supplier and customer carry all four
identifiers: RC (السجل التجاري), NIF (الرقم الجبائي), NIS (رقم الإحصاء),
AI (المادة الجبائية). Confirmed on screen for store and supplier.

## Document templates shipped

Far more than the video suggests — 50+ HTML templates in `src/`:

- `invoice_template_1` … `_9`, plus `_a5` and `_detailed`
- `invoice_thermal_80.html` — 80mm thermal receipts
- `receipt_template_1..5` in Arabic, English and French
- `credit_note`, `delivery_note` (A4 / A5 / thermal), `reception_note`,
  `proforma_invoice`, `recap_invoice`, `payment_receipt`, `order_receipt`,
  `sales_summary` — each in ar / en / fr
- `barcode_label_template_1.html`

## Language — corrects the first read

The desktop UI in the recording is Arabic throughout. The **mobile app is
entirely French**: "Convertir en facture", "Ajouter un nouveau fournisseur",
"Dettes fournisseurs", "Taux TVA (%)", "Aucune facture correspondante".

There is a `settings.print_language` key — "Langue d'impression des factures" —
so the printed document language is configurable independently of the UI. The
three-language template set backs that up. Arabic-first is a choice, not a
limitation.

## Navigation (RTL sidebar, right-hand side)

| Group | Item | Arabic |
|---|---|---|
| — | Sales log | سجل المبيعات |
| — | Cash register / till | الصندوق |
| Stock & products | Stock | المخزون |
| | Bundles | الحزم |
| | Promotions | العروض الترويجية |
| Parties & accounts | Suppliers | الموردين |
| | Purchases | المشتريات |
| | Customers | الزبائن |
| | Users | المستخدمين |
| Finance & reports | Proforma invoices | الفواتير المبدئية |
| | Delivery orders | طلبات التوصيل |
| | Statistics | الإحصائيات |
| System | Settings | الإعدادات |

## Data model

**Product** — three tabs: basic data, attributes & pricing, variants & colours.
Auto-generated numeric barcode, category, unit of measure, opening stock with
unit cost, wholesale price, suggested retail price, profit margin %,
per-variant pricing. Batch/lot tracking is real — `/api/batches/` plus
"Ajouter un lot" and "Aucun lot disponible" in the mobile bundle.

**Supplier** — name, phone, address, the four fiscal identifiers, and an
opening-debt field for money already owed before the software was installed.

**Customer** — name, phone, address, credit limit with a warning threshold,
debt ledger, sales history.

**Purchase invoice** — supplier, Bon number, receipt date, line items carrying
quantity / unit purchase price / margin % / suggested sale price / wholesale
price / subtotal, then TVA, transport & extras, cash paid now, and a debt due
date. Saving updates stock automatically.

**Expense categories** — seeded: rent, electricity, water, salaries, transport,
maintenance, other.

## Deployment topology

Settings → الشبكة offers four modes:

1. **Single device** — isolated local database.
2. **LAN** — one machine runs a server on `:8080`, other tills are clients,
   phones join by QR. Auto-discovery plus manual server IP.
3. **Cloud** — sync between branches.
4. **Hybrid** — LAN server *and* cloud sync.

The banner promises data stays local by default: "لا تُرسل بياناتك التجارية إلى
الإنترنت في الوضع المحلي الاعتيادي".

## Mobile app

Expo / React Native, `newArchEnabled`, expo-router. `forcesRTL: true`.

- ML Kit barcode scanning, Bluetooth ESC/POS thermal printing, biometric
  unlock, Firebase messaging, SheetJS for Excel.
- Pairs with the desktop till via `/api/pos/pair` and `/api/devices/register`.
- **AI invoice scanning** — "Scanner facture (IA)", "Extrayez automatiquement
  les produits et le fournisseur depuis la photo". Quota-metered via
  `/api/invoices/scan/quota`. This never appears in the desktop recording and
  is the most advanced thing in the product.

API routes from the JS bundle:

```
/api/analytics   /api/batches/   /api/capital/    /api/categories
/api/cloud/sync/pull   /api/cloud/sync/push       /api/dashboard
/api/customers/  /api/expenses/  /api/imports/excel/quota
/api/inventory-alerts  /api/invoices/scan/quota   /api/mobile/activate
/api/ping   /api/pos/pair   /api/products/   /api/proformas/
/api/purchases/  /api/relay/  /api/sales/  /api/settings
/api/suppliers/  /api/users/edit  /api/variants/
```

`/api/cloud/sync/wiper-wash-alert` also appears — a car-wash route in a POS
bundle. FikraDevs reuses one backend template across products, so some of this
surface is not bespoke to Lumina.

## Backup and data portability

Full system backup to ZIP with restore, Excel import for products
(merge-or-replace, downloadable template), Excel export for products, sales
log, customers and suppliers, and a red danger zone with a full system reset.

## Read on the competitor

- It is a real, finished product. The video looks AI-built because it uses the
  standard Tailwind/shadcn aesthetic, but the forms are wired, the network
  stack is real, and the mobile app ships ML Kit and ESC/POS.
- The fiscal layer is genuinely done: four identifiers on every party, stamp
  duty with caps, amount in words, TVA, and eleven invoice layouts across three
  languages. This is the part that takes longest to build and it is finished.
- Distribution is a phone number and a wilaya. The moat is the vendor's local
  presence in Oum El Bouaghi, not the software.
- Weakest points to probe: TVA is a single global rate rather than per-product,
  activation is fully manual, and the desktop UI ships Arabic only while the
  mobile app ships French only.
