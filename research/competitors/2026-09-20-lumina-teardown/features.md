# Lumina POS — feature list by screen, with their own wording

One line per feature. Every line is marked:

- **[driven]** — seen working on screen in this pass
- **[read]** — read out of the running app's markup or its database, not exercised
- **[unreached]** — known to exist, could not be opened; blockers.md says why

Wording is quoted as the app shows it, French first because that is the column
we need to match, then Arabic. Nothing here is copied from their source files;
these are labels read off the running UI.

## Navigation, the whole sidebar

| French | Arabic | Note |
|---|---|---|
| Point de Vente | شاشة المبيعات | the till **[driven]** |
| Journal des Ventes | سجل المبيعات | **[driven]** |
| Caisse | الصندوق | cash-register shifts **[read]** |
| Stock | إدارة المخزون | **[driven]** |
| Packs | الحزم | bundles **[read]** |
| Promotions | التخفيضات والعروض | **[read]** |
| Fournisseurs | إدارة الموردين | **[driven]** |
| Achats | مشتريات وفواتير | purchases **[read]** |
| Clients | إدارة الزبائن | **[driven]** |
| Utilisateurs | إدارة المستخدمين | **[read]** |
| Pro-formas | الفواتير المبدئية | **[read]** |
| Bons de commande | طلبيات التوصيل | delivery orders **[read]** |
| Statistiques | الإحصائيات والتقارير | **[driven]** |
| Paramètres | الإعدادات | **[driven]** |

The section headers group them as Ventes / Stock & produits / Tiers & comptes /
Finance & rapports / Système (المبيعات / المخزون والمنتجات / الأطراف والحسابات /
المالية والتقارير / النظام). **[driven]**

## First run and account

- Owner account setup before anything else: full name, username, password (four
  characters minimum), confirm. Same form on desktop and phone. **[driven]**
- The account settings tab changes username and password, asking for the current
  password. **[read]**
- Trial state is a badge in the top bar, "Version d'Essai" / "نسخة تجريبية", with
  a live countdown of remaining trial sales. Desktop started at 100. **[driven]**
- Activation: a hardware id plus a code in a four-by-four format, and a request
  form asking name or shop, phone, wilaya and commune. There is a standing
  discount banner with a countdown. **[read]**

## Point de Vente — the till

- Product search by name, and a separate barcode field that takes a scanner or
  typed input. **[driven]**
- Product grid with category filter and an advanced filter. **[driven]**
- Cart line shows the product, unit price, the **batch it is drawing from**, and
  quantity steppers. **[driven]**
- Summary bar: Sous-total, "Remise / Majoration" (discount *or* markup, one
  control), Total Dû. **[driven]**
- Customer widget on the cart to attach a regular customer to the sale, searching
  by name or id. **[read]**
- Actions, all on F-keys: Payer (F1), Suspendre (F2), Brouillons (F3), Annuler
  (F4), Impression (F5), plus turn-into-pro-forma and turn-into-order. **[driven]**
- Checkout modal: payment method and amount paid. Methods are Espèces (Cash),
  Chèque bancaire, Chèque postal (CCP), Virement bancaire, Carte bancaire,
  Autre. **[driven]**
- A dedicated "Crédit total" button that books the whole sale to the customer's
  debt and prints a receipt rather than an invoice. **[read]**
- Confirming gives "Confirmer Vente & Facture". A cash sale for 250.00 DA was
  rung up and landed with the right subtotal, total, paid amount, method and a
  zero stamp. **[driven]**
- Off-catalogue line: a custom product can be added to the cart with a name and
  a cost, and the sale line keeps that name. **[read]**
- Recent-sales and sale-details modals off the till. **[read]**

## Stock — products

- Product form in three tabs: "Infos de base", "Propriétés & tarifs",
  "Variantes & couleurs". **[driven]**
- Basic tab: barcode (left blank it is generated — the generated one was twelve
  digits beginning with the year), name, category, family, supplier, minimum
  stock, location, description, image. **[driven]**
- Pricing: opening stock, unit cost, markup %, selling price, an optional
  wholesale price, and an expiry date. Saving a plain product **created a batch
  row** to carry the stock and the prices; the product row itself holds no
  price. **[driven]**
- Unit list: pièce, unité, carton, boîte, sachet, paquet, bouteille, kilogramme,
  gramme, **quintal**, litre, millilitre, mètre, centimètre. **[driven]**
- Weight/volume, colour, PLU code, wholesale unit and how many units in it. **[read]**
- A has-variants switch that opens the variants tab; bulk fields set cost, price,
  wholesale and stock across generated variants at once. **[read]**
- Batches have their own modal: a display code, supplier, variant, stock, cost,
  markup, price, optional wholesale price and an expiry date. **[read]**
- Per-product analytics modal. **[read]**
- Barcode-label printing from the product, from a variant, or in bulk, with the
  label's width and height in mm, barcode height (or auto), and switches for
  shop name, product name, price, price type, barcode digits, variants and
  discount. **[read]**

## Packs and Promotions

- Pack: name, optional barcode, description, a product search to add members,
  a cost price and a selling price for the pack as a whole. **[read]**
- Promotion: a discount type and value against a product, with start and end
  dates, a maximum quantity and a used-quantity counter, and an active flag. **[read]**

## Clients

- Customer form: name, phone, address, then **NIF, RC, AI, NIS**, then an opening
  debt and a credit limit. Only the name appeared to be required — a customer
  saved with all four identifiers filled. **[driven]**
- Customer details modal with a debt ledger and a payment submission. **[read]**
- The list screen is headed "Clients Réguliers". **[driven]**

