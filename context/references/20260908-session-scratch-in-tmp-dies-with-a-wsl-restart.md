---
title: 'Session scratch in /tmp dies with a WSL restart'
slug: 'session-scratch-in-tmp-dies-with-a-wsl-restart'
status: 'active'
category: 'references'
created: 20260908
tldr: 'Keep the night-build ledger, briefs and review diffs under ~/.dz-night, not the /tmp scratchpad; WSL restarts wipe /tmp and kill running agents'
---
# Session scratch in /tmp dies with a WSL restart

Seen 2026-09-08 20:53. Samir restarted WSL; the Claude process, three
running subagents and the whole `/tmp/claude-1000/.../scratchpad` went
with it: the build ledger, the six track briefs, the contracts file, the
review diffs and the agent reports. Branches survived because every
track pushed after each step; one uncommitted edit in a worktree was
recovered with `git diff` before the worktrees were removed.

Rules that follow:
- Ledger, briefs and anything a resumed session needs go under
  `~/.dz-night/` (or another path under `$HOME`), never `/tmp`.
- Agents push after every step; a worktree is disposable.
- Agent worktrees carry their own `target/` (8 GB for the API track);
  remove them with `git worktree remove --force` and `git worktree
  prune` when the track is merged, then delete `worktree-agent-*`
  branches.
- The agents' transcripts survive under `~/.claude/projects/...` and can be
  resumed by id only while their worktree still exists.

