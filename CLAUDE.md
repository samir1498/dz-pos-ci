# CLAUDE.md — dz-pos

Placeholder-named product. Read `README.md` first; the six rules there are
load-bearing, and the research folder it links to holds the reasons.

## How to work
- Senior-engineer mindset: verify APIs and constraints before writing code.
- Respect the existing layering: `crates/core` = models → repos → services.
  UI code (Tauri commands, later HTTP handlers, mobile) calls services, never
  diesel directly.
- No new dependencies without a stated reason. No TypeScript `as` casts — a
  cast in a fixture makes a green test prove nothing.
- Comments: 3 lines max, only for a constraint, a measured number or a trap.
- Never write "pre-existing"; state the actual root cause.
- Money is integer centimes. Anything that touches TVA, stamp duty or
  amount-in-words gets property tests on rounding.
- Invoice templates get golden-file tests. A wrong field on a printed
  facture is a legal problem no UI test catches.

## Gates before "done"
`cargo fmt --all --check` · `cargo clippy --workspace -- -D warnings` ·
`cargo test --workspace` · `pnpm -r build` · `pnpm -r test`.
Show the output. CI green is not "tested" — drive the behaviour.

## Git
- Branch + PR; never commit to `main` directly once CI exists on it.
- Never `git reset --hard`, `git clean -fd` or force-push without asking.
