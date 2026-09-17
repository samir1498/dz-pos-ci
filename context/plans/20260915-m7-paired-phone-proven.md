---
title: 'M7 paired phone proven'
slug: 'm7-paired-phone-proven'
status: 'active'
category: 'milestone'
created: 20260915
tldr: 'Close the three M6 review gaps on the shop floor: atomic QR claim, device auth that is actually checked, sales that ring once'
tasks:
  - id: 'T1'
    desc: 'QR claim atomicity (core): conditional mark-used with rows-affected check inside one transaction; two concurrent claims, second is AuthRefused'
    status: 'done'
  - id: 'T2'
    desc: 'Device middleware (api): X-Dzpos-Device required off loopback, unknown/revoked is 401, last_seen bump; device_by_hash comes alive with tests'
    status: 'done'
  - id: 'T3'
    desc: 'Phone session story (api+mobile): a person signs in on a paired phone, device+session both required, desktop permission gates apply unchanged'
    status: 'done'
  - id: 'T4'
    desc: 'Sale idempotency (core+api+mobile): client key per sale, server dedupe with unique constraint, same-key retry returns the original, print follows a retried ring'
    status: 'done'
  - id: 'T5'
    desc: 'Mobile auth cleanup: EXPO_PUBLIC_API_TOKEN bearer misuse gone, device token + session stored and sent'
    status: 'done'
  - id: 'T6'
    desc: 'Maestro proof on a real phone over Tailscale: pair, sign in, sell, kill Wi-Fi, retry rings once, revoke, next call 401s'
    status: 'pending'
  - id: 'T7'
    desc: 'Closing sweep: docs/features.md §6, docs/architecture.md transport table, docs/roadmap.md M7 demo, dz-review (money+roles) before checkpoint PR'
    status: 'done'
acceptance: []
---
# M7: the paired phone, proven

M6 put a phone on the shop floor. The closing review found three gaps
between what the milestone claims and what the wire enforces, and the dig
on 2026-09-15 proved all three real. This milestone closes them, on a real
phone, before any new feature.

Demo that closes it: pair a phone, sign in as cashier, kill the Wi-Fi
mid-sale, retry and watch the sale ring exactly once; revoke the phone
mid-shift and watch its next call fail closed; scan one QR from two
claimants at once and watch the second fail.

In: atomic QR claim; device auth that is checked, not stored; a session
story for the phone so the desktop's permission gates apply unchanged;
idempotent sale ring with the print following a retried ring; Maestro
proof over Tailscale.

## What the dig proved (2026-09-15)

**QR claim TOCTOU is real.**
`claim_pairing_token` (`crates/core/src/services/pairing.rs:72-112`)
reads the token row and checks `used_at`/expiry *outside* the transaction,
and `mark_pairing_used` (`crates/core/src/repos/pairing.rs:34-43`) is an
unconditional `UPDATE … WHERE id` with the row count unchecked. The comment
claiming a concurrent claim "cannot both succeed" is wrong: SQLite
serializes the two writes and both succeed, so one QR pairs two phones.
Fix: checks plus a conditional update (`used_at IS NULL`) with a
rows-affected assert, all inside one transaction.

**Device auth is unenforced.**
`device_by_hash` (`crates/core/src/repos/pairing.rs:55-67`) carries
`#[allow(dead_code)]`: no request path ever looks a device token up. No
`X-Dzpos-Device` header exists anywhere in `crates/api`, the mobile app,
or the desktop client. The pairing reply mints a device token
(`crates/api/src/dto.rs:2961`, `routes/pairing.rs:68`) and nothing ever
asks for it again — pairing is a ceremony with no gate behind it.

**The phone has no session story.**
`Till.tsx` sends `Authorization: Bearer $EXPO_PUBLIC_API_TOKEN`, a
build-time env var, and no `X-Dzpos-Session`. The one router
(`crates/api/src/lib.rs:603-745`) puts `session::require` (401 without a
session, `session.rs:114`) and `token::require` (per-boot random launch
token) in front of every guarded route including `POST /sales`
(`routes/sales.rs:265`, takes `CurrentUser`). A build-time bearer cannot
match a per-boot token, and no session means `SessionRequired` — so the
phone as built cannot complete a sale against the real server, and the
"device says which phone, session says which person" split from the
pairing module docs exists only in prose.

**Retry re-rings.**
`queue.ts:70-90` replays the identical `POST /sales` body on retry and the
server has no idempotency key anywhere (`grep idempotency crates/ apps/`
is empty). Commit-then-timeout rings the sale twice. Side gap in the same
flow: `Till.tsx:39-60` only attempts the print on the first try, never
after a queued retry, so a retried sale prints nothing.

## Where T6 actually stands (2026-09-17)

The milestone is closed and the roadmap entry says so; this task is the
one piece of it left open, and it is left open on purpose.

Proven: pairing, sign-in and an online cash sale from a phone. Samir
drove it by hand on 2026-09-16 (QR scanned, name tapped, four digits,
twelve coffees rung as ticket 2 for 1 443,00 DA in the shop file), and
`just demo-phone` repeats pair, sign in and sell on the emulator every
time it films.

Not proven on hardware: the rest of T6's sentence. Nobody has killed the
Wi-Fi mid-sale, watched the retry ring exactly once with the print
following it, then revoked the phone from the desktop and confirmed the
next call gets a 401. The server side of all three has tests; the phone
side has never been driven through them by a person or by Maestro.

`apps/mobile/maestro/pair-and-sell.yaml` is the flow that would do it and
its steps 4 to 7 are written. Its step 2 still types a user id into a
field the sign-in screen removed in PR #99, so the flow stops before it
reaches them. Fixing that step is the cheap half of closing T6; borrowing
a second phone for twenty minutes is the other half.
