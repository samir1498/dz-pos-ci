---
title: 'Quality gates'
slug: 'quality-gates'
status: 'active'
category: 'processes'
created: 20260908
tldr: 'just gates and just e2e, what counts as tested, the extra layers for money/roles/deletion, and when a test is deleted or merged instead of kept'
---
# Quality gates

Before "done" and before a PR, run from the repo root and show the output:

```
just gates   # cargo fmt --check, the desktop eslint rule, the file-size
             # ratchet (sizes), the inline-test ban (no-inline-tests),
             # clippy --all-targets -D warnings, generated TS types
             # diffed against the DTOs (types-check), cargo test,
             # pnpm -r test, pnpm -r build
just e2e     # Playwright against a fresh API and database
```

GitHub Actions is not the PR gate. `just ci` copies the commit to the
public personal mirror: Restricted (rustfmt, desktop eslint, release-gate
script) on every push, Full (clippy, tests, build; Windows and coverage
on main) on main or `just ci <branch> full`. Dinar-dz never starts a
runner.

Sonar is local, same as ObserveOne: `just sonar` against
sonar.observeone.com (project `dz-pos`, gate "ObserveOne way") on the
branch before merge and again on main after. Not in CI. `just sonar`
depends on `just coverage`, which runs `cargo llvm-cov --workspace
--lcov` for the Rust half and `vitest run --coverage` in each of the
five TypeScript packages, so a scan always reads a fresh lcov report
rather than whatever an earlier run left behind.

A DTO change without `just types` fails `types-check`; the committed
`packages/shared/src/generated` is diffed both ways.

CI green is not "tested". Say what you drove and on which machine: a
mockup route through the `dz-mockup` drive scripts, a Tauri window on the
laptop, an HTTP call against `crates/api`. If a gate did not run (no
display, laptop offline), write that rather than ticking the box.

Money, roles and permissions, and anything that deletes data need two more
layers before merge: every path executed against the real thing (a real
SQLite file, a real request, a real printed layout), and the `dz-review`
skill's parallel adversarial pass with its challenge round. Mocks confirm
the code does what you intended; they say nothing about what the database
did.

A new test of a money rule is not done until a mutation (flip the constant
or the rounding direction) makes at least one named fixture fail. Commit
before the mutation loop.

## What a test has to be able to do

Fail, for a reason that matters. Everything below follows from that, and
it applies to every suite in the repo, not only to money.

A test that cannot fail is deleted, not repaired. The shapes to look for:
a loop that asks a list about itself (every member of `ALL` is in `ALL`),
a fixture that computes its expectation by calling the
code under test, a golden regenerated from the run it was meant to check,
a property whose invariant restates the implementation. The suite's test
count is not the claim; what a failure would have told you is.

A change-detector is deleted on sight. That is a test whose whole
assertion is that a call was forwarded to a mock with the arguments it was
handed. It goes red when a method is renamed and stays green when the
answer is wrong, so it costs maintenance and buys nothing.

Trivial tests get merged, not kept apart. Twelve cases each asserting
one key of one object are one case with a sorted comparison, and the
merged one prints which keys and which way round where each of the twelve
printed half. The merge is only allowed when no claim is lost: count the
claims, not the test cases. Going from thirty-two to twenty-three while
every mutation still fails is the suite getting sharper.

None of the three is decided by reading. Break the thing the test
claims to hold, watch that named test go red, put it back. A test kept or
deleted on a plausible read of its assertion is a guess, and the guess has been
wrong here often enough to be worth a rule.

These four come from the ObserveOne testing process; the mutation loop
above is the dz-pos money case of the same rule.

Property tests: proptest in Rust, fast-check in TypeScript, for pure
logic with a real invariant, a round trip, totality over the input
range, an invariant that survives the operation, or an oracle to check
against. Not for UI and not for anything behind mocked IO; a generator
feeding a mock proves the mock, not the code. When a property test finds
a failing case, freeze that case as a plain fixture in the same commit
so the regression stays even if the generator's seed changes. No
root-cause claim for a property failure without a repro that actually
ran; a plausible read of the assertion is not evidence.

Sonar: `just sonar` on this machine, project `dz-pos` on
sonar.observeone.com, gate "ObserveOne way". The team server has the Rust
plugin (1.5.0, 85 rules). Not a CI job.

