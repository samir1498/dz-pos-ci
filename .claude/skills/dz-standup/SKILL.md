---
name: dz-standup
description: Write or rebuild a daily standup page on the Dinar status site. Use when Samir says "standup", "daily standup", or asks what to show Anouar today. Covers the page rules (plain words, no codes, three shots), the rebuild, the deploy, and committing the site repo.
---

## Source of truth

The site lives at `/home/samir/.dz-night/report/`. It is an Astro site
(`src/pages/`, `src/legacy/`, `src/data/`, `public/`, `deploy.sh`) and its
own git repo, `Dinar-dz/dinar-reports` (private): every change there ends
committed and pushed, like any other repo. The home page is a status board
rendered from `src/data/status.ts` plus the newest dated report, which
features itself automatically. Every page sits in one docs shell with a
grouped sidebar; the sidebar is built from the data, so a new report turns
up in it on its own.

## What to gather first

Read `context/progress/now.md` (Active + Done recently), `git log
--oneline -10` in `dz-pos`, and the milestone plan's task states. The
standup reports what moved and what holds it, not the diff: one line per
thing a shop owner would notice.

## Page rules

Plain words for Anouar, who is not in the code: no milestone or task
codes in the body (`M6`, `T9`), no route or constant names, no em dashes.
Short — what moved, what it means, what is next. Holds name Samir, never
Anouar. At most three inline screenshots (`screens/*.png`), each wrapped
in a new-tab link plus the native `<dialog>` preview snippet copied from
the previous standup. The layout supplies the `<head>`, so the
page is a body fragment, not a whole document.

A new standup is one file, `src/legacy/reports__YYYY-MM-DD-daily-standup.html`,
holding an optional `<style>` block and the body, plus one row in
`src/data/legacy.json` giving its title, summary, date and its URL
`/reports/YYYY-MM-DD-daily-standup.html`. Copy the previous standup's
fragment and edit it; that keeps the house style without copying a head
that no longer belongs to the page.

## Ship

From `/home/samir/.dz-night/report/`: `./deploy.sh`, which pulls the
screenshots, runs `astro build` and publishes `dist/` (the new page becomes
the featured latest on the home board on its own). Then verify on the
preview URL the deploy prints. `pnpm exec astro dev` serves it locally
while writing. Production
(`https://dinar-reports.pages.dev/`) sits behind Cloudflare Access, so
the preview URL is the verification path and the link to share with
anyone outside the Access policy. Then commit and push
`Dinar-dz/dinar-reports`.

Dated reports are history: never rewrite a past standup's body to match
today. If the milestone count moved, the place to update is
`src/data/status.ts` and the reference pages, which is a stale-check sweep, not an edit
to old news.
