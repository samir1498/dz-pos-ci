# Lumina POS, driven end to end — 2026-09-20

Second pass on Lumina. The 2026-09-07 pass read their shipped files; this one
runs both builds and clicks them. Its job is to check the earlier claims, and
it overturns two of them.

## What ran, and how

| | |
|---|---|
| Desktop | `Lumina POS.exe`, the already-unpacked Electron build, run in place from a staging folder on the Windows side. **Driven end to end.** |
| Android | `lumina.apk`, package `com.fikradevs.lumina`, versionName 1.0.0, versionCode 1, minSdk 24, targetSdk 36, native libs for arm64-v8a / armeabi-v7a / x86 / x86_64. Installed on the existing `dinar` AVD (API 34, google_apis, x86_64). **Onboarding driven; the rest not reached — see blockers.md.** |
| Version disagreement | The APK badge says 1.0.0, the desktop installer names itself 1.0.0, the desktop UI reported 1.5.1 in the 2026-09-07 recording. Treat the marketing version and the build version as unrelated. |

Nothing was downloaded and nothing was installed system-wide. Both artefacts
were already on the box from the September pass. Free space on `C:` was 48 GB
before the run, above the 20 GB gate.

### How the desktop was driven, since it matters for repeatability

The unpacked Electron build accepts Chromium flags, so it was started with a
remote-debugging port. WSL cannot reach a Windows-side loopback port — the same
firewall fact `scripts/maestro-windows.sh` already records for adb — so the
driver runs on the Windows node install and talks to the port from that side.
From there the app is fully scriptable: it exposes a global navigation function
that switches screens and another that switches UI language, so every screen
could be visited, screenshotted and have its fields enumerated.

It is a single-page app: every screen's markup is in one document, 924 elements
carry ids and 293 of them are form fields. That is why `hierarchy/` can hold a
complete field inventory rather than a sample.

On startup the app prints its own database path and starts an HTTP server on
`0.0.0.0:8080` even in single-device mode, with the mobile pairing relay off
and cloud sync skipped. Its store is a WAL-mode SQLite file with **39 tables**.

## Claims from 2026-09-07 and 2026-09-17: what this pass did to them

### Contradicted

**"Their desktop UI ships Arabic only."** Wrong. Settings has a language
selector offering Arabic, English and French. Switching to French retitles
every screen and flips the whole layout from RTL to LTR. Both language walks
are in `hierarchy/` and both sets of screenshots in `screens/`.

**"Their mobile app is entirely French."** Wrong, and backwards. The Android
app opens in Arabic and carries an Arabic/French toggle at the top of the
onboarding screens. Arabic is its default, not French.

Together these kill the "Three languages everywhere" entry in
`context/references/20260917-dinar-against-lumina-what-is-missing-and-what-is-already-ahe.md`
under "Where Dinar is already ahead". We are level on UI languages, not ahead.

**"Trial capped at 100 sales."** True of the desktop only; its top bar counts
down remaining trial sales and started at 100. The Android app states a
different cap on its own first screen: 50 invoices and 10 customers, then a
one-time activation code. Two products, two trial rules.

### Confirmed by driving

- Four fiscal identifiers (RC, NIF, NIS, AI) on the shop record and on every
  customer and supplier. A customer was created with all four and they landed
  in the customer row.
- Supplier opening debt exists on their form, which is what the withdrawn gap 7
  was originally about. Ours exists too, so the withdrawal stands.
- TVA is a single global percentage. Confirmed twice: the settings field is one
  shop-wide rate, and the sale row carries one tax-percentage column for the
  whole sale, with no tax column on the line item. Per-product TVA remains a
  real advantage for us.
- Stamp duty is a per-shop toggle and it is **off by default**. The sale rung up
  in this pass recorded a stamp of zero for that reason. The rate itself was not
  re-derived here; the 2026-09-07 reading of it stands unchecked, and the
  research README already notes their reading is out of date against the 2025
  finance law.
- Four network modes, worded on the desktop as single device / several devices
  on the same network (LAN) / over the internet (Cloud) / combined server plus
  Cloud. The Android app offers the same choice in three entries.
- Product form in three tabs, the third being variants and colours.
- Batches are real and not optional: saving a plain product silently created a
  batch row to hold its stock and its prices. The product row carries no price.
