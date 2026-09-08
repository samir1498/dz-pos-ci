---
title: 'Money module: centimes, TVA, stamp, amount in words'
slug: 'money-module-centimes-tva-stamp-amount-in-words'
status: 'active'
category: 'core'
created: 20260908
tldr: 'Pure money module in crates/core, no DB: centimes, TVA per rate, stamp, words; one fixture set shared by Rust and the mockup JS'
tasks:
  - id: 'T1'
    desc: 'Money newtype over i64 centimes with checked arithmetic'
    status: 'pending'
  - id: 'T2'
    desc: 'pct half-away-from-zero, TVA grouped per rate rounded once, global discount spread across rate groups'
    status: 'pending'
  - id: 'T3'
    desc: 'Stamp duty clamp(1%, 500, 250000) centimes, cash only, toggle'
    status: 'pending'
  - id: 'T4'
    desc: 'Amount in words fr/en/ar'
    status: 'pending'
  - id: 'T5'
    desc: 'Shared JSON fixtures under fixtures/money loaded by cargo test and by vitest against design/shared/money.js'
    status: 'pending'
  - id: 'T6'
    desc: 'proptest: centimes conserved, stamp within bounds, words round-trip'
    status: 'pending'
  - id: 'T7'
    desc: 'Gates green, PR with every fiscal assumption listed'
    status: 'pending'
  - id: 'T8'
    desc: 'Stamp rule follows research R1 (progressive tranches, electronic exempt), not the clamp; do not implement the stamp until R1 lands'
    status: 'pending'
acceptance:
  - 'cargo test and pnpm -r test both load fixtures/money/*.json and pass; a changed constant fails both'
references:
  - 'docs/features.md#fiscal-rules-current-assumptions'
---
# Money module: centimes, TVA, stamp, amount in words

## Goal

Calculations right before anything else (Anouar, 2026-09-07). Every fiscal rule in `docs/features.md` becomes a named fixture that both `cargo test` and vitest (against `design/shared/money.js`) load, so Rust and TypeScript can never disagree on a centime. Rules are current assumptions until a comptable confirms; a correction is one constant and one fixture.

## Scope

In: `crates/core/src/money/`, `fixtures/money/*.json`, Rust tests + proptests, a vitest file under `design/`. Out: database, API, UI, Arabic words beyond a placeholder if the generator is not straightforward.
