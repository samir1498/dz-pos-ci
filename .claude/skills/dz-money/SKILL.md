---
name: dz-money
description: Rules for any dz-pos code that computes an amount: prices, TVA, discounts, stamp duty, totals, change, debt, amount in words, invoice numbering. Use before writing or changing such code in Rust, TypeScript or the design mockups, and when reviewing it. Also use when a fiscal rule is questioned, added, or confirmed by Anouar or a comptable.
---

Anouar's two hard requirements were "UI must look good" and "calculations
must be correct". This skill is the second one. A wrong centime on a
printed facture is a legal problem, and a rounding rule that differs
between the Rust core and the web layer is a wrong centime that no single
test sees.

## Where things live

| Thing | Home |
|---|---|
| The rules in prose, each with its fixture name | `docs/features.md`, fiscal rules table |
| Constants (rates, stamp bounds, rounding mode) | one module: `crates/core/src/money/` (Rust), display side in `packages/shared/src/money.ts`; `design/shared/money.js` is the mockup copy the shared fixtures pin |
| Fixtures | `fixtures/money/*.json`, loaded by `cargo test` and by vitest. One file, two runners. |
| Property tests | `crates/core/tests/money_prop.rs` (proptest) |

Search these before adding a rule. A second definition of the stamp bound
is the bug, even if both are right today.

## Non-negotiables

- Money is `i64` centimes behind a `Money` newtype. No `f64` on any path
  that reaches a total, a ticket, or the database. Percentages are integer
  basis points (`1900` for 19 %) applied with checked integer arithmetic.
- Rounding happens once per rate group, half away from zero, at the point
  `docs/features.md` says. Not per line, not again at the total.
- Every arithmetic op is checked (`checked_add`, `checked_mul`); overflow
  is an error variant, not a panic. `clippy::arithmetic_side_effects` is on
  for the money module.
- A rule has three parts or it does not exist: the constant, the fixture
  that asserts it by name, and its row in `docs/features.md`. Change one,
  change all three in the same commit.
- Fixtures are JSON with inputs and expected outputs in centimes. No
  computed expectations; a fixture that derives its expected value with the
  same formula proves nothing.
- Property tests state invariants, not formulas: `sum(lines) == total_ht`,
  `total_ttc == total_ht + tva`, stamp inside its bounds, `net_to_pay -
  paid == change`, translating back from words yields the same integer.

## When a rule is in doubt

State it as an assumption, pin it in a fixture with the name from the
table, and list it under "Assumptions pinned" in the PR. Do not block on
Anouar for a rule the fixture can flip later; do block if two readings
change the data model (TVA per product against TVA global did, and was
decided per product).

When a rule is confirmed (a stamped facture, a comptable's answer), record
who and when in the `docs/features.md` row. Until then the row says
"assumption".

## Mutation check before calling a money test done

Flip the constant or the rounding direction and run the suite; at least one
named fixture must fail. Twice in the ObserveOne repos a fresh spec passed
with the code broken because its expectation was computed from the code
under test. Commit before the mutation loop so `git checkout <file>`
cannot take uncommitted work with it.

## Amount in words

Golden files per language (`words_fr_golden`, `words_en_golden`,
`words_ar_golden`), covering 0, 1, 10, 11, 71, 80, 81, 100, 200, 1000,
1 000 000, and a centime fraction. French agreement rules (`quatre-vingts`
against `quatre-vingt-un`, `cent` against `cents`) are where implementations
go wrong; the fixture has them. Arabic is a placeholder in the mockup until
a native speaker reviews the golden file.
