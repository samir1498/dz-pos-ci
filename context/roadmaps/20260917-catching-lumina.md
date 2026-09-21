---
title: 'Catching Lumina'
slug: 'catching-lumina'
status: 'active'
category: 'roadmap'
created: 20260917
period: '2026-H2'
tldr: 'Anouar made Lumina the benchmark on 2026-09-17. Seven real gaps; a weekend loop and then a second loop built most of them, and a third pass finishes the till and cleans the pages before Dinar is replanned as a modular product. What is left needs a market decision or a legal one, and is parked with the reason.'
priority: 95
entries:
  - ref: 'architecture-fixes-without-a-domain-split'
    status: 'done'
    note: 'Goes first. Splitting crates/api/src/dto.rs by domain has to land before the Lumina branches start appending wire types to it in parallel'
  - ref: 'invoice-layouts-a-shop-can-choose'
    status: 'done'
    note: 'Gap 1 and the priority Anouar named. The picker first, then a dense A4, an A5 and an 80mm facture'
  - ref: 'the-phone-in-three-languages'
    status: 'done'
    note: 'Not a Lumina gap, we are ahead of them on languages. It is our own hole, and it is where the i18n refactor shape gets built small'
  - ref: 'a-print-language-the-shop-keeps'
    status: 'done'
    note: 'Gap 3. One stored setting decides what language every fiscal paper prints in, independent of the screen'
  - ref: 'the-last-six-reaches-stand-behind-rings'
    status: 'done'
    note: 'The carried-in architecture work: the cross-domain reach list is two rows and three reaches, each kept with its reason'
  - ref: 'whether-dinar-becomes-a-core-and-modules'
    status: 'active'
    note: 'Not a Lumina gap. Anouar asked on 2026-09-21 for a shared core with a module per trade; this decides it on evidence rather than argument, and blocks nothing in the loop'
  - ref: 'till-shifts-a-float-and-a-count'
    status: 'active'
    note: 'Gap 2. The table, the service and the four routes are merged; the two screens, the cash refund and the closing sweep are what is left'
---
# Catching Lumina

Anouar, 2026-09-17: build against Lumina POS, they have been selling to
Algerian shops for years and they customise for this market, which is what we
want rather than software for ourselves or for an international market. He
asked for it to be the priority.

The comparison is
`context/references/20260917-dinar-against-lumina-what-is-missing-and-what-is-already-ahe.md`,
written the same day against their APK, their Windows installer and a screen
recording. It found eight gaps and one of them was wrong: supplier opening
debt has been built since M2, core, route, form and tests, and was withdrawn
on the page. Seven stand.

## What this roadmap builds

Three plans, in the order above, as one loop across the weekend of 18 and 19
September. The loop file is
`context/loops/20260917-catching-lumina-weekend-loop.md`.

Only the first of the three is a Lumina gap. That is deliberate. The
architecture work is the thing that makes the Lumina branches cheap rather
than expensive, and the phone's languages are a hole of our own that nobody
outside would excuse in a product whose premise is that it speaks three.

## What is parked, and what unparks it

**Batches and lots**, and **product variants**. Each is a schema change with a
migration, and each decides a market: batches open groceries and pharmacies,
variants open clothing and shoes. Samir, 2026-09-17: neither this weekend.
They also need rulings an unattended loop cannot take, what happens to a lot
that expires mid-basket, whether a variant carries its own barcode and its own
stock count. Unparked by Samir picking a market and answering those.

**Bundles and promotions.** Two items in Lumina's sidebar. A shop running a
promotion has no way to express one in Dinar today. Nothing in
`crates/core/src/models/` matches bundle or promotion. Unparked when a shop
asks, or after variants, whose shape it partly shares.

**AI invoice scanning.** Photograph a supplier invoice, extract the products
and the supplier. The most advanced thing in their product and the one thing
`docs/features.md` already calls "the one modern thing the competitor has".
It never appears in their desktop recording, so it is a phone feature. Needs a
model, a quota and a cost per shop; that is a business decision before it is a
build.

**Cloud sync between branches.** They offer four network modes, we have two.
A shop with two branches cannot buy us. This is open decision 1, not a bug.

**Delivery notes.** They ship one in A4, A5 and thermal.
`DocumentKind::BonDeLivraison` exists at `crates/core/src/models/sql_types.rs:114`
with no renderer behind it, on purpose: décret 05-468 art. 14-17 allows a bon
de livraison only alongside a facture récapitulative and a wilaya
authorisation. Closing this means building both, not copying their template.

## Where we are ahead, so nobody trades it away

TVA per product defaulted from the category, against their one global rate,
which CTCA art. 23 makes wrong for any shop selling both 9 and 19 per cent
goods. Three languages on both apps and on the printed documents, against
their Arabic-only desktop and French-only phone. Integer centimes with checked
arithmetic and fixtures. An audit log and a permission table, neither of which
appears anywhere in their bundle.

None of these is on the list above, because none of them needs work. They are
here so a sprint aimed at looking like Lumina does not quietly copy a global
TVA rate on the way.
