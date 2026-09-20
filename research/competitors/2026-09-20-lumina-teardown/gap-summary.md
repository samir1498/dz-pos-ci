# Gap summary: Lumina POS against Dinar

Feature-by-feature standing as of the 2026-09-20 teardown (desktop driven end
to end, Android driven through onboarding, a cash sale, a credit sale and the
report and settings screens). Every Lumina claim below names a screenshot or
a `findings.md` heading; every Dinar claim names a line or a rule in
`docs/features.md`. A claim with no citation is marked "not checked" rather
than assumed.

Per `research/README.md`: this describes what each product does, not how
either is built. Nothing here is a suggestion to copy Lumina's code.

## 1. What Lumina does that Dinar does not

1. **Till sessions with an opening float, running cash movements and a
   balance check at close.** Opening a shift asks for a starting balance,
   shows a live dashboard of cash sales and cash out during the shift, and
   closing it computes the difference against what the shift should hold
   before confirming. Evidence: `android-fr-caisse-open-session-modal.png`,
   `android-fr-caisse-session-active.png`,
   `android-fr-caisse-close-session-modal.png`. Dinar's dashboard says the
   opposite is true today: "there is no opening float and no count at close"
   and cash handed back over the counter is not recorded at all
   (`docs/features.md` §1 Inventory baseline, Dashboard, the cash position
   paragraph).

