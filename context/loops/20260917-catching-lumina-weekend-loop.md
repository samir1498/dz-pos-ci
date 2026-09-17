---
title: 'Catching Lumina: the weekend loop'
slug: 'catching-lumina-weekend-loop'
status: 'active'
category: 'loops'
created: 20260917
tldr: 'Runs three plans unattended across 18 and 19 September: the dto split first because the other branches all touch that file, then the facture layout picker and three layouts, then the phone in three languages, then the rest of the architecture fixes. Work goes to named agents; the session keeps every ruling and every review verdict.'
roadmap: 'catching-lumina'
---
# Catching Lumina: the weekend loop

Samir, 2026-09-17: make a full roadmap to fix the gaps, loop through the
weekend, and hand the work to agents. Sonnet for light work, Opus where money
is involved, a sharper model for one review at the end.

The roadmap is `catching-lumina`. This file is how it runs.

## The four rulings taken before it started

Samir, 2026-09-17, so the loop never has to guess:

- Four facture layouts, not one and not eleven: the picker, a dense A4, an A5
  and an 80 mm facture.
- No batches and no variants this weekend. Both decide a market and both need
  rulings he is not here to give.
- The architecture fixes and the phone's languages are in scope.
- The review lenses run Sonnet per pull request and a sharper model once, over
  everything the loop merged.

The fifth item on the original list, supplier opening debt, was struck before
the loop started: it has been built since M2, core, route, form and tests, and
the Lumina reference page was wrong. Checking beats copying a survey.

## The order

**Phase 0, alone, nothing else running.** T2 of
`architecture-fixes-without-a-domain-split`: `crates/api/src/dto.rs` becomes
`dto/{domain}.rs` with `dto/mod.rs` re-exporting everything. Every later
branch in this loop adds a wire type, and without this they all add it to the
same 3071-line file. Its proof is that `just gates` passes with no edit
outside `crates/api/src/dto*`. `dz-builder`.

**Phase 1, the priority Anouar named.**
`invoice-layouts-a-shop-can-choose`, T2 through T6. T2 alone, because it is
the only task that can change an existing document and its proof is that the
eighteen `facture_a4` goldens are byte for byte unchanged. T3, T4 and T5 are
independent after that and can run together, one branch each. `dz-builder`
throughout: no new arithmetic is written, the totals arrive already computed,
and the golden harness parses the amounts back out of every file so a drifting
total cannot be accepted by regenerating it. T6 last, once there is something
to pick between.

**Phase 2.** `the-phone-in-three-languages`, T2 through T5. `dz-builder`. T2
and T3 are one branch, they are meaningless apart.

**Phase 3, the rest of the architecture fixes.** T3 through T8 to
`dz-builder`, in any order, at most two branches at once. T9, the two JSX
money sums, to `dz-money-builder`, because the fix is that the API answers a
total it does not answer today.

**Phase 4, closing.** The three lenses over everything the loop merged, on the
sharper model. The session proves or drops every finding itself. Then
`dz-standup`, so Anouar opens the site on Monday and sees what the weekend did
rather than hearing it from Samir.

## Who does what

`dz-builder` for routes, templates, hooks, i18n, file moves and the dto split.
`dz-money-builder` for T9 and for anything that turns out to touch centimes, a
rate, a balance, the schema or a number on a printed document. A builder that
discovers it is in money work hands the task back rather than finishing it.

`dz-verifier` runs the gates and the long builds, so a full run does not land
in the session that has to think about the result.

`dz-review-centimes`, `dz-review-reach` and `dz-review-tests` review each pull
request. They find; they never rule. Pass 2 is the session's and stays the
session's: an agent that reports a finding and then rules on it has marked its
own paper, which is where the two review fixes that opened new holes came
from.

Four agents at once, no more. Samir's cap, 2026-09-10.

## The guardrails, each one written because it already cost a day here

- Every cargo command through `just`. A bare `cargo` in a worktree silently
  reuses another branch's artifacts, because cargo names our three crates the
  same everywhere and trusts mtimes.
- `just disk` before every build. On this box `df -h /` lies, it is a VHDX on
  Windows C. Under 20 GB free, clean before building, do not build and hope.
- `just claim` in a worktree before any bare cargo, and export
  `CARGO_TARGET_DIR` into that same shell; `just claim` alone does not.
- Never `pkill -f`. It has twice killed unrelated servers here by matching the
  shell that ran it.
- Never `git stash`. It made a stash entry this session that had to be matched
  against disk before it could be dropped safely. Copy to the scratchpad.
- Never `git add -A`. Add the files the task named.
- Never run prettier. This repo has no prettier config, `just fmt` is
  `cargo fmt --all --check` only, and one `npx prettier --write` reformatted
  152 files and took hours to unpick.
- `just types` regenerates Rust to TypeScript bindings and does not typecheck
  the frontend. That is `npx tsc --noEmit -p tsconfig.json` in `apps/desktop`,
  and it is how a use-before-declaration got through a green `just types`.
- jsdom is not a browser. It has twice hidden a focus bug that Playwright
  caught at once. A claim about focus, selection or key delivery needs a real
  browser run.
- A `ctx:` trailer with an action word, in the same paragraph block as
  `Co-Authored-By`, no blank line between them. The hook rejects the commit
  otherwise.
- The words "pre-existing" and "already present" do not appear in a commit or
  a pull request. If something fails, find the root cause or say it is not
  found.
- Tear a worktree down with `just worktree-rm <name>`, never `rm -rf`.
- A branch and a pull request for code. Only `context/` bookkeeping goes
  straight to `main`.
- Anything money touches gets `just ci <branch> full` on the mirror
  `samir1498/dz-pos-ci` before it merges. The Dinar org skips every CI job by
  design, so a green tick there means nothing ran.

## When something goes wrong

Three attempts on one task, then park it: write what was tried and what it
did on the plan page, set the task back to pending with a note, and move to
the next task. A loop that grinds on one problem for six hours has spent the
weekend and built nothing.

Never park by lowering the bar. A test weakened until it passes is worse than
a parked task, because the parked task is visible on Monday and the weakened
test is not.

Call the advisor at each phase boundary, and before merging anything in
Phase 3, which is the phase with the most ways to be quietly wrong.

## Not in this loop

Batches, variants, bundles and promotions, invoice scanning, cloud sync,
delivery notes: all parked on the roadmap with the reason and what unparks
each.

The desktop's migration off its JSON dictionaries. The phone builds the new
shape; moving five hundred desktop keys proves nothing new and risks a lot.

Lumina's eight other A4 variants. Four layouts prove a shop can choose.

## What waits for Samir

- The barcode scanner plan cannot close. Its last task is his: an Android
  phone running `hid-barcode-scanner` from F-Droid as a Bluetooth wedge, and
  the laptop driving a `uinput` virtual keyboard on its French layout, both
  against the Tauri window rather than a browser tab. Step 1, installing the
  app, was sent and never answered. Nothing in this loop unblocks it, because
  the whole point is that a headless Chromium cannot prove it.
- One ruling still open from earlier: whether the PIN and the password share
  a lockout counter or keep separate ones.
