---
title: 'The phone in three languages'
slug: 'the-phone-in-three-languages'
status: 'active'
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
    status: 'pending'
  - id: 'T3'
    desc: 'The phone provider and hook reading those dictionaries, language persisted on the device, RTL for Arabic on every screen the phone has'
    status: 'pending'
  - id: 'T4'
    desc: 'Every literal string in apps/mobile replaced by a key. pair.tsx, sign-in.tsx and the till are the ones with visible copy'
    status: 'pending'
  - id: 'T5'
    desc: 'A lint or a test that fails on a literal string inside JSX in apps/mobile, so the next screen cannot reintroduce one'
    status: 'pending'
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

## Not in this plan

The desktop's migration off JSON, which is the follow-up this makes cheap.
Translating the printed documents, which already have their own strings and
their own reason for being separate: a headless server prints the same paper
with no browser and no React in it.
