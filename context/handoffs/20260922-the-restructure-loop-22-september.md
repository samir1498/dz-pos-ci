---
title: 'The restructure loop, 22 September'
slug: 'the-restructure-loop-22-september'
status: 'active'
category: 'handoffs'
created: 20260922
tldr: 'Where the crate split stands, what was decided today and by whom, what waits on a person, and the five pages a new session reads first. Kept current while the loop runs.'
---
# The restructure loop, 22 September

Paused for the day at 2026-09-22 17:00. This is the page to read first when
the work resumes.

## Where the work is right now

Paused, nothing running, no agent alive, no worktree torn down. Main is at
`615cffb`. The branch `feat/the-kernel-stops-naming-the-shop` is pushed and
sits in the worktree `kernel-smaller` with a clean tree at `01410d8`, a
work-in-progress commit whose body carries the plan for every unfinished
piece. Read that commit message before anything else.

S2 and S3 are merged as `9cc5255` (PR #156). `crates/kernel` and
`crates/retail` exist, retail depends on the kernel, the kernel depends on
nothing of retail's, and cargo enforces the line that a reading test used to.

S4 is half done on the branch, two of its five pieces finished:

- Done: the nine shop column types moved to retail and `Role` kept
  (`b0eee52`); retail got its own error type for the six shop variants and
  the kernel's manifest dropped `askama` and `rust_xlsxwriter`, which it
  carried only to wrap their error types (`6a5fa47`); the printed word list
  split into a shared key and a shop key (`5681189` plus two formatting
  commits).
- Done but not gated: the audit log's 24 shop action tags moved to
  `crates/retail/src/audit_actions.rs`, deliberately outside `services/` so
  the ring walk does not read it as a second audit service. `just test`
  exited 0 on that tree; `just gates` has not run since. **Run the full
  gates first thing, before touching anything else.**
- Not started: the two raw counts in `backup.rs` and `support_bundle.rs`,
  the discount threshold in `settings.rs`, and the regression test in
  `crates/api/src/error.rs` that proves every error still maps to the same
  status and body after the enum became two. The commit body works out the
  shape for each, including that four thin wrappers are needed because the
  kernel's `repos` module is `pub(crate)`.

The allow list, which is S4's score, stands at five rows and must reach
three: `money/mod.rs`, `money/totals.rs` and `services/permissions.rs`, the
three Samir decided to keep.

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
