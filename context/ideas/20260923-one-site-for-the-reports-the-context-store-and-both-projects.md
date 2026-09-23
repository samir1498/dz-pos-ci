---
title: 'One site for the reports, the context store and both projects'
slug: 'one-site-for-the-reports-the-context-store-and-both-projects'
status: 'active'
category: 'ideas'
created: 20260923
tldr: 'Samir, 2026-09-23: later, merge the pc-ctx web view and the Dinar reports site into one site that carries both projects (Dinar and ObserveOne); consolidating tooling is part of why the monorepo started.'
---
# One site for the reports, the context store and both projects

Samir, 2026-09-23 18:10, while asking for the day's report: "i plan to merge
pcctx web and reports website and put both projects in it but this is for
the future ... i want to consolidate a ton of stuff if possible, this is why
i started the monorepo, or it was an inspiration."

Today there are two separate sites:

- the Dinar reports site, an Astro site at `~/.dz-night/report/` (repo
  `Dinar-dz/dinar-reports`), published to dinar-reports.pages.dev behind
  Cloudflare Access;
- the pc-ctx web view of the `context/` store (plans, research, handoffs).

The idea is one site that holds both, for both projects (Dinar and
ObserveOne): the standups and status board beside the plans and research
they come from.

Not started, and no plan yet. Questions for when it starts:

- where it lives: in the monorepo or its own repo;
- who may see what: Anouar sees the reports today but not ObserveOne;
- whether the context pages are rendered at build time or read live.