2. **Invoice and receipt print language set once, separately from the
   interface language, and kept.** Confirmed on both products: the desktop's
   settings screen and the phone's settings screen each let a shop choose
   Arabic, French or English for what prints, independent of what the screen
   itself is showing. Evidence: features list, "### Factures" ("Invoice print
   language and receipt print language chosen separately from the UI
   language"); findings.md, Android pass §5 Settings ("interface was French,
   print language was still Arabic"). Dinar's v1 has no stored setting for
   this: the print language is whatever language the till is open in at the
   moment of printing, passed on the call (`docs/features.md` §4 Printing:
   "There is no separate print-language setting in v1"). Worth flagging to
   Samir: the Scope section states the two should be independently
   selectable; the v1 mechanism in §4 does not yet do that.

3. **Batches as the default, not an option.** Saving a plain product
   silently opens a batch behind it to hold the stock and the price; the
   product row itself carries neither. Evidence: findings.md, "Confirmed by
   driving." Dinar parks this outright: "Batches/lots: later"
   (`docs/features.md` §1, Product paragraph, and "Later, agreed").

4. **An off-catalogue line at the till.** A cashier can add an item to the
   cart with just a typed name and a cost, no product record needed.
   Evidence: features list, "Point de Vente." Dinar's Sale (till) description
   covers only lines against a catalogue product (`docs/features.md` §1,
   Sale (till) paragraph).

5. **Negative stock as a shop-wide choice, with its own confirmation
   switch.** One toggle allows selling before stock is entered; a second,
   separate toggle decides whether that sale asks for confirmation first.
   Evidence: findings.md, "Things nobody had recorded." No equivalent switch
   appears anywhere in `docs/features.md`.

6. **Suspend and draft a sale in progress.** Pay, suspend, drafts, cancel and
   print are all bound to F-keys at the till. Evidence: features list, "Point
   de Vente," Actions bullet. Dinar's till section describes completing or
   refusing a sale; there is no suspended or drafted state.

7. **CCP as a checkout payment method.** Algérie Poste's postal cheque sits
   alongside cash, bank cheque, bank transfer and card on the payment
   screen. Evidence: features list, "Point de Vente," checkout modal bullet.
   Dinar's v1 payment modes are cash, credit and card; cheque and transfer
   are named as parked for later, and CCP is not named at all
   (`docs/features.md` §3, `payment_mode` row of the totals table; "Later,
   agreed").

8. **A zakat calculator on the reports screen**, pre-filled from the shop's
   own stock value, receivables and payables. Evidence:
   `android-fr-reports-zakat.png`; findings.md, "Things nobody had recorded."
   No such screen or figure exists in `docs/features.md`.

9. **Capital injections as their own record**, categorised as own capital,
   partner, loan, reinvestment or other. Evidence: findings.md, "Things
   nobody had recorded"; features list, "Statistiques" section. Not present
   in `docs/features.md`.

10. **An electronic-scale integration and a cash-drawer toggle**, both in
    settings. Evidence: features list, "### Factures." Neither appears in
    `docs/features.md`.

11. **Three printers configured separately** (a default, a receipt printer
    and a barcode-label printer), a silent direct-print mode that skips the
    OS dialog, and Arabic sent to the receipt printer as a picture so it
    renders correctly on a cheap thermal head. Evidence: features list, "###
    Factures"; findings.md, Android pass §5 Settings. Dinar's thermal path
    sends text through a fixed codepage table and its own spec names the gap
    Lumina's picture trick solves: "a cheap head will need a different
    codepage" (`docs/features.md` §4 Printing, Thermal paragraph). No
    per-purpose printer selection is described there.

12. **A wider unit list for this market**, quintal included alongside
    kilogram, gram, litre and the packaging units a grocery uses. Evidence:
    features list, "Stock, products," unit list bullet. Dinar's v1 list is
    "piece, kg, litre, box" (`docs/features.md` §1, Product paragraph).

## 2. What Dinar does that Lumina does not

1. **A customer's opening debt has exactly one place it lives**: the first
   row of an append-only ledger, so the balance is always the sum of that
   table and correcting it later is itself a logged ledger movement
   (`docs/features.md` §2, Customer paragraph). Lumina's own customer form
   asks for an opening debt and a credit limit, but its customers table has
   no column for either, confirmed twice, once on desktop and once on
   Android. Evidence: findings.md, "One oddity," and the Android section
   ("confirms the earlier note that the form collects an opening debt and
   credit limit the `customers` table has no column for"),
   `android-fr-customer-form-filled.png`.

2. **An issued document is never edited and never deleted.** It can only be
   annulled, with a reason written to the audit log, and it keeps its number
   (`docs/features.md` §3, Cancellation paragraph). Lumina's Android receipt
   view shows edit and delete actions on an already-posted sale; the buttons
   were seen on screen, not pressed. Evidence: `android-fr-sale-receipt.png`.

3. **A dedicated printed slip for a customer's account balance**, stating in
   all three languages that it carries no fiscal value, and printing the
   ledger's real balance rather than the sum of the movements shown on the
   page (`docs/features.md` §2, "Debt slip" paragraph). Lumina's equivalent
   is a customer details modal with a debt ledger and a payment field, an
   on-screen view rather than a document of its own. Evidence: features
   list, "Clients" section.

## 3. Same ground, different execution

1. **TVA.** Dinar rates a product individually, 19%, 9% or 0%, defaulted
   from its category, matching how the tax code lists the 9% goods by
   tariff line (`docs/features.md`, Fiscal rules table, TVA rates row, and
   Open decision 3). Lumina charges one shop-wide percentage, confirmed
   twice: a single settings field and a single tax column on the sale row,
   with no tax column on the line itself. Evidence: findings.md, "Confirmed
   by driving." Better for an Algerian shop: Dinar's. A grocery selling both
   9% staples and 19% goods on one rate would misstate one of the two under
   Lumina's model.

2. **Droit de timbre.** Dinar computes it in progressive tranches against
   the 2025 finance law, though its own base is still an assumption pending
   a comptable's confirmation (`docs/features.md`, Fiscal rules table,
   Droit de timbre row). Lumina's stamp is a toggle, off by default, and the
   rate behind it was read in an earlier pass as a flat 1% capped at 2 500
   DA, a reading the research rules already flag as superseded by the 2025
   law. Evidence: findings.md, "Confirmed by driving," stamp paragraph;
   `research/README.md`. Better for an Algerian shop: Dinar's, once its own
   base is confirmed, because it follows the current law rather than one the
   law has since replaced.

3. **Document numbering.** Both keep a counter per document kind per year.
   Dinar's is proven under test never to skip a number or hand one out
   twice, including when a sale is refused before a document is issued and
   when a document is later annulled (`docs/features.md`, Fiscal rules
   table, Numbering row). Lumina's schema carries "a document counter keyed
   by document type and year with a prefix format," but this pass did not
   drive it far enough to know whether it behaves the same way under a
   refusal or a cancellation. Evidence: findings.md, "What their schema
   holds" (Lumina side not checked past the schema). On the evidence in
   hand, Dinar's is the one with a proof behind it.

4. **Credit notes.** Lumina keeps the credit note on the sale row itself:
   the same row that carries the original sale also carries the credit-note
   number, its date, the original invoice's number and date, and a reason.
   Evidence: findings.md, "What their schema holds." Dinar's avoir is a
   separate numbered document out of its own series, may be partial, is
   capped so the running total against one facture can never pass that
   facture's own total, and never refunds the stamp (`docs/features.md` §3,
   Avoir paragraph, and the Fiscal rules table's Avoir row). Better for an
   Algerian shop: Dinar's. The decree treats a credit note as its own
   numbered paper, and a shop's own books read more cleanly with the
   correction as a separate document than folded into the sale it corrects.

5. **Roles.** Lumina names five roles against Dinar's three, but only checks
   the choice in the screen's own menu: the same session's underlying
   functions carried no check at all, and this pass used that gap to create
   a brand-new administrator account while still signed in as a low-privilege
   one. Evidence: findings.md, "Roles: enforced only in the renderer, never
   at the data layer"; `screens/roles-05-seller-created-admin-account-bypass.png`.
   Dinar checks every route that changes data, and eighteen more that only
   read it, against a table of thirteen permissions on the server itself,
   answered the same way whether the request comes from an honest screen or
   not (`docs/features.md` §5, "The permission table" paragraph). Better for
   an Algerian shop: Dinar's. It is the one of the two that would still hold
   if the till software itself were compromised or scripted against.

6. **Language and layout.** Both run Arabic, French and English with a
   right-to-left mode. Dinar flips the whole layout from one switch at the
   top of the page, so no single screen is left to handle it on its own
   (`docs/features.md` §8, Language paragraph). Lumina's Android app mirrors
   inconsistently between screens that look alike: the stock list flips a
   card's contents to read right to left, the till's near-identical product
   list does not, and one modal shows an Arabic-Indic numeral next to Western
   digits used everywhere else on the same screen. Evidence: findings.md,
   "Arabic pass: layout is not one thing"; `android-ar-product-list.png`,
   `android-ar-pos-list.png`, `android-ar-sale-confirm-modal.png`. Better for
   an Algerian shop: Dinar's approach, on the evidence, though Dinar's own
   phone client has not itself been checked for the same consistency (not
   checked).

## What to build next, ranked

1. **Till sessions**, with an opening float, cash movements through the
   shift and a count at close. Dinar's own dashboard names both gaps
   outright ("no opening float and no count at close"; nothing records cash
   handed back), and Lumina's caisse screens show a shop expects this at the
   end of a shift.

2. **A stored, independent print-language setting**, so a shop can print in
   one language while the till runs in another, matching Dinar's own stated
   aim in the Scope section. Both of Lumina's products already ship this,
   and Dinar's own §4 admits the v1 mechanism does not yet deliver it.

3. **Arabic on a cheap thermal head.** Dinar's own Thermal paragraph names
   the exact failure mode, a codepage a low-cost printer will not have,
   that Lumina's picture-based printing already works around.

Zakat and capital-injection tracking are worth a later look on market fit,
but that case rests on Lumina's evidence alone and is not backed by a gap
named in Dinar's own spec, so it sits below these three.