## Fournisseurs and Achats

- Supplier form: name, phone, address, RC, NIF, AI, NIS, and an initial debt for
  money already owed when the software arrives. **[read]**
- Supplier details, a payment against supplier debt, and a supplier return with
  its own line items. **[read]**
- Purchase invoice against a supplier, with line items, and its own tables for
  the invoice and the lines. **[read]**

## Caisse — the register

- Two tabs, "La session en cours" and the shift history. **[read]**
- Open a shift with an opening balance, close it with a closing balance and a
  note. **[read]**
- Cash in and cash out during the shift, each with an amount and a category;
  the seeded categories are rent, electricity, water, salaries, transport,
  maintenance, other, and the shop can add its own. **[read]**
- Shift history filtered by date range and by employee. **[read]**

## Statistiques — eight report tabs

Résumé Financier · Statistiques avancées · Dettes & Soldes · Analyses du Stock ·
Dépenses · Mouvements de Caisse · Graphiques · **Calculateur de Zakat**. The tab
strip was seen; only the default tab was opened. **[read, tab list]**

- Net profit and cash flow, and a most-profitable-products table with columns
  produit, catégorie, qté vendue, revenus, coût, bénéfice. **[driven]**
- Advanced statistics filter by date range, scope, family, category and a
  multi-product picker, with a compare toggle. **[read]**
- Zakat calculator: cash on hand, other debts, and the nisab. The tab was never
  opened; the three inputs were read off the app's own markup. **[read]**
- Capital injections, categorised own capital / partner / loan / reinvestment /
  other, with amount, date and a note. **[read]**
- A recap invoice over a date range with a payment method. **[read]**

## Paramètres

Eight tabs: Compte · Paramètres généraux · Factures · Réseau · Import/Export ·
Activation de l'Application · Application Mobile · Mises à jour. Six of the
eight were opened; "Application Mobile" and "Mises à jour" were only seen as
buttons. **[driven, six of eight]**

### Paramètres généraux
- **Language selector: Arabic / English / French**, switching the whole UI and
  flipping RTL to LTR. **[driven]**
- Shop info: name, description, address, mobile, landline, email, **RC, NIF, AI,
  NIS**, a logo, and one shop-wide "TVA %". **[driven]**
- Expense categories, editable. **[read]**
- Switches: quick-sale confirmation skip, financial-accounting-only mode, allow
  negative stock, ask before selling into negative stock, average-cost pricing
  instead of batch-by-batch, and **enable droit de timbre — off by default**. **[driven]**

### Factures
- Invoice template picker and receipt template picker. **[read]**
- **Invoice print language and receipt print language chosen separately from the
  UI language**, each Arabic / French / English. **[driven]**
- Show barcode on the receipt. **[read]**
- Direct silent printing, and three printer choices from the OS printer list: a
  default, a POS-receipt printer and a barcode-label printer. **[driven]**
- Cash drawer enable. **[read]**
- Barcode-label geometry and contents. **[driven]**
- **Electronic scale settings.** **[read]**

### Réseau
Four modes — single device, several devices on the same network (LAN), over the
internet (Cloud), combined server plus Cloud — plus a mobile-pairing toggle, a
connection test, and a cloud join code in a prefixed four-by-four format. The app
runs its HTTP server on port 8080 even in single-device mode. **[driven, mode list]**
- The mobile-pairing toggle is below the four mode cards, off by default; turning
  it on reveals a QR code for the phone to scan. The phone's matching "link via
  QR" option is worded as going over the internet — the pairing path Lumina ships
  is relay/cloud-mediated, not a LAN handshake. The phone also offers a separate
  LAN auto-search option, not yet confirmed to avoid the relay. **[driven]**

### Import/Export
Full system backup to ZIP and restore · Excel product export · Excel product
import with a downloadable template · Excel exports of the sales journal, the
customer list and the supplier list · a danger zone with a factory reset. **[driven, list]**

## Android app

- Opens in **Arabic** with an Arabic/French toggle. **[driven]**
- First screen offers three ways to run: straight on the phone with no computer
  and no internet (their recommended one), pair with a desktop over LAN or a
  relay, or cloud mode for several devices. **[driven]**
- The phone-only mode is pitched as a complete stock-and-sales system with no
  computer, and states its own trial: **50 invoices and 10 customers**, then a
  one-time activation code. **[driven]**
- Owner account setup identical in shape to the desktop's. **[driven]**
- Past account creation: a "connect to Lumina" screen offering manual IP, LAN
  auto-search and QR-via-cloud connection modes, reached via an unplanned path.
  Everything else past account creation. **[unreached — host CPU contention on
  the shared box stopped further driving; not a Lumina limit, see blockers.md]**
- Barcode scanning, Bluetooth thermal printing, biometric unlock and the AI
  invoice scan were read in the September pass and not re-checked here. **[unreached]**

## Roles

- Add-user form offers five roles: Seller (بائع), Cashier (كاشير), Accountant
  (محاسب), Assistant (مساعد), Admin (مدير عام). **[driven]**
- A non-Admin role (tried: Seller) has the Utilisateurs sidebar entry hidden
  and is refused with a toast when the router is asked to navigate there
  directly. **[driven]**
- The refusal is router-only: the same session's in-renderer functions and
  `window.api.*` calls (`loadUsersGrid`, `getUsers`, `getSettings`,
  `getCustomers`) return full data with no permission check, and calling the
  Admin-only add-user function directly creates a real Admin account while
  still logged in and displayed as a Seller. **[driven]**
- Cashier, Accountant and Assistant specifically were not created or logged in
  as. **[unreached]**