- Activation is a manual form asking name/shop, phone, wilaya and commune,
  against a hardware id, with the code in a four-by-four format. No self-serve.

### Could not reach

Anything behind activation, any cloud or multi-branch behaviour, the AI invoice
scan, and all Android screens past onboarding. See blockers.md.

## The first pass's desktop screenshots caught a purchase modal, not the app

Every one of the 42 screenshots taken earlier in this pass turned out to show
the same "Rejoignez plus de 500 commerçants" upsell modal sitting on top of the
app (element ids beginning `popupBuy`), not the screen it claimed to show —
`d-productsSection-fr.png`, `d-settings-overview-fr.png` and
`d-salesSection-fr.png` were pixel-identical, and the same was true across the
set. The `hierarchy/` DOM walks are unaffected and are the real evidence behind
every claim above; only the images were wrong.

Root cause: `Page.captureScreenshot` over CDP returned a stale, frozen frame
from the moment the modal first appeared, independent of any later DOM
change — clicking through screens changed the underlying document (the
`hierarchy/` walks prove that) but the composited frame Chromium handed back
never updated. This looked at first like a wrong close-button selector, but
inspecting the modal's actual markup showed the close attempt
(`button[class*="w-10 h-10 rounded-xl"]`) never matched an element in the
modal at all — it silently reported "clicked" while clicking nothing, so the
modal was never dismissed and the renderer never got a chance to repaint. The
real close control is `button[class*="top-5"][class*="left-5"][class*="z-50"]`
(top-left in the French/LTR layout, top-right once the layout flips to
Arabic/RTL). Clicking it removes the modal from the DOM. The frozen-frame
symptom itself only cleared after killing the Electron process by pid and
relaunching it fresh with the remote-debugging flag; no CDP-level command
(`Emulation.setFocusEmulationEnabled`, `Page.setWebLifecycleState`, forcing a
resize) unstuck the existing process once it had latched onto the stale frame.

The retake covered 40 desktop screenshots (14 sidebar sections times two
languages, plus 6 settings sub-tabs times two languages), under the same
filenames in `screens/`.

**Four of them are still the modal**, found on 2026-09-20 while building the
report pages: `d-settings-overview-fr.png`, `d-settings-overview-ar.png`,
`d-product-form-filled.png` and `d-customer-form-filled.png`. The
"confirmed modal-free by inspection" claim above the fix was written from a
sample, not from all 40. The check that found them is mechanical and takes
seconds, and is the one to use next time: `compare -metric RMSE <shot>
flow.png null:` over every file, then sort. The four modal shots score under
13 000 against `flow.png`; every real screen scores above 29 000, so the two
groups do not overlap and no judgement call is involved. All 58 files are
byte-distinct, so a hash check would not have caught it.

The other 54 are real, distinct screens and are the evidence behind the
claims here. The report site carries 55 images: those 54 plus `flow.png`
itself, which is the modal and is captioned as the modal. The three
mislabelled files were not published; `d-settings-overview-ar.png` was
published under the honest name `modal-upsell-ar.png`, because the Arabic
layout of their purchase banner is worth having.

## Roles: enforced only in the renderer, never at the data layer

The users screen offers **five roles**, not the three Dinar has: Seller
(بائع), Cashier (كاشير), Accountant (محاسب), Assistant (مساعد) and Admin
(مدير عام), picked from a `userRole` `<select>` on the add-user form
(`screens/roles-01-add-user-form.png`, `screens/roles-02-add-user-filled.png`).
A second user was created with the Seller role, logged in as that user
(`screens/roles-03-logged-in-as-seller.png`), and driven directly.

What the Seller role visibly refuses: the sidebar hides the Utilisateurs
entry, and navigating to it directly (calling the app's own router function,
`navigate('usersSection')`) is refused with a toast saying the account lacks
permission (`screens/roles-04-seller-blocked-from-users-toast.png`). That
refusal is real but it is the only layer of refusal there is.

