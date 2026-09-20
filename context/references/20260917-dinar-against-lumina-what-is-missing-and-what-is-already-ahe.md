---
title: 'Dinar against Lumina: what is missing and what is already ahead'
slug: 'dinar-against-lumina-what-is-missing-and-what-is-already-ahe'
status: 'active'
category: 'references'
created: 20260917
tldr: 'Feature-by-feature comparison with Lumina POS: seven real gaps ranked by what a shop notices, an eighth withdrawn after checking our own code, four places Dinar is already better, and three things not worth copying'
---
# Dinar against Lumina

Anouar, 2026-09-17: build against Lumina POS as the example, because they
have been selling to Algerian shops for years and customise for this market,
and that is what we want rather than software for ourselves or for an
international market. He asked for it to be the priority.

The competitor map is `~/lumina/features.md`, reverse-engineered from their
APK, their Windows installer and a screen recording. This page is the part
that was missing: what they have that we do not, ranked by what a shop owner
would notice in the first hour, and what we have that they do not.

Lumina sells for 12 000 DZD once, no subscription, trial capped at 100 sales,
activation by phone call. They are in Aïn Beïda, Oum El Bouaghi.

## The gaps, ranked by what a shop notices

**1. One invoice layout against their eleven.** They ship
`invoice_template_1` through `_9`, plus an A5 and a detailed variant, plus an
80 mm thermal receipt, plus five receipt templates, each in Arabic, English
and French. We print one facture layout, one ticket, one barcode label, one
debt slip and one statement. A shop that does not like our layout has no
answer, and a buyer comparing the two sees a menu against a single page. This
is the cheapest gap to close and the most visible.

**2. No batches or lots.** Their `/api/batches/` is real and the mobile app
says "Ajouter un lot". A grocery or a pharmacy tracks expiry, and without it
those shops cannot use us at all. Already in `docs/features.md` under "Later,
agreed"; this is the argument for moving it.

**3. No product variants.** Their product form has a third tab for variants
and colours with per-variant pricing. A clothing or shoe shop is unsellable
to without it. Also already in "Later, agreed".

**4. No bundles and no promotions.** Two items in their sidebar,
الحزم and العروض الترويجية. A shop running a promotion has no way to express
one in Dinar today.

**5. No AI invoice scanning.** Their mobile app photographs a supplier
invoice and extracts the products and the supplier, quota-metered. It never
appears in their desktop recording and it is the most advanced thing in the
product. `docs/features.md` already calls it "the one modern thing the
competitor has".

**6. No cloud sync.** They offer four network modes: single device, LAN,
cloud between branches, and hybrid. We have single device and LAN. A shop
with two branches cannot buy us. This is open decision 1, not a bug.

**7. Withdrawn, 2026-09-17.** This slot read "no supplier opening debt". It
was wrong. `services::suppliers::create` takes `opening_debt: Option<Money>`
and writes it as the first `opening` row of the supplier ledger inside the
same transaction as the fiche and the audit entry
(`crates/core/src/services/suppliers.rs:103-160`). The route reads
`opening_debt_centimes` (`crates/api/src/routes/suppliers.rs:76`), the field
is on the form (`apps/desktop/src/routes/suppliers.tsx:528`) in all three
languages, and `suppliers.test.tsx:339` types into it. Customers have the
same, deliberately built the same way. The survey that produced this page
checked Lumina's form and not ours.

**8. No delivery notes.** They ship `delivery_note` in A4, A5 and thermal.
We hold the `bon_de_livraison` kind in the model and nothing else, on purpose:
décret 05-468 art. 14-17 allows one only alongside a facture récapitulative
and a wilaya authorisation. Closing this gap means doing both, not copying
their template.

Seven gaps, then, not eight. Every claim above except the withdrawn one was
re-checked against the code on 2026-09-17: there is one facture renderer and
no layout setting (`crates/core/src/print/`), nothing matches batch, lot or
variant in `crates/core/src/models/`, nothing matches bundle or promotion
outside `support_bundle`, and `DocumentKind::BonDeLivraison` exists in
`sql_types.rs:114` with no renderer behind it.

Smaller, listed so the next sweep does not rediscover them: biometric unlock
and Bluetooth ESC/POS printing from the phone, a capital screen, and their
seeded expense categories against our empty list.

## Where Dinar is already ahead

**TVA per product, defaulted from the category.** They use a single global
rate. CTCA art. 23 lists the 9 % goods by customs tariff line, so the rate is
a property of the product and a global rate is wrong for any shop selling
both. Decided here 2026-09-08 and built that way.

**Three languages everywhere: struck 2026-09-20, it was never true.** This
page said their desktop was Arabic only and their phone French only. Driving
both builds says otherwise. Their settings screen offers Arabic, English and
French and switching retitles every screen and turns the layout round; their
Android app opens in Arabic and carries an Arabic-French toggle. Nothing here
is an advantage, and our own phone had no dictionaries at all until
2026-09-20. What is left of the claim is the printed documents, which carry
their own three-language strings in `crates/core/src/print/strings.rs` and are
a separate thing from the screens. The walks and the screenshots that settle
it are in `research/competitors/2026-09-20-lumina-teardown/`.

**Where the language work actually stands.** The desktop has carried all
three since M0. The phone got its dictionaries and its provider on
2026-09-20 and its screens still read literal English
(`context/plans/20260917-the-phone-in-three-languages.md`).

**Money that cannot drift.** Integer centimes, checked arithmetic, no float
anywhere near a total, and fixtures the core is tested against. Their pricing
logic is compiled to bytecode so it cannot be read, but a single global TVA
rate suggests the model is simpler than the law.

**An audit log and a permission table.** Nothing in their bundle suggests
either.

## Not worth copying

Activation by a phone call and a human sending back a code. Their
`/api/cloud/sync/wiper-wash-alert` route, which is a car wash leaking through
a shared backend template. Their trial cap counted in sales rather than days.

## What this means for the order of work

The invoice layouts are the one gap that is both visible to a buyer and cheap,
because the fiscal field list and the three-language strings already exist and
only the layout changes. Batches and variants decide which kinds of shop can
buy at all, so they come next and the choice between them is a choice of which
market to enter first. Everything else is further out or blocked on a
decision.
