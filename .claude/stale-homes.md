# Where facts live (read by the `stale-check` skill)

## Homes

| Fact | Home | Pointers allowed |
|---|---|---|
| The gate list | `justfile` (`gates` recipe) | prose says `just gates`, never the list |
| Commands to run | `justfile` | pages name the recipe |
| Routes and what they need | `crates/api/src/lib.rs`; `docs/architecture.md` § Transport and auth | `apps/desktop/e2e/README.md` |
| Milestone order | `docs/roadmap.md` | `context/roadmaps/` entry, `context/progress/now.md` |
| Milestone status and tasks | `context/roadmaps/` entry (status), the milestone plan in `context/plans/` (tasks) | `now.md` Active paragraph |
| What is in flight | `context/progress/now.md` and `~/.dz-night/ledger.md` on the WSL box | none |
| Fiscal rules | `docs/features.md` table | fixtures by name |
| Branch shape | `context/processes/20260908-git-and-planning.md` | `dz-pr` skill |
| The boss site URL | `context/progress/now.md`, `CLAUDE.md` | memory file |
| Skills list | `.claude/skills/` directory | `CLAUDE.md` index line |

## Routing

- `context/**`: straight to `main`, one commit, `chore(context):` subject,
  `ctx:` trailer; plan task statuses through `plan_task_status` when the
  tool is available.
- Everything else in the repo: a branch and a PR (inside a milestone, a
  task branch off the milestone branch, merged after `just gates`).
- The report site (`~/.dz-night/report/`): edit, then `./deploy.sh`. Boss
  wording, current state only.
