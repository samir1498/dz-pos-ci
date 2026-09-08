# Context System

A shared folder for planning, progress, ideas, and references.
Uses `pc-ctx` CLI for deterministic plan/roadmap management.

## MCP tools (preferred)
All plan/roadmap/research operations have MCP tools. Use them over raw CLI:
- `plan_list` / `plan_show` / `plan_status` / `plan_validate`
- `plan_set_status` / `plan_task_status` / `plan_add` / `plan_add_task`
- `plan_references` — refs + backlinks
- `roadmap_list` / `roadmap_show`
- `research_list` / `research_show`

Fall back to `ctx <subcommand>` if MCP tools are unavailable.

## Quick start
- `ctx status` — overview
- `ctx list` — all plans
- `ctx show <slug>` — plan details
- `ctx plan add <title>` — new plan
