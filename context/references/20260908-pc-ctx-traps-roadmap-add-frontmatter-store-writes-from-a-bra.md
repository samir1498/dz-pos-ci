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
- Worktrees for parallel agents share one Rust build with
  `CARGO_TARGET_DIR=/home/samir/dz-pos/target`; cargo's directory lock
  serialises builds, so "Blocking waiting for file lock on build
  directory" is normal, not a hang.

