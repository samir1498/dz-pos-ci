---
title: '16 Sep morning: the phone could not ring a sale, fixed; walkthrough resumes at step 4'
slug: '16-sep-morning-the-phone-could-not-ring-a-sale-fixed-walkthr'
status: 'active'
category: 'handoffs'
created: 20260916
tldr: 'Phone till sent the HT sum as tendered so a default réel shop refused every cash sale; fixed and merged (#94). Run pnpm install on the laptop before restarting Expo, then pair the phone.'
---

## Read this first: two things the last handoff got wrong

- **The last handoff called the five landing `shots.test.ts` webp failures
  "environment goldens, not ours", already failing on a clean tree and proven
  so via stash.** Not true, and it is the kind of excuse `CLAUDE.md` says
  never to make. The stash only showed the failure did not come from
  *uncommitted* work; the cause was already committed, at `fc70422`, which retook
  `apps/desktop/e2e/screenshots/products.png` for the filter-bar fix without
  anybody rerunning `pnpm run shots`. Regenerating the five webp files fixed
  it, which is the proof. Gates are green on main.
- **"`main` is red on `tsc` (T4 fallout)."** Stale. Main is green.

## What happened this morning

The desktop and mobile polish batch merged (PR #93, squashed at `1a4e6d3`),
then a three-lens adversarial review ran over everything merged since
2026-09-12 — 65 commits, 228 files. Full report and the probe that proved
the blocker are in `research/reviews/2026-09-16-m6-m7-adversarial-review.md`.

**The blocker: the phone could not complete a cash sale in a normal shop.**
`Till.tsx` sent `Σ selling_centimes × qty` as the amount the customer handed
over. Under the réel régime — the default — that sum is HT: no TVA, no
stamp. The server owes `total_ttc + stamp` and refused, `422
field=tendered`. Proven against a running router: 110,00 offered against
130,90 owed. Over 300,00 DA cash it fails under the IFU too, by the stamp.

Then the till treated every refusal as a dropped connection, queued the
sale, and retried the same body for ever. The everyday trigger was not even
that bug — it was the 15-minute session idle: sell after idling out and
goods left the counter with no document, no number, no stock movement and
no cash. Nothing was corrupted either way; the refusal lands before a number
is burned. The till simply did not work.

Step 4 of the 15 Sep walkthrough — pair the phone — is exactly where this
would have shown up, and the laptop went offline before it was reached. The
HT sum has been in `Till.tsx` since `966cb74` (M6 T5).

## What changed

Phone (`apps/mobile`):
- `lib/basket.ts` prices the basket through `@dzpos/shared`'s
  `computeTotals` — the same mirror of `crates/core/src/money` the desktop
  till uses, pinned by the same files under `fixtures/money/`. The screen
  shows **To pay**, a **Pay exact** button fills the box, the cashier can
  type any amount at or above it, and the **Change** shown afterwards is the
  server's answer, never the phone's arithmetic.
- `lib/outcome.ts` sorts an answer: a refusal is shown to the cashier, a
  dead session sends them to the PIN box, a revoked device sends the phone
  back to pairing, and only a call that got no answer at all is queued.
- The device token now lives apart from the person's session, so an
  idle-out asks for a PIN, not for a QR only a manager can supply.
- `apps/mobile` had **no typecheck in any gate** — `moduleResolution` was
  unset, so `tsc` could not resolve a single import. Fixed; the package's
  `build` now typechecks, so `just gates` covers it.
- The Maestro flow could not run as written (it tapped a button the till
  unmounts when the queue empties, and its revoke was a comment, not a
  step). Repaired, with the human-only steps marked as such.

Server:
- Pairing a phone now writes a `device.paired` audit row, inside the claim's
  own transaction. Revoking already wrote one, but outside its transaction;
  both are now in one.
- `/auth/first-setup` moved behind the device gate. A host on the shop Wi-Fi
  holding only the launch token could claim the owner of a shop that had
  none, without pairing.

## Deliberately not done, with the evidence

The race loser on a duplicate retry key gets **500, not the 409** the design
intends. Measured, not guessed: with two connections the loser answers
`Query(DatabaseError(Unknown, "database is locked"))`. SQLite's busy
handling wins before the UNIQUE insert is ever attempted, so the `Ok(false)`
branch in `repos::sale_idempotency::record` is unreachable across processes.

The real fix is a `BEGIN IMMEDIATE` on the sale transaction, the way
`services::pairing` takes the write lock up front. That is a shape change to
the money path — `sales::issue` uses `conn.transaction`, and `pairing` had
to drop to manual `BEGIN`/`COMMIT`/`ROLLBACK` to do it — so it wants its own
scoped change rather than riding along here. `crates/core/tests/sale_idempotency.rs`
now pins the behaviour as measured, so making it answer 409 turns that test
red and has to be done on purpose. No money moves either way: exactly one
document exists, and the loser's retry reads the winner.

## Your order, resuming the walkthrough

1. **On the laptop, `git pull` and then `pnpm install`.** This is not
   optional: `apps/mobile` gained `@dzpos/shared`, `@types/react` and
   `@types/node`, and Metro will not bundle without them.
2. Restart Expo per the recipe below. **Watch the first bundle** — this is
   the first time the phone imports a workspace package, and Metro resolving
   a pnpm workspace dep is the one thing that could not be tested from the
   WSL box. If it fails to resolve `@dzpos/shared`, say so and it becomes a
   Metro config fix, not a rewrite.
3. Step 4: pair the phone. Mint the QR only once the phone is showing the
   pair screen — it lives 60 seconds.
4. Sign in, then sell: the till should now show **To pay: 130,90** for one
   Café (100,00 at 19%), not 100,00. Tap **Pay exact**, then **Pay (cash)**,
   and expect a **Change: 0,00** line and an empty cart. Try typing 200,00
   as well — change should read 69,10.
5. Kill the Wi-Fi, sell again, expect **Queued: 1**. Back online, tap
   **Retry queued** — the button disappears with the queue. Check the
   desktop's documents list: **one** ticket for that basket, not two.
6. Revoke the phone from settings mid-shift, then try to sell. The phone
   should land back on the pairing screen with a message, not queue the sale.

## Laptop recipes (carried forward, still current)

Expo restart — run ON the laptop, expand the token there, never locally:
`cd apps/mobile && EXPO_PUBLIC_API_URL=http://192.168.0.134:4317 EXPO_PUBLIC_API_TOKEN=$(cat ../../.dev/api-token) REACT_NATIVE_PACKAGER_HOSTNAME=192.168.0.134 npx expo start`

LAN IP `192.168.0.134`, firewall 4317/tcp open, avahi active. tmux `dz`
windows api/dev/mobile. The package is `dinar-mobile`, not `mobile`, and it
has no `start` script — use `npx expo start`.

Pairing: `POST /pairing/qr` as owner (cookie login via curl, launch token
from the laptop's `.dev/api-token`), 60s TTL, type the 64-hex by hand.
Phone signs in as owner with password (no cashier in the dev DB).

Shop state in `dev.db`: owner Samir (EN), Café (barcode 2000010000012,
100,00, 19%, stock 20), one ticket sold on the desktop at step 3.

## Gotchas (carried forward, do not relearn)

- A stale laptop browser tab shows yesterday's JS: close it, fresh tab,
  Ctrl+Shift+R.
- Dev persist needs ONE fresh sign-in after the fix to seed storage;
  incognito starts empty; use `http://127.0.0.1:5173`, not `localhost`, so
  the cookie flows.
- `sr-only` on a kit `Input` loses to `w-full` and makes a full-width
  invisible box. Use `hidden`.
- A launch token rotates on every `just api` restart; one in tmux scrollback
  is already dead.
- Open questions from Samir, still untouched: the "For sale" label reads
  like a discount in EN (it means active; FR/AR fine), and the TanStack
  Table question (answered: hand-rolled DataTable, migrate only for sorting
  and paging).

## Plan bookkeeping, corrected

- **M7 T6 is back to pending.** It was marked done and the milestone
  archived, but the proof never ran — the walkthrough stopped at step 3 and
  the Maestro flow could not execute. Only your phone closes it; the code
  landing does not.
- M7 T4 was left `in-progress` while the milestone closed `done`. The code
  merged in #89 and holds up under review; the status line was wrong.
- The ESC/POS plan sat in `archived/` with its header still `active`, both
  tasks done. Marked done.
- Nothing was cancelled or archived by mistake. M1 T6 "ESC/POS over USB" was
  cancelled on purpose (no thermal printer here) and the byte-level work
  shipped as its own plan.