What it does not refuse: the app exposes its business logic as plain
in-renderer JavaScript functions and an IPC bridge (`window.api.*`) that carry
no permission check of their own. As the same logged-in Seller, calling
`loadUsersGrid()` and `window.api.getUsers()` / `getSettings()` /
`getCustomers()` directly returned full data with no error — the router
blocks the menu link, not the underlying calls. Going one step further,
calling the app's own `openUserModal()` function directly (the function the
Admin-only "add user" button wires up) and submitting it while still logged
in and displayed as a Seller **created a brand-new Admin account**
("QA Admin" / "qaadmin", role Admin) that could then log in with full access
(`screens/roles-05-seller-created-admin-account-bypass.png`). Nothing server-
side stopped it because there is no server-side in the access-control sense —
the desktop app is the only party enforcing anything, and it only enforces at
the router.

This is the answer to the question this pass most cared about. Compared with
`crates/api/src/gates.rs` and `crates/core/src/services/permissions.rs`:

- Dinar has 3 roles (`Owner`, `Manager`, `Cashier`) against 13 named
  `Permission`s, checked by a pure function (`permissions::can`). Lumina has
  5 roles with no equivalent permission table found in the driven UI — the
  distinctions between Cashier/Accountant/Assistant were not individually
  probed this pass (see "Not driven" below), only Seller was.
- Every mutating dz-pos route, plus 18 named reading routes, is gated
  server-side in `gates.rs` via `MatchedPath`, independent of the client:
  a scripted or compromised frontend cannot get more than an honest one can,
  because the HTTP API itself refuses the request. Lumina has no equivalent
  gate on `window.api.*`: the guard lives entirely in the renderer's
  `navigate()` call and the sidebar's visibility logic, both of which are
  trivially bypassed from the same process that is already running the UI
  (no separate exploit needed — just calling an existing global function with
  the console open, or from injected script).
- Net: Lumina's role model is **broader on paper** (5 named roles vs. 3) but
  **weaker in practice** — it is a UI convenience, not an access control, for
  every account below Admin. Dinar's 3-role, 13-permission, server-checked
  model is narrower in vocabulary but is the only one of the two that would
  survive a dishonest or scripted client.

Not driven this pass, for the record: Cashier, Accountant and Assistant were
never individually created or logged in as, so whether the sidebar hides
different things per role (as opposed to one blanket non-Admin hiding) is
unconfirmed — the bypass above was demonstrated through the Seller account
specifically, but nothing about the IPC layer suggests the other three would
behave differently, since the same ungated `window.api` surface backs all
of them.

## Pairing needs Lumina's cloud, not just LAN reachability

The desktop's mobile-pairing control is not on the Réseau tab's main view —
it is below the four network-mode cards, reached by scrolling
(`screens/d-settings-network-scrolled-ar.png`), and its own label is
"اقتران تطبيق الهاتف" (pair the phone app). Turning it on and saving reveals a
QR code (`screens/d-settings-network-pairing-qr-ar.png`). The phone side
offers the matching entry point: a "ربط عبر QR" (link via QR) option on its
own connect screen, described in the app's own copy as going over the
internet, alongside "بحث تلقائي" (auto-search, local network) and "اتصال
يدوي" (manual connection, by IP).

This overturns the assumption written into blockers.md before this pass —
that because the desktop listens on `0.0.0.0:8080` and the emulator can reach
the Windows host's address, "the link to a computer" mode could be tested
without a second machine or a network change. The QR-pairing path Lumina
actually ships is relay-mediated through their own cloud infrastructure by
its own labelling, which puts full pairing behind the same paying-customer
wall as activation and cloud sync, and out of reach under the hard rule
against touching their cloud. The "بحث تلقائي" LAN auto-search option is the
one path that might not need their relay, since it is worded as local-network
discovery rather than internet-relayed; it was not attempted this pass (the
emulator was CPU-starved by the time it was found — see blockers.md) and
remains the one part of pairing worth trying next, ahead of the QR path.

## Things nobody had recorded

- **A zakat calculator**, as a tab of the reports screen, taking cash on hand,
  other debts and the nisab. A deliberately Algerian, deliberately
  Muslim-market feature, and it is in none of our notes.
- **Capital injections** as a first-class record, categorised as own capital,
  partner, loan, reinvestment or other.
- **CCP as a payment method**, alongside cash, bank cheque, bank transfer, card
  and other. Algérie Poste's CCP being on the list is exactly the kind of local
  fit Anouar is pointing at.
