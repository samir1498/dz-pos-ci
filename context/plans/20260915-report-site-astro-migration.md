---
title: 'Report site Astro migration'
slug: 'report-site-astro-migration'
status: 'paused'
category: 'other'
created: 20260915
tldr: 'Move ~/.dz-night/report/ off hand-written HTML + python generators onto Astro (like apps/landing); parked until product work allows'
tasks:
  - id: 'T1'
    desc: 'Inventory: every page, generator behavior (make-index.py, make-screens.py), shared style, OG/favicon, preview dialog, deploy.sh flow'
    status: 'pending'
  - id: 'T2'
    desc: 'Scaffold Astro for the report site, deploying to the same dinar-reports Pages project'
    status: 'pending'
  - id: 'T3'
    desc: 'Port shared layout + style (Fraunces/Plex, dark mode) to Astro layouts; no new JS libs'
    status: 'pending'
  - id: 'T4'
    desc: 'Reports as a content collection with identical URLs (links to dated standups/checkpoints are shared and must not break)'
    status: 'pending'
  - id: 'T5'
    desc: 'Reference pages, screens gallery (26 shots, fr/ar), landing/design embeds'
    status: 'pending'
  - id: 'T6'
    desc: 'Index dashboard from a data file replacing the STATUS block in make-index.py'
    status: 'pending'
  - id: 'T7'
    desc: 'URL-compat check old-vs-new, deploy, verify on preview URL and behind Access'
    status: 'pending'
  - id: 'T8'
    desc: 'Retire make-index.py/make-screens.py, update dz-standup skill + CLAUDE.md references'
    status: 'pending'
acceptance: []
---
# Report site Astro migration (parked)

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
