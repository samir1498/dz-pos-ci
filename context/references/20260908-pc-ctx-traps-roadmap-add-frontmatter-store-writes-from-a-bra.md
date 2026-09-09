---
title: 'pc-ctx traps: roadmap add frontmatter, store writes from a branch'
slug: 'pc-ctx-traps-roadmap-add-frontmatter-store-writes-from-a-bra'
status: 'active'
category: 'references'
created: 20260908
tldr: 'ctx roadmap add omits category and created; plan_validate fails until added by hand. Context writes land on whatever branch the main checkout has.'
---
# pc-ctx traps seen on 2026-09-08

- `ctx roadmap add` writes a roadmap file without `category` and
  `created` in the frontmatter, and `plan_validate` then reports two
  errors. Add `category: 'roadmap'` and `created: YYYYMMDD` by hand after
  creating a roadmap. `ctx plan add` does not have this gap.
- `ctx plan validate <slug>` is not a command; validation is
  `plan_validate` through the MCP (whole store) with no CLI equivalent
  found.
- The MCP is bound to `/home/samir/dz-pos/context`, which is the main
  checkout. When that checkout sits on a feature branch, every
  `progress_log`, `references_add` and plan edit lands on that branch.
  Either commit them as separate `chore(context)` commits and cherry-pick
  to `main`, or switch the checkout to `main` before writing.
- Do not share one `CARGO_TARGET_DIR` between worktrees of this repo.
  Cargo keys artifacts of a path crate by name and version, not by path,
  so `dzpos-core` built from one worktree overwrites the other's and a
  later `cargo clippy` links tests against a stale library ("no variant
  RateOutOfRange" on a source that has it). Seen 2026-09-08; cleared with
  `cargo clean -p dzpos-core`. Each worktree gets its own `target/`
  (`CARGO_TARGET_DIR=$PWD/target`), with `CARGO_BUILD_JOBS=3` when three
  builds may run at once on the 12-core, 11 GB box.

