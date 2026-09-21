---
title: 'Git and planning'
slug: 'git-and-planning'
status: 'active'
category: 'processes'
created: 20260908
tldr: 'Branch + PR, ctx trailer, the context store and session rituals'
---
# Git and planning

## Git
- Branch off `origin/main` (`feature/`, `fix/`, `chore/`, `docs/`) and
  open a PR with the `dz-pr` skill. Inside a milestone, task branches
  (`m1/<task>`) merge into the milestone branch (`m1/2026-09-09`) after
  gates and a review, and the milestone branch goes to `main` by PR at
  its checkpoint tasks. Code never lands on `main` without a PR.
- `context/` bookkeeping (plan status, progress entries, handoffs, a
  reference page) commits straight to `main` with a `chore(context):`
  subject, the way `observeone-context` pushes to its own main. A PR per
  progress line is friction nobody reads.
- WIP commits on a branch are fine; the laptop pulls them. Squash at merge.
- One PR per plan task or coherent change, never per commit (Samir,
  2026-09-21). A review fix, a missing test, a follow-up screen or a fix
  for what the same task broke rides on the task's branch; two tasks of
  one plan touching the same files ship together. CI minutes are spent
  only on the mirror through `just ci`: run it once after the day's last
  merge, and ask for the Windows job only for a path, file or OS change,
  batched with the next full run. The `dz-pr` skill has the full rule.
- Never `git reset --hard`, `git clean -fd` or force-push without asking.
- Every commit carries `ctx: <plan-slug>[/T<n>] <start|progress|close>`
  in the same block as the other trailers; `.githooks/commit-msg` rejects
  it otherwise. The `git-commit-convention` skill has the actions.

## Planning
`context/` is the pc-ctx store: `plans/`, `progress/`, `handoffs/`,
`references/`, `processes/` (these pages), `repos/`. The `dz-context`
skill says which front door to use (MCP when the session cwd is this repo,
`ctx` from inside `context/` otherwise) and what a handoff opens with.
Never edit plan files by hand.

Session start: `ctx stale`, `ctx reconcile --dry`, read `progress/now.md`.
Session end: `ctx reconcile --apply`, a progress entry, a handoff if the
next session is on the other machine.

