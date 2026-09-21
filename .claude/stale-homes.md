# Where facts live (read by the `stale-check` skill)

## Homes

| Fact | Home | Pointers allowed |
|---|---|---|
| The gate list | `justfile` (`gates` recipe) | prose says `just gates`, never the list |
| Commands to run | `justfile` | pages name the recipe |
| Routes and what they need | `crates/api/src/router.rs`; `docs/architecture.md` § Transport and auth | `apps/desktop/e2e/README.md` |
| Milestone order | `docs/roadmap.md` | `context/roadmaps/` entry, `context/progress/now.md` |
| Milestone status and tasks | `context/roadmaps/` entry (status), the milestone plan in `context/plans/` (tasks) | `now.md` Active paragraph |
| What is in flight | `context/progress/now.md` and the newest page in `context/loops/` | none |
| Fiscal rules | `docs/features.md` table | fixtures by name |
| Branch shape | `context/processes/20260908-git-and-planning.md` | `dz-pr` skill |
| The boss site URL | `context/progress/now.md`, `CLAUDE.md` | memory file |
| How long a file may be, and which files are already over it | `scripts/file-sizes.mjs` (the limits) and `scripts/file-sizes.json` (the pinned list) | prose says `just sizes`, never the numbers or the list |
| Skills list | `.claude/skills/` directory | `CLAUDE.md` index line |
| Agent list and which model each runs | `.claude/agents/*.md` frontmatter | `CLAUDE.md` index line, the `dz-review` skill names the lenses but not their models |
| Core module layers | `crates/core/src/lib.rs` mod list | `docs/architecture.md` § Layers inside core |
| Golden-file tooling | `crates/core/tests/print_ticket.rs` (`goldens_dir`, `golden`, `UPDATE_GOLDENS`) | `docs/architecture.md` Testing matrix, `docs/features.md` §4 |
| e2e spec list | `apps/desktop/e2e/*.spec.ts` | `apps/desktop/e2e/README.md` § Files |
| Who opens and merges PRs | `docs/roadmap.md` § Every milestone | `dz-pr` skill, the boss site's How the work runs page |
| Milestone status tags in `docs/roadmap.md` headings and on the boss reference pages (roadmap, how-the-work-runs, file-structure, decisions) | the roadmap entry in `context/roadmaps/` | the sweep at each checkpoint rewrites them; nothing else may |
| Committed e2e screenshots | `apps/desktop/e2e/README.md` § screenshots | the `justfile` screenshot comment points there |
| Where CI runs and what it runs | `.github/workflows/ci.yml` and the `justfile` `ci` recipe | `README.md` Quality gates, `docs/architecture.md` Testing matrix name the split, never restate the per-job conditions |
| Theme default and what happens when a shop has never chosen one | `docs/architecture.md` § Design | `docs/features.md` §8 names the default (Comptoir) and points here for the mechanism |
| Desktop screens list | `apps/desktop/src/routes/*.tsx` | `README.md` § Screens |
| Gated-read count and which routes are closed outright vs. field-redacted | `crates/api/src/gates/mod.rs` module doc, counted off `ROUTE_GATES` | `docs/features.md` §5, `context/progress/now.md` name examples, never restate the count |
| How many permissions there are, which role holds each, and which are a coarse route gate paired with a fine service check | `crates/core/src/services/permissions.rs` (`Permission::ALL` for the count, `can` for who holds what, each variant's own doc for whether a route or a service asks it) | `docs/features.md` §5 restates all three in prose and moves with them; `crates/api/src/dto/auth.rs` and `packages/shared/src/schemas/session.ts` mirror the list and the compiler holds them to it; `crates/api/src/gates/table.rs` names only the coarse half, never the fine one |
| API error codes and what each carries | `crates/api/src/error.rs` (`ApiError::parts`, `Figures`) and `crates/core/src/error.rs` (`CoreError::code`) | `docs/architecture.md` error-code table |
| Committed landing shots (which files exist) | `apps/landing/src/lib/shots.ts` (manifest, `buildShots()`) | `public/shots/*.webp`, the e2e PNGs they compose |
| E2E test-id inventory | `data-testid` attributes in `apps/desktop/src` and the specs that click them | e2e README § test ids |
| Report home status (stamp, holds, featured) | `STATUS` block in `make-index.py` (`Dinar-dz/dinar-reports`) | generated `index.html` |
| A shift's expected figure (what it is made of, and that it is stored rather than derived) | `crates/core/src/services/shifts.rs`, the arithmetic in `close` and `report` | `docs/features.md` §1 restates it and moves with it, the `shifts` migration's own comment names why it departs from architecture.md rule 4 |

## Routing

- `context/**`: straight to `main`, one commit, `chore(context):` subject,
  `ctx:` trailer; plan task statuses through `plan_task_status` when the
  tool is available.
- Everything else in the repo: a branch and a PR (inside a milestone, a
  task branch off the milestone branch, merged after `just gates`).
- The report site (`~/.dz-night/report/`): edit, then `./deploy.sh`. Boss
  wording, current state only.
