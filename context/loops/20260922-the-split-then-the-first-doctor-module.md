---
title: 'The split, then the first doctor module'
slug: 'the-split-then-the-first-doctor-module'
status: 'active'
category: 'loops'
created: 20260922
tldr: 'The build loop after the morning decided the shape. Phase A is the crate split, tasks S2 to S7 of the split plan: a kernel crate and a retail crate, the shop half of the twelve pinned files moved out, the api rows behind a feature, and a kernel build that signs a user in with no shop in it. Phase B writes and starts the first doctor module, patients and appointments together, because the clinic research found an appointment cannot ship without a patient. Phase C draws the plug points, and only then, because that is the first day two real modules exist to draw them from. The shared Claude plan was near its limit this morning, so this loop runs one builder at a time and skips the three-lens fan-out on the pure move.'
roadmap: 'catching-lumina'
---
# The split, then the first doctor module

Samir, 2026-09-22, 13:22: start the loop for the restructure and the clinic
code.

The plan this loop executes is
`context/plans/20260922-a-kernel-crate-and-retail-as-the-first-module.md`.
The target it builds towards is
`context/research/20260922-what-clinic-software-provides.md`. Anouar settled
the shape at 09:03 and 09:49: not at runtime, and we prepare each customer's
package ourselves.

## Budget, because it decides how this loop runs

Anouar, 10:38: the shared Claude plan reached 95 percent and he needs
headroom for his own work. Samir asked for the loop anyway at 13:22, which
is his call, so it runs with the cost cut where cutting it loses nothing:

- One builder at a time. Never the four the guardrails allow.
- No three-lens fan-out on Phase A. A file move's only real question is
  whether behaviour changed, and `just gates` answers that better than a
  reading lens does. The lenses come back for Phase B, which is new code.
- The session spot-reads the diff itself rather than paying an agent to
  describe it.

## Phase A: the split (S2 to S7)

S2 and S3 ship as one pull request. Two empty crates are not a unit of work
on their own, and the plan separates them only so the move is a move rather
than a move plus a build fight.

Then S4 (the twelve pinned files give up their shop half), S5 (the api rows
behind the feature), S6 (the tests move, the boundary walk rewritten
against two crates), S7 (a build with retail off that signs a user in).

Two questions inside S4 are Samir's, not a builder's, and the builder stops
rather than guessing: whether the money kernel's discount and price are
shop-only or shared, and whether the permission enum's ten shop variants
stay in the kernel as a decision (his ruling of the 21st says the list stays
one list, so the expected answer is yes and the row's reason gets rewritten).

The rule that proves Phase A finished honestly: nothing a shop can see
changes. If a screen, a total, a printed paper or a permission behaves
differently at the end of S7, something in the move was a rewrite.

## Phase B: patients and appointments, the first doctor module

Its plan page comes first, written from the clinic research, and it carries
the three questions that research left open: whether a patient is the
module's own row or a differently typed customer row, how the shared
permission gate learns a module's own verbs when each package is built
separately, and how a module's migrations sequence against the kernel's so a
package still passes the upgrade test.

The first slice is the one the research names: a patient file and an
appointment book, joined to the shared tables by plain foreign keys to the
staff list and the shop, logging to the audit trail without changing it. No
money, no fiscal question, no prescription yet.

## Where it stands, 2026-09-24 08:45

Phase A (#155 to #162) and Phase B (#163 to #174) are merged, and the
whole-loop review ran once on the stronger model: money clean, reach and
tests findings fixed in #175. A clinic build of either app ships no till,
and gates proves it on both (`check-clinic-bundle`, `check-mobile-bundle`).
Phase C below has no tasks yet; it waits on a design pass with Samir.
Open for Samir: whether "arrived" on a cancelled booking undoes the cancel.

## Phase C: the plug points

Only after Phase B has a second real module to draw them from. Nothing in
Phase A or B creates a trait, a registry or a module list.

## Out of this loop

- The logo. Samir asked on the 22nd whether a custom logo and theme belong
  in a customer's package. The theme is in already, as a per-machine
  preference beside the facture layout and the print language. A logo exists
  nowhere in the code, and the recommendation on the 22nd was a stored
  setting printed on the facture rather than a compiled-in brand, so that
  what differs per customer is the module list and not the paint. Its own
  task, later.
- The disk cleanup, about 40 GB of build leftovers, still waiting on Samir.
- The refund design question and the scanner check on Samir's phone.

## Guardrails

The ones the last loop lists, plus the budget cut above: every cargo command
through `just`, `just claim` before any bare cargo, `df -h /mnt/c` before a
heavy build, the `.owner` check or the gates run is void, read a log's own
exit line, never `pkill -f`, never `git stash`, never a bare `git add -A`,
never prettier. One PR per unit of work, `just ci` once after the day's last
merge.
