# dz-review — M6 + M7, `0746966..main` (65 commits, 228 files, ~19k insertions)

Reviewed 2026-09-16 at `1a4e6d3`, after PR #93 merged. Three Fable lenses
(centime correctness, roles and data reach, fixture quality) on
non-overlapping file sets, then a challenge pass where the session proved
each surviving claim itself.

## Pass 0 — what actually ran

- `just gates` green at `1a4e6d3`: 1159 Rust tests (core + api), 527
  desktop, 344 shared, 131 landing, 103 design, 11 mobile. First run was
  red on five stale landing shots; that was this branch's doing and is
  fixed at `2ff5de5`/`ac8a767`.
- `apps/mobile` IS in the gate (`pnpm -r test`, workspace member), but its
  11 tests are `queue.test.ts` and `session.test.ts` only. No screen is
  tested. The phone's findings below are uncovered, not unreachable.
- One probe run by the session against a real `axum::Router` over a real
  SQLite file (`crates/api/tests/review_probe_phone_tendered.rs`, deleted
  after; copy in this scratchpad).
- `just e2e` did NOT run in this session. The last evidence it ran is
  `fc70422`, which retook `apps/desktop/e2e/screenshots/products.png`;
  three commits landed after that.
- Not driven by hand: no Tauri window, no phone on a LAN, no printer.
- What a green suite cannot see here: a real phone against a real desktop
  over shop Wi-Fi, a printed ticket, a second device.

## Confirmed findings, ranked

### 1. The phone cannot complete a cash sale in a normal shop

`apps/mobile/src/screens/Till.tsx:61` computes what the customer handed
over as `Σ selling_centimes × qty`. Under the réel régime that sum is HT:
`money/totals.rs:12-13` states `unit_price` is "HT under the réel regime".
The server owes `net_to_pay = subtotal_ht + tva + stamp`
(`money/totals.rs:173-178`) and refuses anything short
(`services/sales.rs:1284-1290`).

Proven, not inferred — probe against a real router:

