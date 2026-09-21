# CLAUDE.md for dz-pos

Product name Dinar; repo and crate paths stay `dz-pos`. Read `README.md`
first. Every rule lives in
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
  for money, roles and deletion, and when a test gets deleted or merged
  rather than kept.
- `machines-and-heavy-jobs`: WSL box vs laptop, the shared-box claim rule,
  the disk gate, worktree teardown.
- `security-and-provenance`: ISO 27001 controls per feature; where a
  learned fact or a fiscal claim gets written.
- `git-and-planning`: branch + PR, the `ctx:` trailer, session rituals.
- `frontend-conventions`: folder shape, the design package tiers, which
  runner owns which layer.

## Skills (`.claude/skills/`, symlinked into `~/.claude/skills/` on the WSL box)
`step-by-step` (one step per message when Samir drives by hand; M1 runs
as a loop instead, see `context/progress/now.md`) · `dz-context` · `dz-money` · `dz-review` · `dz-pr` ·
`dz-mockup` · `dz-standup` · `laptop-dev` · `git-commit-convention` · `dont-sound-like-ai`.
User-level `stale-check` (any repo) reads `.claude/stale-homes.md` here for
where each duplicated fact lives and how a fix is routed.

## Agents (`.claude/agents/`)
`dz-builder` (ordinary work), `dz-money-builder` (anything touching centimes,
the schema or a printed total), `dz-review-centimes` / `dz-review-reach` /
`dz-review-tests` (the three `dz-review` lenses, which find but never rule),
`dz-review-prover` (settles one lens's findings with a quoted line or a
run; never the lens that found them),
`dz-verifier` (runs named commands and reports the output, no judgement).
Each file's frontmatter decides its model; a caller may override it, which is
how the whole-loop review runs the lenses on a sharper model.

## Non-negotiables (repeated here because they are cheap to forget)
- Money is integer centimes, checked arithmetic, no `f64` near a total.
- No `unwrap` / `expect` in shipped code (clippy denies it). No TS `as`
  type assertions (`as const` and `satisfies` are fine).
- Branch + PR for code, `ctx:` trailer in the trailer block; only `context/`
  bookkeeping goes straight to `main`.
- Never excuse a failure as already present before your change; state the
  actual root cause.
- On the WSL box, `df -h /` lies (it is a VHDX on Windows `C:`). Check
  `df -h /mnt/c`, or just `just disk`, before any heavy build. Tearing down
  a worktree means `just worktree-rm <name>`, never a bare `rm -rf` of the
  tree — worktrees hold uncommitted work; only `target/` is disposable.
- Disk, after the 125 GB day (2026-09-10): every cargo command in this repo
  runs through `just` (`just clippy`, `just test`, `just types`, `just api`,
  `just e2e`), which exports the one shared build folder
  (`<main checkout>/.cargo-target`, four jobs, one cargo run at a time) and first runs
  `just claim`: cargo names our three crates' artifacts the same in every
  worktree and trusts mtimes, so a bare `cargo` in a worktree after another
  worktree built silently reuses the other branch's crates. `just claim`
  before any bare `cargo`, and export `CARGO_TARGET_DIR` into that shell
  too: `just claim` alone does not export it there
  (`context/processes/20260908-machines-and-heavy-jobs.md`). Never a
  per-worktree `target/`. A worktree is torn down with
  `just worktree-rm` the moment its branch merges, never moved around to
  keep a warm cache. `df -h /mnt/c` before any build; under 20 GB free,
  clean first. One Claude session per conversation: a second
  `claude --continue` on the same transcript kills the first one's agents.
- Never `pkill -f`.
