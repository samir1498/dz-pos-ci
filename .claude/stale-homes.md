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
| Core module layers | `crates/core/src/lib.rs` mod list | `docs/architecture.md` § Layers inside core |
| Golden-file tooling | `crates/core/tests/print_ticket.rs` (`goldens_dir`, `golden`, `UPDATE_GOLDENS`) | `docs/architecture.md` Testing matrix, `docs/features.md` §4 |
| e2e spec list | `apps/desktop/e2e/*.spec.ts` | `apps/desktop/e2e/README.md` § Files |
| Who opens and merges PRs | `docs/roadmap.md` § Every milestone | `dz-pr` skill, the boss site's How the work runs page |
| Milestone status tags in `docs/roadmap.md` headings and on the boss reference pages (roadmap, how-the-work-runs, file-structure, decisions) | the roadmap entry in `context/roadmaps/` | the sweep at each checkpoint rewrites them; nothing else may |
| Committed e2e screenshots | `apps/desktop/e2e/README.md` § screenshots | the `justfile` screenshot comment points there |
| Which CI jobs run on a pull request and which only on `main` | `.github/workflows/ci.yml` and the `justfile` `ci` recipe | `README.md` Quality gates, `docs/architecture.md` Testing matrix name the split, never restate the per-job conditions |
| Theme default and what happens when a shop has never chosen one | `docs/architecture.md` § Design | `docs/features.md` §8 names the default (Comptoir) and points here for the mechanism |

## Routing

- `context/**`: straight to `main`, one commit, `chore(context):` subject,
  `ctx:` trailer; plan task statuses through `plan_task_status` when the
  tool is available.
- Everything else in the repo: a branch and a PR (inside a milestone, a
  task branch off the milestone branch, merged after `just gates`).
- The report site (`~/.dz-night/report/`): edit, then `./deploy.sh`. Boss
  wording, current state only.
