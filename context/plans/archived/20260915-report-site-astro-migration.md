---
title: 'Report site Astro migration'
slug: 'report-site-astro-migration'
status: 'done'
category: 'other'
created: 20260915
tldr: 'Done 2026-09-20: ~/.dz-night/report/ is an Astro site in one docs shell; all 39 URLs verified unchanged on the live deploy'
tasks:
  - id: 'T1'
    desc: 'Inventory: every page, generator behavior (make-index.py, make-screens.py), shared style, OG/favicon, preview dialog, deploy.sh flow'
    status: 'done'
  - id: 'T2'
    desc: 'Scaffold Astro for the report site, deploying to the same dinar-reports Pages project'
    status: 'done'
  - id: 'T3'
    desc: 'Port shared layout + style (Fraunces/Plex, dark mode) to Astro layouts; no new JS libs'
    status: 'done'
  - id: 'T4'
    desc: 'Reports as a content collection with identical URLs (links to dated standups/checkpoints are shared and must not break)'
    status: 'done'
  - id: 'T5'
    desc: 'Reference pages, screens gallery (26 shots, fr/ar), landing/design embeds'
    status: 'done'
  - id: 'T6'
    desc: 'Index dashboard from a data file replacing the STATUS block in make-index.py'
    status: 'done'
  - id: 'T7'
    desc: 'URL-compat check old-vs-new, deploy, verify on preview URL and behind Access'
    status: 'done'
  - id: 'T8'
    desc: 'Retire make-index.py/make-screens.py, update dz-standup skill + CLAUDE.md references'
    status: 'done'
acceptance: []
---
# Report site Astro migration

The status site (`~/.dz-night/report/`, live at
https://dinar-reports.pages.dev/) is hand-written HTML plus two python
generators. It works but every redesign means editing string templates,
and the standup/index/reference pages share no components. `apps/landing`
already proves Astro fits this project.

Parked on 2026-09-15 by Samir: product work first ("focus back on the
product"). Resume by setting this plan `active` and starting T1.

## Constraints (carry into the build)

- URLs are shared (standups, checkpoints go to Anouar): every existing
  `.html` path must serve the same content after the move. Redirects only
  as a last resort, never for dated reports.
- Same visual language, same no-JS-lib rule (the `<dialog>` preview trick
  stays vanilla).
- The report folder lives in `Dinar-dz/dinar-reports` (private, created
  2026-09-15); commit and push there with every site change.
- Production sits behind Cloudflare Access now; preview URLs stay the
  verification path.

## Done, 2026-09-20

Every task landed in one pass, in `Dinar-dz/dinar-reports` commit `12821b7`.

- The hand-written pages are fragments under `src/legacy/`, rendered by one
  catch-all route. Their bodies are byte for byte what was written; the
  dated reports went to Anouar with that wording and a migration is not the
  moment to reword them.
- All 39 URLs were diffed old against new, then checked one by one against
  the live preview deploy. Every one lands on the same page. `build.format:
  "file"` is what keeps the `.html` endings, and the three directory indexes
  are one-path `[index].astro` routes because an `index.astro` would have
  been written as `reports.html`.
- The shell is the docs shape Samir asked for on the day: grouped sidebar
  built from the data, filter box, breadcrumb, contents rail. The chip rows
  went, which was the point: navigation by tags.
- `make-index.py`, `make-lumina.py`, `make-shell.py` and `site_style.py` are
  deleted. `make-screens.py` kept only its copying half as
  `scripts/pull-screens.py`. The `dz-standup` skill now says how to add a
  standup to the Astro site.

One thing the plan asked for that was overtaken: it said to keep the same
visual language and no new JS libs. The palette and the fonts are unchanged
and there is still no JS library, but the layout is new, because Samir asked
for the ObserveOne docs shape while the migration was being built. Starlight
was the obvious open-source fit and was turned down for one reason: it owns
its own routing and would not serve `/reports/2026-09-16-daily-standup.html`.
