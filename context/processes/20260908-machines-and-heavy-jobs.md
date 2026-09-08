---
title: 'Machines and heavy jobs'
slug: 'machines-and-heavy-jobs'
status: 'active'
category: 'processes'
created: 20260908
tldr: 'WSL box vs laptop, the shared-box claim rule, no worktrees while one session per machine'
---
# Machines and heavy jobs

Two machines. The `laptop-dev` skill has the loop between them.

| | `fedora-wsl` | `fedora` laptop |
|---|---|---|
| Role | code, tests, CI-equivalent gates | anything with a window: Tauri, Expo, a browser Samir clicks |
| Shared with | several ObserveOne sessions | nobody |
| Heavy jobs | claim the box first | build freely |

## The shared-box rule
WSL crashed on 2026-08-20 with three sessions building at once and took
every background agent with it. So on `fedora-wsl`:
- Heavy: a Tauri `cargo build` or `tauri dev`, a `pnpm build` of the
  desktop app, anything `--coverage`. `ListAgents`, announce it with a
  rough duration, wait for a clear, say when it is done.
- Not heavy, just run it: `cargo test` on `crates/core`, `cargo clippy`,
  `cargo fmt`, vitest, `tsc`.
- One heavy job at a time, foreground, and batch your own.

## Sessions and worktrees
One session per machine works this repo, so branches in the checkout are
enough; no worktree ceremony. If two sessions ever work it on the same box,
move to `.claude/worktrees/<name>` the way the ObserveOne repos do.

## Dev servers
Nothing runs by default. The web UI (`pnpm desktop dev --host`) can run on
either machine; the native window only on the laptop. Say which servers
you started and stop them when done; use a pid file or `fuser -k
<port>/tcp`, never `pkill -f` in a chained command (it matches the shell
running it).

