---
title: 'A print language the shop keeps'
slug: 'a-print-language-the-shop-keeps'
status: 'active'
category: 'feature'
created: 20260920
tldr: 'One stored setting decides what language every fiscal paper prints in, independent of the screen. A shop that never opens the panel keeps exactly today behaviour, so nothing changes until somebody chooses.'
priority: 80
tasks:
  - id: 'T1'
    desc: 'This page: what the setting is, what reads it, what deliberately does not, and the precedence in one line'
    status: 'done'
  - id: 'T2'
    desc: 'A print_lang key in preferences, the service pair that reads and writes it, the gated PUT route, the DTO and the settings panel beside the facture layout'
    status: 'done'
  - id: 'T3'
    desc: 'The six document routes take ?lang= as an override and read the preference when it is absent; docs features.md 4 and 8 rewritten in the same PR'
    status: 'pending'
---
# A print language the shop keeps

`docs/features.md:18` has said since v1 that the UI language and the print
language are chosen separately. `docs/features.md` §4 admits the v1
mechanism does not do it: what prints is whatever language the till happens
to be open in at the moment somebody presses print, passed on the call.
Both of Lumina's products ship the stored setting, and the teardown ranked
it second of the three gaps worth closing
(`research/competitors/2026-09-20-lumina-teardown/gap-summary.md`).

## What Samir ruled, 2026-09-20

One setting, not two. It covers every fiscal paper and every slip: the
ticket, the facture, the customer statement and the debt slip. Lumina
keeps the invoice and the receipt apart. A shop that prints its tickets in
Arabic and its factures in French is unusual, and a second setting is a
second thing that drifts out of step with the first.

The default is today's behaviour. A shop that never opens the panel prints
in the language the till is being used in, and it does not fall back to
French. A fresh shop running an Arabic till would otherwise print French
paper until somebody found the panel, and nobody would connect the two.

The barcode label and the import template keep following the caller's
language. Neither is a fiscal paper, and a label carries a name, a price
and a barcode that are already on the shelf.

## The precedence, in one line

`?print_lang=` on the call, then the shop's stored `print_lang`, then
`?lang=`, the language the caller asked in. No step is French-by-default.
It is `?print_lang=` and not `?lang=` because every desktop caller already
sends `?lang=` as the till's own language on all six routes, so `?lang=`
becoming the override would mean the stored setting was never reached.

The first step already exists in the same shape: `?layout=` wins over the
stored facture layout for a preview at `crates/api/src/routes/sales.rs:293`.
This follows it rather than inventing a second precedence.

## What holds the setting

`preferences`, the same table `theme` and the facture layout live in. It
has no CHECK, so the service parses `fr|en|ar` the way
`services::preferences::theme` parses a theme, and a value it does not
recognise reads as no preference rather than as an error. A row written by
a future version, or by hand, therefore degrades to today's behaviour
instead of refusing to print.

No migration. A key in an existing table is not a schema change, and the
shape that would need one, a per-document-kind setting, is the thing the
ruling refused.

## What reads it

The six routes that answer a fiscal paper:

- `GET /sales/{id}/ticket`
- `GET /sales/{id}/ticket/escpos`
- `POST /sales/{id}/print`
- `GET /sales/{id}/facture`
- `GET /customers/{id}/statement`
- `GET /customers/{id}/debt-slip`

## What does not

`GET /labels/sheet` and `GET /import/products/template`. Both keep the
caller's language, by the ruling. A test asserts this rather than leaving
it to be noticed: a later change that quietly routes them through the
preference should go red.

`print::strings` is untouched. It already holds the three languages and is
deliberately separate from the desktop's own i18n so a headless server
prints the same paper (`docs/architecture.md`). Nothing here changes what
a word is, only which of the three is asked for.

## What the goldens say

Nothing changes in them. `amount_in_words` in the totals table of
`docs/features.md` says `net_to_pay` is written out in the print language,
and the per-language goldens already prove each of the three. This work
decides which language is asked for, not what that language produces, so a
golden that moved would mean something went wrong.

## Who may set it

`edit_settings`, a row in `crates/api/src/gates/table.rs` beside the
facture layout's. It changes the paper the shop hands a customer, which is
the same reason the layout is not open to a cashier. The gates table's own
write-side allow-list, added on 2026-09-20, means a row landing here with
no permission fails the unit test rather than shipping open.

## Done, per task

**T2.** A `PUT` persists per shop and survives a restart. A cashier gets
`forbidden`. The panel mutates and reads back in its own test.

**T3.** An e2e in `apps/desktop/e2e/settings.spec.ts` sets Arabic with the
UI in French, prints a ticket, and asserts the returned page is
`lang="ar" dir="rtl"`. A core test pins all three steps of the precedence:
the override beats the setting, the setting beats the caller's language,
and a shop with nothing stored prints in the caller's language.

## Out of scope

A second setting per document kind, a per-customer language, and anything
that makes the label or the import template follow the setting. Each was
refused by name in the ruling.
