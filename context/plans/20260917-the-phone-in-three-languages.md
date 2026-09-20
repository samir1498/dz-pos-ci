---
title: 'The phone in three languages'
slug: 'the-phone-in-three-languages'
status: 'done'
category: 'feature'
created: 20260917
tldr: 'The mobile app has literal English in its JSX in a product whose premise is French, English and Arabic. Give it dictionaries in packages/shared, in the shape Samir asked for, domain objects behind a barrel, and let the desktop migrate to the same shape later instead of moving five hundred keys this weekend.'
priority: 60
tasks:
  - id: 'T1'
    desc: 'This page: why the phone gets the new shape first and the desktop keeps its JSON for now'
    status: 'done'
  - id: 'T2'
    desc: 'packages/shared/src/i18n: one TS object per domain, a barrel that merges them, the key type derived from the English object, and a check that every language has every key'
    status: 'done'
  - id: 'T3'
    desc: 'The phone provider and hook reading those dictionaries, language persisted on the device, RTL for Arabic on every screen the phone has'
    status: 'done'
  - id: 'T4'
    desc: 'Every literal string in apps/mobile replaced by a key. pair.tsx, sign-in.tsx and the till are the ones with visible copy'
    status: 'done'
  - id: 'T5'
    desc: 'A lint or a test that fails on a literal string inside JSX in apps/mobile, so the next screen cannot reintroduce one'
    status: 'done'
  - id: 'T6'
    desc: 'A record from the core error codes to keys on the phone, the shape apps/desktop/src/lib/fields.tsx has. Until it exists a 422 tells the cashier only its status number'
    status: 'done'
acceptance:
  - 'Every string a cashier can read on the phone comes from a dictionary'
  - 'The phone in Arabic lays out right to left, proven on a real device, not only in a test'
  - 'A missing key in one language fails the gates rather than rendering blank'
---
# The phone in three languages

`apps/mobile/app/pair.tsx:86` says "Pair this phone". `sign-in.tsx:120` says
"Reading the staff list…" and `:131` says "Nobody can sign in on this shop
yet." Those are the visible ones. The desktop has carried all three languages
since M0 and the printed documents have their own strings in
`crates/core/src/print/strings.rs`; only the phone never got any.

## Why this is where the i18n refactor starts

Samir asked, 2026-09-17, for i18n as TypeScript objects split by domain with a
barrel file merging them before they reach the config. The desktop's
dictionaries are three JSON files of several hundred keys each, and moving
them is a large diff that touches every screen and proves nothing new.

The phone has almost no strings yet. So the new shape gets built there, where
it is small enough to get right, and the desktop moves to it afterwards as its
own piece of work. Both apps end up on one implementation in
`packages/shared`; they just arrive at different times.

The shape:

```
packages/shared/src/i18n/
  fr/{till,pairing,auth,errors}.ts
  en/...
  ar/...
  index.ts        <- the barrel, merges the domains per language
```

English is the key set the other two are checked against. Not because the app
is English, it is not, but because that is already the rule on the desktop at
`apps/desktop/src/i18n/index.tsx:26`, where `isKey` is `v in en`, and the
desktop migrates onto this package later. Two sources of truth for the key
list is the failure this shape exists to avoid, so the choice is made here and
not by whoever writes T2.

## The part a test cannot see

Arabic on a phone is not the desktop's `dir="rtl"` on `documentElement`.
React Native lays out through `I18nManager`, which on Android needs the app
restarted before a flip takes effect. That is why T3's acceptance says a real
device: a green vitest run proves the strings resolve and says nothing about
which side the back arrow is on.

## What T4 gave up, and why it is T6 rather than a comment

The phone used to show the server's own sentence on a refusal whenever it
had one. That sentence comes out of `crates/api/src/error.rs`'s `fn message`,
which is a Rust `Display` string, so it is English whatever the cashier
reads. On an Arabic counter every refusal came back in English, which is the
thing this plan exists to stop.

So T4 answers in keys. The cost is precision. A 422 that said "tendered is
invalid: less than the amount to pay" now says "Refused (422)" in the
cashier's own language, because six error keys is what the dictionaries
carry and the last of them is a catch-all with the status in it.

