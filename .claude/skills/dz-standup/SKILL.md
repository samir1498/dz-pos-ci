---
name: dz-standup
description: Write or rebuild a daily standup page on the Dinar status site. Use when Samir says "standup", "daily standup", or asks what to show Anouar today. Covers the page rules (plain words, no codes, three shots), the rebuild, the deploy, and committing the site repo.
---

## Source of truth

The site generator lives at `/home/samir/.dz-night/report/`
(`reports/`, `reference/`, `screens/`, `make-index.py`, `make-screens.py`,
`deploy.sh`). It is its own git repo, `Dinar-dz/dinar-reports` (private):
every change there ends committed and pushed, like any other repo. The
home page is a status board rendered from the `STATUS` block at the top
of `make-index.py` plus the newest dated report, which features itself
automatically.

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
the previous standup. Copy the `<head>` (favicon, OG/Twitter card) from
the previous standup too. File name: `reports/YYYY-MM-DD-daily-standup.html`.

## Ship

From `/home/samir/.dz-night/report/`: `python3 make-index.py` (the new
page becomes the featured latest on the home board), `./deploy.sh`, then
verify on the preview URL the deploy prints. Production
(`https://dinar-reports.pages.dev/`) sits behind Cloudflare Access, so
the preview URL is the verification path and the link to share with
anyone outside the Access policy. Then commit and push
`Dinar-dz/dinar-reports`.

Dated reports are history: never rewrite a past standup's body to match
today. If the milestone count moved, the place to update is the `STATUS`
block and the reference pages, which is a stale-check sweep, not an edit
to old news.
