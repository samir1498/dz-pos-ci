---
name: dz-context
description: Read or update the dz-pos planning store in `context/` (plans, tasks, progress, handoffs, references). Use whenever a dz-pos session starts, ends, finishes a task, learns a durable fact about the repo, or Samir asks "where are we", "what's next", "write a handoff", "log that".
argument-hint: [now|plans|update <msg>|handoff|fact <text>]
---

`context/` is a pc-ctx store that lives inside the dz-pos repo (no separate
git). Plans, progress and handoffs are the memory that survives a session;
if it is not written there it is gone when the terminal closes.

## Which tool reaches it

Two front doors, and only one of them works from a given session:

- **Session cwd is the dz-pos repo** (laptop, or `claude` started in
  `/home/samir/dz-pos`): the `pc-ctx` MCP in `.mcp.json` is bound to this
  store. Use `plan_*`, `progress_log`, `handoffs_add`, `references_add`.
- **Session cwd is somewhere else** (the observeone workspace session that
  bootstrapped this repo): the MCP is bound to *that* workspace's store, and
  `plan_add` there writes to the wrong repo. Use the CLI instead, from inside
  the store: `cd /home/samir/dz-pos/context && ctx <cmd>`. Check which one
  you are in before the first write; `ctx plan list` from `context/` shows
  the dz-pos plans, `plan_list` from the MCP shows whichever store it holds.

CLI shapes that differ from the MCP: `ctx plan add-task <slug> <id> <text>
<status>` needs the fourth positional (`pending`), `ctx plan activate
<slug>`, `ctx show <slug>`, `ctx plan task-status <slug> <id> done`.

## Behaviour

- **no args / "now"**: read `progress/now.md` and the active plans
  (`ctx plan list --status active`); say what is in progress and what is
  next in two or three sentences.
- **"plans"**: list plans as title then done/total. No slugs, no file names.
- **"update <msg>"**: `progress_log` (or `ctx progress add`) with the
  message, then refresh `progress/now.md` if the priority changed.
- **"handoff"**: `handoffs_add` with the resume block below first, then
  state, decisions, next task, open questions. Also update
  `docs/handoff-<date>.md` when the handoff is for a different machine, since
  the laptop session reads the repo before it reads the store.
- **"fact <text>"**: a durable thing learned about the repo or its tools
  goes in `context/references/` via `references_add`. A fiscal claim does
  not go there: it goes in `docs/features.md` next to its fixture name.

## Every handoff opens with a resume block

```
## Resume this session

    claude --resume <session-uuid>

Cloud view: https://claude.ai/code/session_<id>
Machine <fedora-wsl|fedora laptop>, branch <branch>, head <sha>.
```

Session uuid: `ls -t ~/.claude/projects/<cwd-slug>/*.jsonl | head -1`. Take
the newest by mtime; a compacted session keeps its id.

## Commit trailer

Every commit carries `ctx: <plan-slug>[/T<n>] <start|progress|close>`; the
`.githooks/commit-msg` hook rejects commits without one. The
`git-commit-convention` skill has the actions and the trailer-block trap.

## Prose

Load `dont-sound-like-ai` before writing a handoff, a plan body or a
progress entry. Samir reads these to decide what to do next; a handoff that
narrates ("was broken, then fixed") leaves a skimmer believing it is broken.
State what is true now.
