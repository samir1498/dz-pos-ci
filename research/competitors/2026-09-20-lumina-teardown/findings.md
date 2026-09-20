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

All 40 regenerable desktop screenshots (14 sidebar sections times two
languages, plus 6 settings sub-tabs times two languages) were retaken this
pass under the same filenames in `screens/`, confirmed modal-free by
inspection before the rest were taken, and now show the actual screen named
in each filename. Treat the retake as the first real evidence those filenames
carry, not as duplicate work — the first pass produced no usable images at
all.

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
