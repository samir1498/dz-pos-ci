---
title: 'Quality gates'
slug: 'quality-gates'
status: 'active'
category: 'processes'
created: 20260908
tldr: 'just gates and just e2e, what counts as tested, the extra layers for money/roles/deletion'
---
# Quality gates

Before "done" and before a PR, run from the repo root and show the output:

```
just gates   # cargo fmt --check, clippy --all-targets -D warnings,
             # generated TS types diffed against the DTOs (types-check),
             # cargo test, pnpm -r test, pnpm -r build
just e2e     # Playwright against a fresh API and database
```

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

Property tests: proptest in Rust, fast-check in TypeScript, for pure
logic with a real invariant, a round trip, totality over the input
range, an invariant that survives the operation, or an oracle to check
against. Not for UI and not for anything behind mocked IO; a generator
feeding a mock proves the mock, not the code. When a property test finds
a failing case, freeze that case as a plain fixture in the same commit
so the regression stays even if the generator's seed changes. No
root-cause claim for a property failure without a repro that actually
ran; a plausible read of the assertion is not evidence.

Sonar: not set up for this repo yet (plan
`repo-tooling-skills-and-rules-for-dz-pos`, T4 checks Rust support on the
team server). Do not quote a Sonar gate until it exists.

