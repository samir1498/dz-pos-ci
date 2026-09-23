---
title: 'The restructure loop, 22 September'
slug: 'the-restructure-loop-22-september'
status: 'active'
category: 'handoffs'
created: 20260922
tldr: 'Where the crate split stands, what was decided today and by whom, what waits on a person, and the five pages a new session reads first. Kept current while the loop runs.'
---
# The restructure loop, 22 September

Updated 2026-09-23 10:25. S2, S3 and S4 are merged: the crate split
(`9cc5255`, PR #156) and the kernel no longer naming the shop (`5135814`,
PR #158). The boundary walk's allow list is at its target of three rows,
the money arithmetic and the permission list, each naming Samir's
2026-09-22 ruling. Next is S5, the api's shop routes behind the retail
feature, then S6 (tests move, and the Rust inline tests leave `src/`
under Samir's 2026-09-23 no-inline-tests rule), then S7 (a build with no
shop signs a user in).

In parallel, on its own branch `chore/coverage-reaches-sonar`: TypeScript
tests move out of `src/` into each package's `tests/`, a gate bans inline
tests with a shrinking list for the Rust files still carrying one, and
`just coverage` feeds Sonar, which read 0% because nothing generated the
reports.

Samir's rulings on 2026-09-23: no full ports and adapters, only ports at
the plug points after the doctor module (recorded on the split plan); no
inline tests; compress tests when touched.

## Two agents froze today, and the cause is known

The first S4 builder stopped writing at 15:16 and never resumed; it was
killed at 16:11 and its work recovered intact from the tree. Separately, two
shells left by the S2 and S3 builder spun for two hours: each polled with
`pgrep -f "cargo test ..."` for a process to finish, and the pattern matched
the polling shell's own command line, so the condition could never come
true. Both were killed by process id. Never write a wait loop that greps for
a process by a pattern its own command line contains; run the command in the
foreground and read its exit line.

## What was decided today, and by whom

- Anouar, 08:52: a module brings its own screens, permissions and tables and
  never touches the core's. That is the shape.
- Anouar, 09:03 and 09:49: not at runtime. We prepare each customer's
  package and send it, so modules are compiled in.
- Samir, 12:46: go with the split, make the core smaller, plug points later.
- Samir, 13:23: branding waits, focus on the restructure. The theme is
  already a per-machine preference; a logo exists nowhere in the code, and
  the recommendation was a stored setting printed on the facture rather than
  a compiled-in brand, so what differs per customer is the module list.

## The order, and why it is this order

The split first, the doctor module second, the plug points third. A hook
drawn from one implementer is a guess, so the traits wait until patients and
appointments exist beside retail. The paper test justified starting: all
nineteen places a consultation tears the model sit in the retail tables or
the stamp and TVA rules, none in the shared quarter, so the line the crates
draw is the line the tear already found.

## What is waiting on a person

- Answered, 2026-09-22 14:58, so no longer waiting: the money arithmetic
  stays shared for every trade, and the permission list stays one list. The
  three rows those rulings cover keep their place on the boundary walk with
  their reasons rewritten to name him and the date.
- Samir: the refund design question (manager only, or a cashier with a
  manager's code), the scanner check that needs his phone, Windows 7 with a
  first real shop, and the disk cleanup.
- Nobody: Anouar has answered everything he was asked.

## The budget, which changes how this loop runs

Anouar, 10:38: the shared Claude plan hit 95 percent and he needs headroom
for his own work. Samir asked for the loop anyway, so it runs one builder at
a time, with no three-lens fan-out on the pure move, and the session
spot-reads the diff itself.

## The pages a new session should read, in order

1. `context/loops/20260922-the-split-then-the-first-doctor-module.md`, what
   is being done now.
2. `context/plans/20260922-a-kernel-crate-and-retail-as-the-first-module.md`,
   the seven tasks and the measured counts.
3. `context/research/20260922-what-clinic-software-provides.md`, the target
   the split is built towards, with the appointment book worked through.
4. `context/research/20260922-the-paper-test-a-consultation-in-the-document-model.md`,
   the nineteen tears.
5. `context/research/20260921-module-shape-the-seven-compared.md`, why this
   shape and not the other six. Its own header says D4 later found twelve
   pinned files where that page counts six.

## Where to pick up

1. `cd` to the worktree `kernel-smaller`, `just claim`, run `just gates` and
   read its exit line. The audit-tag move is ungated.
2. Finish S4's three remaining pieces from the `01410d8` commit body.
3. Then S5, S6, S7 of the plan, then Phase B, the patients and appointments
   module, whose plan page is not written yet.

## The one thing that proves Phase A finished honestly

Nothing a shop can see changes. If a screen, a total, a printed paper or a
permission behaves differently when S7 closes, something in the move was a
rewrite and comes back out.
