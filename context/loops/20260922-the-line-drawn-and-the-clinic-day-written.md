---
title: 'The line drawn, and the clinic day written'
slug: 'the-line-drawn-and-the-clinic-day-written'
status: 'active'
category: 'loops'
created: 20260922
tldr: 'The loop after the module-shape brainstorm. Anouar confirmed on 2026-09-22 the shape the brainstorm recommended, a module bringing its own screens, permissions and tables without touching the core, and Samir asked him whether that happens at build time or at runtime; that answer is pending and nothing here depends on it. Three things do not: the brainstorm result goes on the status site as its own section, the boundary between the shared core and the shop becomes a rule the tests enforce (D4), and a first written profile of a second trade day (D2) gets drafted from desk research so the paper test (D3) has something to tear. Then a standup.'
roadmap: 'catching-lumina'
---
# The line drawn, and the clinic day written

Samir, 2026-09-22, 10:04: "do a loop please, also put the result of the debate
somewhere in the reports website as its own section."

What was said this morning, in the people's own words, because a loop that
paraphrases them is the loop that puts words in their mouths:

- Anouar, 2026-09-21, from the decision plan: keep the core, add a module per
  trade, doctors first, the way Odoo does it; and the same morning, "the
  really important thing for us is to make it so modular that adding new
  modules to it wont break the logic."
- Anouar, 2026-09-22, 08:52, on the brainstorm's summary: "yes module bringing
  its own screens, permissions and tables, without ever touching the core's,
  this is what we want."
- Samir to Anouar, 2026-09-22, 08:57: "is it at runtime?? or at build time,
  so are we letting users customize it in real time, or we build each one
  ourselves." Unanswered when this file was written. Samir's own ruling of
  2026-09-21 stands until then: no code is loaded into the running program.

## What this loop does not decide

The shape (D6). It waits on D2, D3 and D5 and on Anouar's runtime answer.
Every phase below is the same work whichever way he answers, which is why it
can start now.

## Phase A: the brainstorm on the status site, as its own section

A new sidebar group on `~/.dz-night/report/`, not a row under Reference,
because Samir asked for a section. One page, plain words for Anouar: what he
asked in his words, what was compared and how (seven cases argued one by one
against the same constraints, then compared; not two agents head to head),
the two facts that decided most of it, what Odoo really does under the hood
and what copying it would cost, what stands today and what is open. No task
codes, no crate or trait names without a plain gloss. The home board's
"being built now" line also moves on, since the cash drawer merged yesterday.
Deploy, verify on the preview URL, commit and push `Dinar-dz/dinar-reports`.

## Phase B: the boundary as a rule (D4)

`crates/core/tests/services_go_through_services.rs` gains a sibling walk:
nothing in users, sessions, permissions, audit, settings, preferences, pairing,
backup or the money kernel may name a product, a sale, a supplier, a stock
movement or a customer fiche. The plan's wording fails on main today because
the shop already lives in six kernel places, each read again on 2026-09-21 by
two of the seven brainstorm pages: the error enum, the permission enum, nine
of the ten column enums, `print::number`, the shared migration list, and two
raw counts in backup and the support bundle. So the test carries those six as
a named allow list with file and line, the way `RINGS_STILL_OPEN` carried the
rings, and fails on a seventh. No file moves, no crate created. dz-builder on
Sonnet, own worktree, Sonnet lenses, prover, gates with the `.owner` check,
PR, merge, `just worktree-rm`.

## Phase C: a second trade's day on paper (D2)

The plan assigns D2 to Samir's own research; Anouar said today the trade need
not be a doctor. This phase drafts it rather than waits: one research page in
`context/research/`, desk research with sources named, in the shape of
`docs/features.md` rows (what happens, in what order, who touches it, what is
printed, what is owed and by whom) for a doctor's cabinet in Algeria, the case
Anouar named first. Written as "a cabinet would", never as a known customer.
Samir's own research confirms or replaces it; the page says so at the top.
Sonnet, one agent. D3, the paper test, follows in this loop only if D2 lands
with a billed consultation concrete enough to express in the document model.

## Phase D: standup

The day's standup on the site, in the usual shape, linking the new section.

## Left out on purpose

- The disk cleanup (about 40 GB of build leftovers). Waits on Samir's word;
  deleting a cache is not part of "do a loop".
- The phone's one remaining type assertion in its API client. Its own comment
  says removing it means a schema per DTO and points at the architecture
  plan; it is not the small fix it was called this morning.
- The refund design question, the barcode scanner check on Samir's phone, the
  Windows 7 confirmation with a real shop: people, not this loop.

## Guardrails

The ones yesterday's loop lists, unchanged: every cargo command through
`just`, `just claim` before any bare cargo, `df -h /mnt/c` before a heavy
build, the `.owner` check before a gates run counts, never more than four
background agents, Sonnet lenses per PR, the prover proves and the session
rules on design calls only, one PR per unit of work, `just ci` once after the
day's last merge.