- **An electronic-scale integration** and a cash-drawer toggle in settings.
- **Three printers configured separately** — a default, a POS-receipt one and a
  barcode-label one — plus a silent direct-print mode that skips the OS dialog.
- **A unit list built for this market**, including quintal alongside kg, g,
  litre, and the packaging units a grocery uses.
- **A financial-accounting-only mode** and a **negative-stock mode** (sell before
  the goods are entered), each a shop-wide switch, with a separate switch for
  whether selling into negative stock asks for confirmation.
- **Average-cost pricing as an alternative to batch-by-batch**, as a toggle.
- Keyboard shortcuts on the POS screen: pay, suspend, drafts, cancel and print
  are all on F-keys.

## What their schema holds

39 tables. The shape worth knowing:

- Sales carry the customer, subtotal, one tax percentage, discount, total, paid
  amount, due date, stamp amount and payment method **on the sale row**. Credit
  notes are not a separate document: the same sale row carries the credit-note
  number, its date, the original invoice number and date, and a reason.
- Sale lines carry batch and variant, a cost price captured at sale time, the
  promotion applied, a custom product name for off-catalogue items, and a flag
  for having been sold without stock.
- Payments against a sale are their own table with method, reference, receipt
  number and a printed-at timestamp, so part payments and debt collection are
  modelled properly.
- Products hold no price and no stock. Both live on batches, alongside expiry
  and location. Products do hold family, PLU code, wholesale unit and wholesale
  quantity, a minimum stock, and a has-variants flag.
- Separate tables for bundles and their items, promotions, product option groups
  and values, variants and their option values, proformas and their items,
  purchases and their items, supplier returns and their items, customer orders
  and their items, cash-register sessions and their transactions, expenses,
  capital injections, inventory reservations, a print log, an edit history, and
  a document counter keyed by document type and year with a prefix format.
- Every business table carries a uuid, an updated-at, a soft-delete timestamp,
  an origin device, a server revision and a dirty flag, feeding a sync outbox.
  Their cloud story is built into the schema, not bolted on.

One oddity: the customer form collects an opening debt and a credit limit, but
the customers table has neither column. Where those two values go was not
established in this pass.

## Reports and exports they ship

The reports screen has eight tabs: financial summary, advanced statistics, debts
and balances, stock analysis, expenses, cash movements, charts, and the zakat
calculator. Import/export offers a full system backup to ZIP with restore, an
Excel product export and import with a downloadable template, Excel exports of
the sales journal, the customer list and the supplier list, and a factory reset
behind a danger zone.

Printing settings let the shop choose the invoice template and the receipt
template, and set the invoice print language and the receipt print language
**independently of the UI language**, each across Arabic, French and English.

## Side effects on this machine, for cleanup

- Lumina is **still installed** on the `dinar` AVD as `com.fikradevs.lumina`.
  Left there on purpose: the next pass drives it with Maestro and tests pairing
  against the desktop. Removing it is one `pm uninstall`. The emulator was not
  killed or wiped — it is Samir's, with Expo Go on it.
- The desktop run created a store under the Windows profile at
  `AppData\Roaming\lumina-pos`, holding the shop, product, customer and sale
  created during this pass.
- About 450 MB staged in a `lumina-run` folder on the Windows profile, plus the
  driver scripts written there.
- **The desktop app is still running**, with its window on Samir's screen and its
  HTTP server on `0.0.0.0:8080`, pairing relay off. Left running on purpose: the
  pairing test in blockers.md needs it, and it is a competitor's server on a
  wildcard bind, so it should be closed once nobody is reading it. Closing it is
  a `taskkill` on its pid, never a `pkill -f`.
  To bring it back: set the execute bit on the copied `.exe` first (a file copied
  onto the Windows filesystem is not executable), then start it with a
  remote-debugging port and drive it from the Windows-side Node, because WSL
  cannot reach a Windows loopback port.

## The Android pass, 2026-09-20

Picked up exactly where the last pass stopped: owner account created, French
selected, a product form filled but never saved. The adb bridge was tested
first, per the environment brief — `cmd.exe /c "adb devices"` from
`/mnt/c/Users/Anwender` returned `emulator-5554 device` on the first try, so
that hazard did not recur this pass. Host load was low throughout (load
average under 1 most of the session), so the CPU-contention freeze from the
previous pass never showed up either. All screenshots below are in
`shots/`; every one was opened and read before being named or described here.

