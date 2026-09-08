---
title: 'Agent worktree isolation starts from main, not the current branch'
slug: 'agent-worktree-isolation-starts-from-main-not-the-current-br'
status: 'active'
category: 'references'
created: 20260908
tldr: 'Agent tool with isolation worktree checks the worktree out from main; tell agents the base branch and make them rebase first'
---
# Agent worktrees start from `main`

Seen 2026-09-08 during the night build. `Agent(isolation: "worktree")`
creates `.claude/worktrees/agent-<id>` checked out from `main`, whatever
branch the main checkout is on. Five agents briefed to build on
`core/money-newtype` started without the Money module.

Fix in the dispatch prompt: the first command an agent runs is
`git fetch origin <base> && git reset --hard origin/<base>` (or a rebase
when it already committed), and the report must say it did. Push the base
branch before spawning, since the agent fetches it from origin.