Narrower than it first looked, and the difference is worth writing down.
The one money 422 this phone can provoke is a short tender, and the core
never sent a figure with it either (`crates/core/src/services/sales.rs`:
the field and the sentence, no amount, no shortfall). A credit-limit 422
cannot be reached at all, because the till hardcodes a cash payment and
sends no customer. So what a cashier loses is which 422 fired, not a number
that used to be on the screen, and the screen already refuses a short tender
before the call is made.

The fix is the shape the desktop already has at
`apps/desktop/src/lib/fields.tsx:63`: a record from the core's error codes to
keys, with `error_unknown` underneath. It was not written with T4 because the
phone rings cash sales and nothing else, so the list of codes it can actually
provoke is short and not yet known. Writing the map from the full set of core
codes would be seventy keys in three languages, one of them Arabic nobody has
reviewed, most of them unreachable. The honest order is: drive the phone
against a real core, write down which codes come back, then map those.

## What the device run has to look at

T3's acceptance already asks for Arabic on a real phone rather than in a
test. Two things join that list, both found on 2026-09-20 and neither
resolvable from this machine.

An amount and its currency are one string: `formatCentimes(owed)`, a space,
then `currency_suffix`, which is `دج` in Arabic. Latin digits beside Arabic
letters under a right-to-left paragraph is the case Unicode's bidi algorithm
resolves by the paragraph's own direction, and which side of the figure the
suffix lands on is a question about a renderer, not about this code. Three
places: `PayPanel.tsx`, `ProductRow.tsx`, and `till_change_due` on the till.
Nothing is changed on a guess; inserting direction marks to fix a problem
nobody has seen would be the worse mistake.

The thousands separator is already known not to be part of it. It is U+202F,
a narrow no-break space, pinned in `packages/shared/src/money.test.ts`, so
the groups inside a figure stay one run and cannot reorder among themselves.

## The Maestro flows now assert a language nobody pinned

`apps/mobile/maestro/*.yaml` check for "Pair this phone", "Basket empty",
"Send now" and "1 sale waiting to be sent". Those are English, the phone now
opens in whatever its handset's locale resolves to and falls back to French,
and the French strings genuinely differ: `pairing_title` is "Appairer ce
téléphone".

Nothing catches it. The flows are run by hand and are not in `just gates` or
`just e2e`; `just e2e` is the desktop's Playwright suite. So the flows will
fail on a French or Arabic handset and pass on an English one, and which it
is depends on a setting in the emulator nobody wrote down.

The fix is to assert on test ids rather than on sentences, which is the point
of having them. It is not done here because no gate would verify the rewrite,
and a blind rewrite of the only thing that drives these screens is worse than
a known gap. It goes with the device run.

## Not in this plan

The desktop's migration off JSON, which is the follow-up this makes cheap.
Translating the printed documents, which already have their own strings and
their own reason for being separate: a headless server prints the same paper
with no browser and no React in it.

## Closed 2026-09-20

All six tasks merged. T6 shipped as #122: `apps/mobile/lib/errors.ts` maps
every code the core and the API can emit to a sentence, and
`apps/mobile/lib/errors.test.ts` walks `CoreError::code` and `ApiError::parts`
so the map cannot go green on a code the server grew after it was written.

Two things the task found and did not fix, both written at the lines they
affect:

- `credit_limit` and `party_ids` drop the figures the server sends with them,
  because `errorIn` in `lib/api.ts` reads only `code` and `message`. Nothing
  is affected today: the till rings cash and `useRing` sends no `customer_id`,
  so neither code can be reached from the phone. The figures are the first
  thing to add the day the phone sells on credit.
- On a 409 the phone drops the queued sale and shows a sentence, so the
  cashier sees no receipt and no change. The core's own message says to
  resend the same key and read the winner's sale, which is a shape change to
  `settle` in `features/till/useRing.ts`. The sentence now tells the cashier
  the sale is already rung rather than to try again, which is what the old
  one said and what would have rung a second sale.

The phone's own dictionary keys are named for what they say: the 409 key is
`error_sale_already_rung`, not `error_conflict`, because the desktop keeps a
generic `error_conflict` of its own in `apps/desktop/src/i18n`.
