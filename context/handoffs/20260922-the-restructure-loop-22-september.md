---
title: 'The restructure loop, 22 September'
slug: 'the-restructure-loop-22-september'
status: 'active'
category: 'handoffs'
created: 20260922
tldr: 'Where the crate split stands, what was decided today and by whom, what waits on a person, and the five pages a new session reads first. Kept current while the loop runs.'
---
# The restructure loop, 22 September

Kept up to date while the loop runs. Last touched 2026-09-22 15:26.

## Where the work is right now

Phase A of `context/loops/20260922-the-split-then-the-first-doctor-module.md`.
S4 is running in the worktree `kernel-smaller`, one commit in (the nine
column enums moved to retail, `Role` kept). Samir ruled at 14:58 on both
questions it was told to stop for: the money arithmetic stays shared for
every trade, and the permission list stays one list. Those two rows become
written decisions rather than open questions, and are expected to be the
only rows left on the allow list when S4 lands.

S2 and S3 are merged as `9cc5255` (PR #156): `crates/kernel` and
`crates/retail` exist, retail depends on the kernel, the kernel depends on
nothing of retail's, and cargo now enforces the line a reading used to. The
worktree is torn down. S4 is next.

Carried out of S3 and finished in S4: `crates/kernel/Cargo.toml` carries
`askama` and `rust_xlsxwriter`, because `error.rs` wraps both crates' error
types and S3 did not split the enum by variant. The manifest says so at the
dependency. `schema.rs` stays whole in the kernel and retail re-exports it,
so the kernel's schema names 21 retail tables, the tradeoff the plan
accepted when the migration folder stayed single. `crates/core` survives as
a thin re-export facade so the api, the seeder, the Tauri app and all 59
core test files compile unchanged; S5 rewrites those call sites.

The boundary walk shrank honestly: `KERNEL_FILES` 21 to 19 and
`SHOP_WORDS_ALLOWED` 12 to 10, both by exactly the two files that left the
kernel (`print/mod.rs` and `print/escpos.rs`, which name a document on code
lines and so cannot live in a crate that depends on nothing). No rule was
loosened. Checked by the session against the file, not taken on report.

Disk, read correctly on 2026-09-22 at 15:25: `df -h /mnt/c` sits at 20 GB
and will not move when space is freed, because it measures the Windows drive
holding the WSL disk image and that image does not shrink. A stray 15 GB
per-checkout `target/` in the main checkout, which `just disk` says should
not exist, was removed; inside the filesystem `df -h /` now reports 826 GB
free, so cargo reuses blocks instead of growing the image and builds carry
on. The shared `.cargo-target` is 78 GB and its cleanup still waits on
Samir, because that one costs a rebuild. Giving space back to Windows needs
a shutdown and a compaction from PowerShell, which is his to run.

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

- Samir, inside S4: are the money kernel's discount and price shop-only or
  shared. The rows for `money/mod.rs` and `money/totals.rs` stay on the
  boundary walk's allow list until he answers; the safe default is to leave
  them as an accepted decision rather than move money code on a guess.
- Samir, inside S4: confirm the permission enum's ten shop variants stay in
  the kernel as a decision rather than a leftover. His ruling of the 21st
  says the list stays one list, so the expected answer is yes and only the
  written reason changes.
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

## The one thing that proves Phase A finished honestly

Nothing a shop can see changes. If a screen, a total, a printed paper or a
permission behaves differently when S7 closes, something in the move was a
rewrite and comes back out.
