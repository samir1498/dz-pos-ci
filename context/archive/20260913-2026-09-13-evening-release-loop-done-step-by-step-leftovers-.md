---
title: '2026-09-13 evening: release loop done, step-by-step leftovers for tomorrow'
slug: '2026-09-13-evening-release-loop-done-step-by-step-leftovers-'
status: 'active'
category: 'handoffs'
created: 20260913
tldr: 'Main green with releases+downloads shipped; tomorrow walk the leftovers step by step (print route, password reset call, M6 scope, emulator, R9, sonar, first tag)'
---
## Where we are

Main at `dcf0b30`, mirror `samir1498/dz-pos-ci` in sync, both CI green. Tracked tree clean. Store: research 6/9 active, M6 paused, everything else archived. opencode MCP rebound to dz-pos (local configs; global pc-ctx removed).

## Shipped today (all merged, mirror green)

- PR #66: ESC/POS file + TCP:9100 senders, dump goldens (USB/route later)
- PR #67: dz-review of first-setup + test gaps + first-setup rename
- PR #68: 3-OS release matrix on mirror, org publish via ORG_RELEASE_TOKEN, stable names, landing Download fr/en/ar with OS sniff (dry-run built all three, Dinar-Setup.exe verified)
- PR #69: ORG_RELEASE_TOKEN set from Samir's login (boss untouched); PR #70: mirror detour recorded as temporary

## Decisions holding

Org repo hosts releases; wait for cert (drafts until then); all three OSes; mirror builds until next month, then org-direct. Token = login token until fine-grained PAT.

## Tomorrow, step by step (use step-by-step skill, one item at a time)

1. Print route + permission for the ESC/POS bytes (unblocked, needs design: who may print)
2. Password-reset route: needs Samir's product call first (review finding: staff office passwords unsettable, owner password unrotatable)
3. M6 scope before tasks (needs Samir: LAN-only? pairing UX?)
4. Emulator eyeball (boots on :9100/:3000; needs eyes)
5. R9 tooling verdicts write-up
6. Sonar rescan on latest main
7. First tag: needs version word, then `just release vX`

## Waiting on humans (not sessions)

Anouar: Windows cert, updater key, comptable R8, native Arabic R6, price, real facture. Samir: workflow ruleset, Apple signing, version word. Disk: 11G on C:, clean before e2e/heavy builds.
