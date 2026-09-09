---
title: 'pc-ctx loops domain'
slug: 'pc-ctx-loops-domain'
status: 'active'
category: 'ideas'
created: 20260909
tldr: 'A loops category (plan in frontmatter, ctx loop show with live task statuses, a tick entry type) so context/loops/*.md is a first-class page in the CLI and the MCP'
---
# pc-ctx loops domain

`context/loops/20260909-m1-loop.md` is the first page of a new category the
CLI and the MCP do not know: a loop is a plan run by agents with the session
as manager. What pc-ctx needs so it stops being a hand-read page:

- `loops` category with `plan: <slug>` in the frontmatter, validated.
- `ctx loop show <slug>`: the waves table with each task's live status from
  the plan, the checks list, the tick cadence.
- A `tick` entry type (timestamp, one paragraph, log file names) so the
  ledger can move from `~/.dz-night/ledger.md` on the WSL box into
  `context/`, where the laptop clone reads it too.
- MCP tools mirroring the CLI: `loop_show`, `loop_tick`, `loop_set_status`.

Logged 2026-09-09 by the M1 loop session (Samir: "plan to update pc ctx
later for mcp and cli to support it").