| basket | tendered | answer |
|---|---|---|
| 1 × 110,00 HT @ 19%, réel | 110,00 (the phone's formula) | `422 field=tendered "less than the amount to pay"` |
| same | 130,90 (`net_to_pay`) | `201`, `net_to_pay_centimes: 13090` |
| 40 × 110,00 @ 0 bps, over the 300,00 stamp floor | 4 400,00 | `422 field=tendered` |

The refusal names `tendered`, so this is not the review trap of a refusal
for the wrong reason. The identical basket rings when the amount is right.

Default régime is `reel`
(`migrations/2026-09-08-000000_init/up.sql:129`), so this is the
out-of-box case. Above 300,00 DA cash the stamp makes it fail under IFU
too (`money/stamp.rs:10,46`).

`TextInput` is imported at `Till.tsx:2` and never used — someone intended
an amount field.

**The manual walkthrough never reached this.** The evening handoff of
2026-09-15 records: step 2 added Café at 100,00 DA with **19% TVA**, step 3
sold one café **on the desktop till**, and step 4 — pair the phone — was
never reached, because the laptop went offline. So the phone was never
driven against this shop, and the shop it would have been driven against is
exactly the réel, taxed configuration that produces the 422 above. The
reduce is not new either: it has been in `Till.tsx` since `966cb74`
(M6 T5), well before the run that PR #91 titled "proven".

- **Covered?** No. The API fixture hardcodes `tendered_centimes: 30_000`
  (`crates/api/tests/sales_idempotency.rs:84`), comfortably above
  `net_to_pay`, so it never exercises the phone's formula. No test imports
  `Till.tsx`. Missing tests belong in `apps/mobile/src/screens/Till.test.tsx`
  and `crates/api/tests/sales_idempotency.rs`.
- **Quick or not?** Not quick — a shape choice. The phone needs
  `net_to_pay` before it can ask for an amount, which means a totals-preview
  route in `crates/api/src/routes/sales.rs`; or the phone posts `card` with
  no tendered (`sales.rs:1298` returns `(None, None)`), which is honest but
  a different fiscal document with no stamp.
- **Who is affected.** Wrong factures: **zero** — the refusal precedes
  `documents::issue` (`sales.rs:412` before `:444`), so no number burns.
  Sales the phone can make on a default shop: **zero** for any cash basket
  with a taxed line, and zero for any cash basket over 300,00 DA under any
  régime. Server side is correct as specified; this is entirely the client.

### 2. The phone treats every refusal as being offline

`Till.tsx:70` `if (!res.ok) throw new Error(String(res.status))`, and the
`catch` at `:79-86` enqueues. A 401, 403 or 422 is stored as if the network
had dropped. The retry resends the same body and the same dead headers, so
it earns the same refusal for ever; `Till.tsx:4` imports only
`enqueue, list, newIdempotencyKey, retry` — never `clear` or `remove`, and
`queue.ts:98-99` removes an item only on success.

Worse after a re-sign-in: `queue.ts:91-94` skips any item whose
`sessionToken` differs from the current one. Written for another person's
queued items (`queue.ts:81-82`), it catches the *same* person after they
sign in again. Those rows are then skipped for ever.

The everyday trigger is not revocation, it is the idle rule:
`services/preferences.rs:36` `DEFAULT_SESSION_IDLE_MINUTES: i64 = 15`, and
the phone has no keepalive. Confirmed enforced on every guarded route, not
just on an idle poll: `services/sessions.rs:196` returns `None` once
`now - last_seen_at >= session_idle`, and `session::require` resolves
through it, so `POST /sales` 401s.

This is the same root cause as finding 1 — the catch-all — which is why
finding 1's 422 becomes permanent rather than a message the cashier can act
on.

- **Covered?** No. `queue.test.ts:58` covers another signer's skip only.
- **Quick or not?** One screen. 401/403 clears the session and returns to
  `SignIn`; 422 shows the message and does not enqueue; enqueue only on a
  thrown fetch or a 5xx.
- **Who is affected.** Every phone idle past 15 minutes and then used to
  sell: goods go over the counter with no document, no number, no stock
  movement and no cash recorded, and the queue never delivers. Per-shop
  count unmeasured — no phone in a real shop yet. Server idle and revoke
  are both correct.

### 3. Pairing a phone is never audited, and the revoke row is written outside its transaction

`services/audit.rs:200` defines `ACTION_DEVICE_PAIRED`; grep across
`crates/` and `apps/` finds no use of it. `services/pairing.rs:171-183`
calls `repo::revoke_device(...)?` and then `audit::record(...)` with no
`conn.transaction` around the pair, against the rule stated at
`services/audit.rs:5-6`. `services/users.rs:310,361` obey that rule.

- **Covered?** No. `crates/api/tests/pairing_api.rs:205-252` asserts
  status codes and `revoked_at`, never an audit row.
- **Quick or not?** Quick. Wrap `revoke_device` in `conn.transaction`; add
  one `audit::record` in the claim closure.
- **Who is affected.** One missing row per phone ever paired. The revoke
  gap needs a crash between two statements. No data reach.

### 4. `/auth/first-setup` sits outside the device gate

`crates/api/src/lib.rs:604-606` puts `/auth/first-setup` and
`/pairing/claim` in the `auth` router, which carries no `device::require`.
`/pairing/claim` must be there — it is how a device token comes to exist.
`/auth/first-setup` does not need to be.

- **Quick or not?** Quick — move the route into `phone_auth`; loopback
  passes the gate bare (`device.rs:79-81`), so the desktop is unchanged.
- **Who is affected.** Needs all of: `--lan` running, a shop with no owner
  yet, and the launch token in hand. Zero shops today, one claim per shop
  ever (`gates.rs:112`), and the owner of an empty shop is noticed at once.
  Real, cheap to close, small consequence.

### 5. The race loser can get a 500 where the design says 409

`repos/sale_idempotency.rs:44-49` returns `Ok(false)` only on a UNIQUE
violation; a SQLite busy error falls to `CoreError::Query`, which
`crates/api/src/error.rs` maps to `INTERNAL_SERVER_ERROR`. The core race
test (`crates/core/tests/sale_idempotency.rs:229-238`) accepts `Err(_)` as
"conflict" either way, so nothing pins which branch fires.

Note the in-process race is not reachable: `crates/api/src/lib.rs:72` holds
one connection behind `Arc<Mutex<..>>`, so claims serialize before SQLite.
This needs a second process.

- **Who is affected.** No money moves wrong — the test still asserts
  exactly one document and the loser's retry replays the winner. The phone
  shows a server-fault message for a case the design calls a conflict.

### 6. The phone's pair-and-sell walkthrough cannot run as committed

`Till.tsx:133` renders the retry button only when `queued > 0`, and
`apps/mobile/maestro/pair-and-sell.yaml` taps "Retry queued" immediately
after asserting "Queued: 0". The revoke in step 6 is a comment, not a step.
The device token is a literal placeholder. Every assertion reads a label;
none reads a sale row or a total. The README says it is not in CI.

This matters because it is the only thing standing behind the claim that
M7 was "proven" on a real phone.

## Cleared, and worth saying plainly

The sharp edges I was most worried about are genuinely handled, each with a
quoted line:

- **Same key, different cart.** `services/sales.rs:310-325` fingerprints
  the request, compares `hit.request_hash != hash`, and answers 409. It
  does not hand back a neighbour's sale. Filtered by `shop_id`.
- **A replayed ring prints the stored paper**, not the incoming body —
  the replay returns `documents::get(conn, shop_id, hit.sale_id)`.
- **The QR claim is genuinely atomic.** `services/pairing.rs:95`
  `BEGIN IMMEDIATE`, loser refused at `:98`, and
  `two_concurrent_claims_pair_only_one_phone` is a real race — `Barrier`,
  two connections, loser must be `AuthRefused`. The only genuinely
  concurrent test in the range.
- **Middleware order is token → device → session** (`lib.rs:748-760`), as
  the architecture says.
- **`::ffff:127.0.0.1` cannot spoof loopback** — `lib.rs:814` binds
  `0.0.0.0`, IPv4 only, and no forwarded header is read.
- **A revoked phone dies on its next call**, not on restart
  (`services/pairing.rs:144-147`).
- **Password reset is owner-only and shop-scoped**, and audits actor,
  target and before/after without ever the hash (`services/users.rs:314-322`).
- **Money stays `i64` at every hop**, each through `within_js_safe_range`
  (`dto.rs:991-1014`). No float near a total, no second rounding in ESC/POS.

## Pass 3 — the five questions

**Did we build the right thing?** The server half of M7 is right. The
phone half was never driven at all — see the walkthrough note under
finding 1. "Paired phone proven" rests on a Maestro flow that cannot run
as committed (finding 6).

**Is there a materially simpler shape?** Findings 1 and 2 are one bug:
`Till.tsx` has a single `try/catch` doing double duty as a network handler
and an error handler. Splitting refusal from disconnection fixes both and
is the highest-value change in this list.

**How complicated is this, honestly?** The Rust side is in good shape —
fingerprinting, `BEGIN IMMEDIATE`, layered gates, shop scoping all hold up
under a hostile read. The complexity that bit is on the phone, where a
thin client silently re-derives a number the server owns.

**Already solved elsewhere in the repo?** Yes — the desktop till already
gets `net_to_pay` from the server and never computes it. The phone should
do the same.

**What can be deleted?** The unused `TextInput` import at `Till.tsx:2`.
The Maestro flow needs repair or removal — as committed it cannot pass, and
a walkthrough that cannot run is worse than none because it is cited as
proof.

## Recommended order

1. Finding 1 + 2 together — one screen, one root cause. Blocks the phone
   shipping at all.
2. Finding 6 — repair the walkthrough so 1 and 2 stay fixed.
3. Findings 3 and 4 — both quick, both close a real gap.
4. Finding 5 — smallest consequence; fold into the next idempotency touch.
