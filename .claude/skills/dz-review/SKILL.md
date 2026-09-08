---
name: dz-review
description: Adversarial review of a dz-pos change or plan with 2-3 parallel agents on non-overlapping lenses, followed by a challenge pass that proves each finding before anything is fixed. Use before merging anything that touches money, TVA, stamp duty, invoice numbering, roles and permissions, or deletes data; and before code starts on a plan in those areas.
argument-hint: [<branch>|plan:<slug>|challenge]
---

Two passes. The first finds problems, the second proves they exist. Acting
on unproven findings is how a review adds bugs: two of four review fixes
in one ObserveOne PR opened new holes.

## Pass 0: say what actually ran

Before spawning anything, write down:
- which tests ran (`cargo test` counts, vitest counts, proptest cases)
- what was driven by hand and where (mockup route, Tauri window on the
  laptop, HTTP call against `crates/api`)
- what a green suite cannot see here (a printed layout, a real printer, a
  second device on the LAN)

If nothing ran, the review still happens, but no finding may be closed on
reading alone.

## Pass 1: three lenses, in parallel, no overlap

Spawn the agents in one message. Vary the model. Brief each with the
business rules from `docs/features.md`, the branch, and "do not edit, do
not spawn, say clean rather than pad".

- **Centime correctness.** Every way a total, TVA, stamp, change or debt
  comes out wrong: rounding twice, float leaking in, a discount spread that
  loses or invents a centime, overflow, a zero or negative line, a credit
  sale that bypasses the limit, a gapless number that gaps on a failed
  write.
- **Roles and data reach.** What a cashier can do that only a manager
  should, what any client on the LAN can ask the API without a role, which
  `shop_id` filter is missing, what a deleted row takes with it, what an
  audit log does not record.
- **Fixture and test quality.** Which assertion still passes with the
  constant flipped; which fixture computes its own expectation; which
  proptest invariant is a tautology; which golden file was regenerated
  from the code it tests.

For a plan (`plan:<slug>`), read it with `ctx show` or `plan_show` and run
the same lenses on the design: does the data model cover the awkward case,
what is missing that a reviewer expects (rollback, migration, i18n), and
which of the plan's claims were verified against code rather than assumed.

## Pass 2: challenge every finding

Each finding is a claim. Sort it:
1. **Provable by reading code.** Read it yourself. Quote the line.
2. **Depends on data or runtime.** Run it: a fixture, a `cargo test` with
   the flip, a request against the API. Don't take the agent's word.
3. **A design tradeoff.** Say so plainly and let Samir decide; it is not a
   defect.

Report only what survived, ranked by consequence on a real facture or a
real customer's debt. Mechanism and blast radius are separate claims; do
not supply the stake a finding lacks.

## Pass 3: five questions on the whole

Did we build the right thing. Is there a materially simpler shape. How
complicated is this honestly. Is any of it already solved elsewhere in the
repo or in a crate we already depend on. What can be deleted.

## Then fix, and review the fixes

The fixes get their own round on the same lenses, aimed only at the diff
the fixes produced. Report with `ReportFindings` when the host asks for
it, otherwise as a short ranked list with file and line.
