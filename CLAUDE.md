# CLAUDE.md for dz-pos

Placeholder-named product. Read `README.md` first. Every rule lives in
`context/` (the pc-ctx store) so the laptop clone and every session read
the same pages; this file is the index.

## Read before working
- `context/repos/20260908-dz-pos.md`: where the repo lives on each machine,
  stack, layout, commands, the files that decide things.
- `docs/features.md` (spec, fiscal rules with fixture names) and
  `docs/architecture.md` (the six rules, layers, error policy).
- `context/progress/now.md` and `ctx status` for what is
  in flight. The status site for Anouar is https://dinar-reports.pages.dev/
  (generator in `~/.dz-night/report/` on the WSL box).

## Processes (`context/processes/`)
- `coding-rules`: Rust, TypeScript, CSS; layering; comments; money in centimes.
- `quality-gates`: `just gates` and `just e2e`, what counts as tested, the extra layers
  for money, roles and deletion.
- `machines-and-heavy-jobs`: WSL box vs laptop, the shared-box claim rule,
  no worktrees while one session per machine works the repo.
- `security-and-provenance`: ISO 27001 controls per feature; where a
  learned fact or a fiscal claim gets written.
- `git-and-planning`: branch + PR, the `ctx:` trailer, session rituals.
- `frontend-conventions`: folder shape, the design package tiers, which
  runner owns which layer.

## Skills (`.claude/skills/`, symlinked into `~/.claude/skills/` on the WSL box)
`step-by-step` (one step per message when Samir drives by hand; M1 runs
as a loop instead, see `context/progress/now.md`) · `dz-context` · `dz-money` · `dz-review` · `dz-pr` ·
`dz-mockup` · `laptop-dev` · `git-commit-convention` · `dont-sound-like-ai`.
User-level `stale-check` (any repo) reads `.claude/stale-homes.md` here for
where each duplicated fact lives and how a fix is routed.

## Non-negotiables (repeated here because they are cheap to forget)
- Money is integer centimes, checked arithmetic, no `f64` near a total.
- No `unwrap` / `expect` in shipped code (clippy denies it). No TS `as`
  type assertions (`as const` and `satisfies` are fine).
- Branch + PR for code, `ctx:` trailer in the trailer block; only `context/`
  bookkeeping goes straight to `main`.
- Never excuse a failure as already present before your change; state the
  actual root cause.
