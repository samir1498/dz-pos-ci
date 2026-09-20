---
title: 'Invoice layouts a shop can choose'
slug: 'invoice-layouts-a-shop-can-choose'
status: 'active'
category: 'feature'
created: 20260917
tldr: 'Lumina ships eleven facture layouts plus an A5 and a thermal one; we ship a single A4 with no way to pick another. Build the picker first, then three more layouts, and let the golden harness that already parses the amounts back out of each file keep the totals honest.'
priority: 85
tasks:
  - id: 'T1'
    desc: 'This page: what exists, what a layout is allowed to change, and which golden set each new layout owes'
    status: 'done'
  - id: 'T2'
    desc: 'The layout registry and the shop setting: a print_layout value, render_facture_with taking a layout, the DTO, the route and the settings picker. No new template; the default stays today A4 so every existing golden holds byte for byte'
    status: 'pending'
  - id: 'T3'
    desc: 'facture_compact_a4.html: a denser A4 that fits more lines on a sheet with the fiscal block intact. Full golden set, six cases by three languages'
    status: 'pending'
  - id: 'T4'
    desc: 'facture_a5.html on the half sheet. Paper::A5 already exists and only sets the CSS page size; this is the layout that fits inside it. Reduced golden set: plain, cash, avoir, by three languages'
    status: 'pending'
  - id: 'T5'
    desc: 'facture_80mm.html, a facture on the roll, browser printable. Reduced golden set by three languages. This alone closes T5'
    status: 'done'
  - id: 'T6'
    desc: 'e2e: the picker persists per shop, and a facture printed after a change comes back in the chosen layout'
    status: 'done'
  - id: 'T7'
    desc: 'The 80mm facture down the ESC/POS path beside escpos.rs. Parkable on purpose: bidi on a roll and the ISO 8859-15 encoding are where a weekend goes, and T5 has already given the shop a printable layout without it'
    status: 'parked'
acceptance:
  - 'A shop picks one of four facture layouts in settings and every facture printed afterwards uses it'
  - 'Every layout carries the fields docs/features.md section 4 requires of a facture, in all three languages'
  - 'Each layout has golden files per language, and each golden has its amounts parsed back out and compared, so a drifting total cannot be accepted by regenerating'
---
# Invoice layouts a shop can choose

Anouar, 2026-09-17: build against Lumina, they have sold to Algerian shops for
years. The reference page
`dinar-against-lumina-what-is-missing-and-what-is-already-ahe` ranks this
first, because it is both the most visible gap to a buyer and the cheapest to
close: the fiscal field list and the three-language print strings already
exist and only the layout changes.

## What exists today

One renderer, `crates/core/src/print/facture.rs`, over one template,
`crates/core/templates/facture_a4.html`. It is pinned by eighteen golden files
in `fixtures/print/facture_a4/`: six cases, plain, cash, avoir, annulee, ifu
and proforma, in Arabic, English and French.

`Paper` in `facture.rs:45` already has an `A5` variant, and all it does is set
the CSS `size` descriptor. Nothing in `crates/api` passes anything but
`Paper::A4`. So the half sheet is reachable from Rust today and produces an A4
layout squeezed onto A5 paper, which is not the same thing as an A5 layout.

There is no setting anywhere that says which layout a shop wants.

## What a layout is allowed to change, and what it is not

A layout changes where things sit on the page and how dense they are. It does
not change what is on the page. Every facture, in every layout, carries the
fields `docs/features.md` section 4 lists, because that list is the law and
not a design choice. A layout that drops the NIF to save a line is not a
layout, it is an invalid facture.

It does not compute anything either. The totals arrive in `FactureInput`,
already computed by the same code in every layout. This is why the templates
can be built by `dz-builder` rather than `dz-money-builder`: no new arithmetic
is written. The guard that makes that safe is the golden harness itself, whose
header at `crates/core/tests/print_ticket.rs:12` records the rule: the amounts
in each golden are parsed back out of the file and compared, so a golden that
drifts from the totals cannot be accepted by regenerating it. Every new
layout's test reuses that same check. A layout test that only compares bytes
is not enough and does not close its task.

## The order, and why the picker comes before the templates

T2 ships the machinery with no new layout behind it. That sounds like the
least interesting task and it is the one that decides whether the rest is
cheap: with a registry in place each later layout is one template file plus
its goldens, addable in any order and by parallel branches. Without it, each
new layout is a fresh argument about where the selection lives.

T3, T4 and T5 each add a variant to the registry T2 built, so all three touch
that one enum and its match arms. They are still independent branches; the
second and third to merge rebase onto the first rather than treating the
conflict as a failed attempt. That is one rebase, not a strike.

T2 is also the only task in this plan that can break an existing document.
Its own proof is that the eighteen `facture_a4` goldens are byte for byte
unchanged afterwards, because the default is still today's layout.

## Golden cost, stated because it is most of the work

The alternate A4 replaces today's A4 in every situation a shop might print
one, so it owes the full set: six cases by three languages, eighteen files.
A5 and thermal are chosen deliberately for a subset of situations, so each
owes plain, cash and avoir by three languages, nine files. That is thirty-six
new goldens across the plan, every one of them read by a human before merge.

## Not in this plan

Lumina's other eight A4 variants. Four layouts prove a shop can choose; nine
more is catalogue work that costs a template each and can be added whenever
a shop asks for one. Receipt templates for the ticket are also out: the
ticket is one paper and nobody has asked for a second.

Delivery notes stay out for the reason the reference page gives. Décret
05-468 art. 14-17 allows a bon de livraison only alongside a facture
récapitulative and a wilaya authorisation, so closing that gap means building
both, not adding a template.
