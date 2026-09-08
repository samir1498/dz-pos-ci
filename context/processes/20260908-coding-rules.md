---
title: 'Coding rules'
slug: 'coding-rules'
status: 'active'
category: 'processes'
created: 20260908
tldr: 'Rust, TypeScript and CSS rules that meet Anouar''s bar; layering; comments'
---
# Coding rules

Anouar's bar: UI looks good, calculations are right, no "error slop", clean
simple architecture. These rules are how a change meets it.

## Everywhere
- Verify an API or a constraint before writing against it. Check crates.io
  or npm for a maintained package before hand-rolling a commodity piece;
  say which one and why. No new dependency without a stated reason.
- Layering: `crates/core` = models → repos → services. Tauri, HTTP handlers
  and mobile call services, never diesel.
- Comments: 3 lines max, only for a constraint, a measured number or a
  trap. No restating the next line.
- Never excuse a failure as already present before your change; state the
  actual root cause.
- Prose a person reads (handoff, PR body, doc, progress entry): load
  `dont-sound-like-ai` first.

## Rust
- Money is `i64` centimes behind a `Money` newtype, checked arithmetic, no
  `f64` on any path that reaches a total or the database. The `dz-money`
  skill has the full discipline: constant + fixture + doc row move together.
- No `unwrap` / `expect` in shipped code (workspace clippy lints deny
  them). Test crates opt out with an explicit `#![allow]` at the top.
- One error enum per layer with `thiserror`. A handler maps it; it never
  re-derives it.
- Every fiscal rule in `docs/features.md` has a named fixture. Invoice
  templates get golden-file tests; a wrong field on a printed facture is a
  legal problem no UI test catches.
- Types shared with TypeScript are generated with ts-rs.

## TypeScript and CSS
- No `as` casts. A cast in a fixture makes a green test prove nothing.
- Every visible string goes through i18n and exists in `ar`, `fr`, `en`.
- Logical CSS properties only (`inline-start`, `margin-inline`). Arabic
  mirrors the layout; `left` / `right` is a bug that shows only in `ar`.
- No hardcoded design pixel or hex in UI code. Spacing, radius and colour
  come from `design/shared/tokens.css` (later the generated token
  module), or from `%`, `flex`, `gap`. A value with no matching token
  takes the closest token or a percentage of the container, never a
  one-off number.