### 1. The unsaved product actually saves

The stuck field was the on-screen quantity keypad, not the system IME — key
111 (ESC) does not dismiss it, only tapping the keypad's own checkmark does.
Once dismissed, "Enregistrer le Produit" saved the product and navigated to
the stock list on its own; the screenshot taken immediately after the tap
still showed the form (a rendering-lag race, same family as the desktop's
frozen-frame bug, just recovered by waiting a beat and re-shooting).
`android-fr-product-list.png` shows "Cafe 250g" in the stock list, barcode
2033948137956, 250,00 DA, 50 pièce, "En stock" — proof the save wrote a real
row, not just a form reset.

### 2. A cash sale end to end

Tapping the product in the POS tab added it straight to a slide-up cart
without needing a till session open (`android-fr-sale-cart.png`). The
walk-in flow is deliberately short: "Confirmer le paiement" opens a
"Confirmation de la vente" modal for "Client Passager" with no payment-method
choice, just subtotal/total and one "Confirmer & Payer" button
(`android-fr-sale-confirm-modal.png`). The sale posted immediately, appears
in the Ventes tab as "Sale #1 · 250,00 DA · Payé" (`android-fr-sales-log.png`),
and opening it shows a real receipt view — "Reçu N° #1", edit/delete
actions, and four print targets (Ticket, Facture, Facture ticket, Livraison)
— with no tax line, consistent with the earlier finding that TVA is a single
shop-wide rate (`android-fr-sale-receipt.png`).

### 3. A customer, created and attached to a sale

Clients → "Ajouter le premier client" opens a form with Nom, Téléphone,
Adresse, NIF, RC, AI, NIS, **Dette initiale** and **Plafond de crédit**
(`android-fr-customer-form-filled.png`) — confirms the earlier note that the
form collects an opening debt and credit limit the `customers` table has no
column for. Saved as "Samir Client" / 0555123456
(`android-fr-customer-list.png`). Back in the POS tab, "+ Client" opens a
"Choisir un client" picker that lists it (`android-fr-sale-choose-customer.png`);
picking it replaces the "+ Client" pill with a customer chip that survives
adding the product to the cart (`android-fr-sale-cart-with-customer.png`).
With a real customer attached, the payment step is a different, richer modal
than the walk-in one: a method row (Espèces, Chèque, Virement, Carte, Autre),
a "Montant payé" field, and quick buttons for Complet / ½ / **Tout en
dette** — a full credit sale — where the walk-in flow only offered
Confirmer & Payer (`android-fr-sale-payment-modal.png`). The resulting
"Sale #2" shows "Samir Client" as the buyer in the sales log
(`android-fr-sales-log-with-customer.png`), and stock ticked down one unit
per sale (50 → 49 → 48), confirmed against the batch shown in the POS list
each time.

Also captured by an accidental tap while the "Nom" keyboard was still up on
the customer form: the "Mise à niveau" pill opens a full activation-request
page ("Commander maintenant") asking name, phone, wilaya and commune, with
copy promising a human will call back with an activation code
(`android-fr-upgrade-activation-request.png`). Nothing was entered or
submitted on it, per the standing rule against asking Lumina for a code;
Android's hardware back popped it cleanly back to the customer form with the
typed name still in place, confirming it is a pushed route and not a
form-owning modal.

### 4. Reports

The "Plus" tab is the home screen's four-tile grid moved into a scrollable
menu, grouped under Documents / Rapports / Opérations / Gestion Financière /
Paramètres / Données / Compte; scrolling it end to end took three
screenshots (`android-fr-plus-menu.png`, `android-fr-plus-menu-2.png`,
`android-fr-plus-menu-3.png`). The home dashboard itself — "Travail
quotidien" (Vente rapide, Vente, Facture proforma, Ventes) and "Stock &
achats" tiles — is in `android-fr-home-dashboard.png`; an earlier, identical
capture of the same screen reached by a mistaken tap during the customer
form is `android-fr-dashboard.png`, kept because it is genuine, not a
duplicate frame.

The Android reports surface is much thinner than the desktop's eight tabs:
Plus → Rapports has exactly two entries, Statistiques and Calculateur Zakat.
Statistiques is one long scrolling page, captured in four screenshots as it
scrolled (`android-fr-reports-statistiques.png`,
`android-fr-reports-statistiques-2.png`,
`android-fr-reports-statistiques-3.png`,
`android-fr-reports-statistiques-4.png`): Revenus et bénéfices (revenu net,
coût des ventes, bénéfice brut, panier moyen, bénéfice net final), Trésorerie
(cash in/out, capital investi, dépenses, solde de trésorerie disponible),
Analyse des dettes (créances clients, dettes fournisseurs, each with a
"Détails" list that read "Aucune dette" since none exist yet), Stock (stock
au prix d'achat, stock au prix de vente estimé) and a closing "Bénéfice
latent" card projecting the profit if all stock sold. Every figure matched
what the two sales and the one unsold-stock line should produce (500,00 DA
revenue, 12 000,00 DA stock at sale price). Calculateur Zakat
(`android-fr-reports-zakat.png`) auto-fills stock value, créances clients
and dettes fournisseurs from the same data and asks for liquidités
disponibles, autres dettes and the day's nisab value by hand — matches the
desktop finding exactly, now with the actual screen instead of just the
menu label.

### 5. Settings

Plus → Paramètres Principaux is one very long page; scrolled and captured in
ten shots (`android-fr-settings-general.png` through
`android-fr-settings-general-10.png`). Worth recording specifically:
appearance (Jour/Nuit/Auto); interface language and **invoice/receipt print
language set independently of it** (interface was French, print language was
still Arabic — matches the desktop); a "RTL: Activé" label sitting next to
the network header even while the interface was French and the layout was
plainly LTR (a leftover status string, not a real state — see the Arabic
pass below for what RTL actually does here); network fields (IP, port 8080,
Appairage anti-mélange **Désactivé**); user info showing the account role as
"مدير عام" in Arabic script even with the whole rest of the screen in
French; a "Mode Comptabilité Financière uniquement" switch (disables stock
tracking) matching the desktop's financial-only mode; ticket printing with
58 mm / 80 mm / Auto / custom-mm width and a **Bluetooth direct-print**
toggle whose own description says "L'arabe est imprimé en image, donc
parfait sur toutes les imprimantes" — a concrete, previously-unrecorded
detail about how they solve Arabic-on-ESC/POS printers; a pricing-method
toggle for average-cost pricing, noted in-app as **not synced in cloud
mode**; "Activer la Licence" and backup/restore (export ZIP explicitly
offered "à envoyer (WhatsApp / Drive)"); and a red "Zone dangereuse" with
"Réinitialisation usine" at the very bottom, which was not tapped.

Two settings screens live one level down and were opened separately:
Gestion des Utilisateurs shows the single admin account, "Dinar" / `@qa` /
مدير عام, with pause and delete actions and an "Ajouter un Utilisateur"
button (`android-fr-settings-users.png`) — a second Seller/Cashier/etc.
account was not created this pass, so whether the Android sidebar hides
things per-role the way the desktop does is still unconfirmed on phone.
Paramètres de Sécurité offers session-lock-on-close (off), biometric unlock
(refused with "Aucune empreinte enregistrée sur cet appareil" since the AVD
has none enrolled) and a PIN quick-unlock, with a note that the PIN is
per-device and clears on logout (`android-fr-settings-security.png`).

### 6. Scanner and caisse (till)

The barcode scanner needed an OS camera-permission grant the first time it
was opened; the dialog was "Allow Lumina to take pictures and record video?"
(`android-fr-camera-permission-dialog.png`), granted "While using the app".
The scanner screen itself opened correctly afterwards — title "Scanner le
code-barres", a Une fois/Multiple toggle, and copy explaining the product is
added automatically on a hit — but the preview area stayed black
(`android-fr-scanner-barcode.png`): the `dinar` AVD has no virtual camera
configured, so this is an emulator limitation, not something to hold against
the app; the UI itself is real and was reached.

The till is a separate flow from the sale itself: two sales were rung up
earlier in this pass with **no caisse session open at all**, which the
Gestion de la Caisse screen confirms in its resting state
(`android-fr-caisse-no-session.png`, "Aucune session ouverte" — Lumina does
not gate selling behind an open till). Opening one asks only for a Solde
d'ouverture (`android-fr-caisse-open-session-modal.png`) and produces a live
dashboard — session #1, caissier "Dinar", opening balance, ventes espèces,
dépenses + sorties, solde actuel, three quick actions (Entrée de fonds /
Sortie de fonds / Dépense de caisse), a movements log, and history
(`android-fr-caisse-session-active.png`). Once a session is open, the POS
screen's "Ouvrir une session caisse" button is replaced by a live
"#1 · 0,00" badge. Closing asks for a Solde réel à la fermeture, computes the
variance against the theoretical balance, and flagged "✓ Caisse équilibrée"
before confirming — this pass dismissed that modal with its X rather than
confirming, to leave the session open (`android-fr-caisse-close-session-modal.png`).

### 7. Arabic pass: layout is not one thing

Switched the interface language to Arabic from Paramètres Principaux
(`android-ar-settings-general.png`) and repeated a short walk: home, product
list, one sale screen, and this same settings screen. The header immediately
labelled itself "الاتصال والشبكة (RTL: مفعل)" and every string on every
screen visited was in Arabic — no leftover French or English label was found
anywhere in this short pass, which is a stronger result than the "RTL:
Activé" ghost label under French suggested.

What is genuinely inconsistent is *how much* actually mirrors:

- The **home dashboard** mirrors text alignment throughout, but the bottom
  tab bar keeps the exact same left-to-right icon order as French (home is
  still the leftmost icon, "المزيد"/More still the rightmost) — only the
  active-tab label under "الرئيسية" changed, not its position
  (`android-ar-home-dashboard.png`).
- The **Stock/product list** card layout does mirror: the bag icon stays on
  the left but the quantity badge ("48 قطعة") and price move to read
  right-to-left as a group (`android-ar-product-list.png`).
- The **POS product list**, which looks like the same card, does **not**
  mirror — bag icon left, quantity badge left, price right, pixel-identical
  positions to the French version, only the text inside is Arabic
  (`android-ar-pos-list.png`). Two near-identical list screens in the same
  app handle RTL differently.
- The **cart and confirm-sale modal** mirror fully and correctly: the close
  "×" moves from left to right, the cart icon/count moves to the leading
  (right) edge, and "الإجمالي" (total) is right-aligned where "Total" was
  left-aligned in French (`android-ar-sale-cart.png`,
  `android-ar-sale-confirm-modal.png`).
- One numeral inconsistency: the confirm-sale modal's subtitle read "زبون
  عابر · ١ منتج" — an Arabic-Indic "١" for the item count — while every
  price and quantity on the same screen and everywhere else in the Arabic
  UI used Western digits (0,00, 48, 250,00). This is the one string in the
  whole Arabic pass that did not follow the numeral convention the rest of
  the app settled on; visible in `android-ar-sale-confirm-modal.png`.

A third sale was rung up in Arabic the same way as the French ones (walk-in,
no till session gate, immediate posting) purely to reach the confirm modal
for the numeral check above; no new customer or product was created in this
part of the pass.

### What this pass did not reach, and why

- The scanner's camera **preview** (not the screen) could not be shown to
  produce a picture, because the `dinar` AVD has no virtual camera backing
  it — an emulator configuration gap, not a Lumina gap.
- A second Android user account (Cashier/Accountant/Assistant) was not
  created, so whether the phone's sidebar hides things per-role like the
  desktop does remains unconfirmed on Android specifically.
- Cloud sync, multi-branch, the AI invoice scan and anything behind
  activation were not attempted, per the standing rule against touching
  Lumina's paid tier or their cloud.
- LAN pairing ("بحث تلقائي") was not attempted this pass either; it stays
  the next concrete step noted in blockers.md.

### State left on the emulator

The `dinar` AVD was left with: the interface language set to **Arabic**
(not reverted to French); a caisse session (**#1**) still open with a
0,00 DA opening balance; three completed cash sales (#1 walk-in, #2 to
"Samir Client", #3 walk-in in Arabic) all against the one product, "Cafe
250g", whose stock is now 47 pièce; one customer, "Samir Client"; camera
permission granted to Lumina. Nothing was submitted to Lumina's activation
or cloud endpoints, and the factory-reset control was not touched.
