---
title: 'Till shifts: a float and a count'
slug: 'till-shifts-a-float-and-a-count'
status: 'active'
category: 'feature'
created: 20260921
tldr: 'One shifts table, one drawer per person, and an expected figure made only of that person own cash takings. The dashboard does not read shifts at all, so no handover can move the shop cash figure.'
priority: 80
tasks:
  - id: 'T1'
    desc: 'This page: the design, the schema, the awkward cases as decided'
    status: 'done'
  - id: 'T2'
    desc: 'The migration, the models, the repos, and the expense clock fix that ships with them'
    status: 'done'
  - id: 'T3'
    desc: 'services::shifts, the cash window it asks for, and the three audit actions'
    status: 'done'
  - id: 'T4'
    desc: 'The four routes, the gate rows, the open_and_close_till permission and the tag hook'
    status: 'done'
  - id: 'T5'
    desc: 'Opening at sign-in and closing at the till, both in -till/session.tsx'
    status: 'done'
  - id: 'T6'
    desc: 'A refund that leaves the drawer, and the shift figure on the dashboard and expenses screens'
    status: 'pending'
  - id: 'T7'
    desc: 'The closing sweep: the shift list, the e2e with a real cashier, the docs'
    status: 'pending'
---
# Till shifts: a float and a count

A cashier opens the till with what is in the drawer, sells all day, and at
close types what is in the drawer again. The shop learns whether the two
agree. That is the whole feature, and everything below exists to keep it
from answering a bigger question than that.

The word is **shift**, never session.
`crates/core/src/services/sessions.rs` is the auth session, and
`sessions -> users -> sessions` is one of the rings still open in
`RINGS_STILL_OPEN`. A second meaning on that word in the same crate costs a
reader every time. `crates/core/src/print/facture.rs:31` already says "a till
is reconciled by shift", so the word is the repo's own.

Every ruling this rests on was taken by Samir on 2026-09-20 and is written
out in `context/loops/20260920-closing-the-lumina-gaps-loop.md`. None are
open. This page is the build; that file is the why.

## What the shift answers, and what it must not

A shift answers **whose count is short**. It does not answer whose sales,
which `user_id` on the document already answers, and it does not answer what
the shop took today, which `cash::position` already answers.

`cash::position` keeps deriving exactly what it derives now, from sales, both
ledgers and expenses. Nothing about a shift enters it. This is the rule that
keeps a 15 000 DA handover from reading as the shop losing 3 000 on a day it
took 12 000, and it is why the dashboard does not read the `shifts` table at
all in v1.

## The file

One new table, one migration, no ALTER of an existing one except the expense
clock below.

`shifts`: `id`, `shop_id`, `opened_by` (users), `opened_at` (shop clock),
`opening_cash_centimes >= 0`, `closed_at` nullable, `closed_by` nullable,
`counted_centimes` nullable, `expected_at_close_centimes` nullable, `note`
nullable. STRICT, `shop_id` on the row, integer centimes, the rules every
migration since 000000 follows.

Three constraints carry rules the service must not be the only thing holding:

- A partial unique index on `(shop_id, opened_by) WHERE closed_at IS NULL`.
  That is ruling 1's "at most one open shift per user" as a fact the file
  holds. Per user and not per shop, because shifts of different people
  overlap on purpose: each person's cash is physically their own.
- A CHECK tying the close columns together: `closed_at`, `closed_by`,
  `counted_centimes` and `expected_at_close_centimes` are all null or none
  of them is.
- A CHECK that `note` is NOT NULL whenever `counted_centimes` differs from
  `expected_at_close_centimes`. Ruling 4 requires a reason for a gap, and
  nothing else in the file can enforce it.

There is no `cash_movements` table and no `drawers` table. Cash handed to the
owner is the note at close; an expense paid out of the shop's money is an
`expenses` row as it is today, written by a `commit_money` holder. Neither
touches a cashier's expected figure. If a shop later asks for the individual
movements, that is a table added then.

`expected_at_close_centimes` is the one stored figure and it is stored on
purpose. The live position stays derived on every read (rule 4 of
`docs/architecture.md`, and the cash paragraph's "stored nowhere"). But a
shift's difference is a fact about one evening: a ticket annulled on
Wednesday leaves Monday's takings, so a derived expected figure would move
Monday's count after it was signed. The snapshot is what the closer saw, and
the difference is `counted - expected_at_close` with both stored, so it is
checkable and does not drift.

## What the expected figure is made of

    opening_cash + that user's cash sales + that user's cash debt payments

and nothing else. Every term is `checked_*`.

**Why the user filter is not optional.** `repos::cash` carries no `user_id`
anywhere: `sales`, `customer_payments` and `supplier_payments` each filter on
`shop_id` and a time range and nothing more. Ruling 1 lets two cashiers hold
overlapping shifts, so a shop-wide sum handed to each of them counts the
other one's takings. Two cashiers each ringing 1 000 000 centimes between
09:00 and 14:00 would each be told to expect 2 000 000, each be recorded
1 000 000 short, each be made to write a note and each get an audit row
saying so, for doing nothing wrong. `documents` carries `user_id` and so does
`debt_ledger`, so the filter is a where clause rather than a redesign.

**Why expenses and supplier payments are out.** Both are `commit_money` work,
which a cashier does not hold: they are the manager's payments out of the
shop's money, not out of this cashier's drawer. Taking them out also removes
a problem with no cheap fix: `repos::expenses::total_between` filters
`expense_date`, which is a date and not a moment, so a shift running 09:00 to
14:00 would have pulled the whole day's expenses including one filed at 20:00
by somebody else. There is no shift-sized slice of that column to ask for.

**Which timestamp.** `issued_at` on documents, `created_at` on `debt_ledger`.
Both are stamped by the services from the shop clock:
`services::sales::issue` sets `issued_at` to `clock::now()` when the caller
gives none (`crates/core/src/services/sales.rs:233`), and `NewSaleDto` cannot
give one (`crates/api/src/dto/sales.rs:388`, `:461`); `repos::debt` stamps
`created_at` the same way. `repos::cash::sales` already reads `issued_at`, so
a shift and the dashboard read the same column.

`documents.created_at` is left alone and nothing here reads it. It takes
SQLite's `CURRENT_TIMESTAMP`, which is UTC while Algiers is an hour ahead, so
a tag read off it would be wrong by an hour at every shift boundary. That is
the error the boundary cases in T3 exist to catch, and they would pass
against the wrong column, so the column is named in each of them.

## The services

A new `services::shifts` asks `services::cash` for its figures and never
touches `repos::cash` or `repos::expenses`, or `REACHES_PAST_A_SIBLING` grows
on its first commit. That list is down to two rows and three reaches as of
2026-09-21 and only shortens.

`services::cash` gains one function, `takings_for(conn, shop_id, user_id,
from, until)`, answering that person's cash sales and cash debt payments over
a window. `repos::cash` gains the user-filtered queries behind it;
`repos::cash::sales` and `customer_payments` already take a half-open
`[from, until)` on the right columns, so what is new is the `user_id`, not
the window. `cash::position` is untouched and keeps summing the shop over a
day or a month from sales, both ledgers and expenses. A shift is a second,
narrower question put to the same module, not a new input to the old one.
That is what keeps a shift, a handover or a refund from moving the shop's
cash figure.

`shifts::sales_outside_a_shift` is a `documents` question and not a `cash`
one: which of this user's sales fall in none of their own windows. It goes
through a helper on `services::documents` rather than a query of its own,
because a `("shifts", &["documents"])` row is exactly what the burn-down list
refuses.

A sale rung while the ringer has no open shift is accepted and tagged, never
refused (ruling 2). The tag is derived, not stored: a sale belongs to no
shift when its `issued_at` falls outside every `[opened_at, closed_at)` of
that `user_id`. The close screen reads that as its own figure beside the
expected one, so a morning rung before the shift opened does not read as a
shortage.

Three audit actions: `till.open` (opening cash), `till.close` (expected,
counted, difference, note, and the opener's id when the closer differs), and
`till.sale_outside_shift` on a tagged sale. The §5 audit list of
`docs/features.md` gains the three.

The shift list is its own screen under `see_reports`, not a view of the audit
log, because `see_audit_log` is the owner's alone (`docs/features.md` line
891) and a manager who cannot read a day's shifts cannot run a floor.

Opening and closing took two permissions, not one, and they shipped with T4.
`open_and_close_till` is held by all three roles under ruling 10 and gates
the routes; `close_another_persons_till` is Owner and Manager only and is
asked for inside `services::shifts::close`, and only when the closer is not
the opener. Together they took `permissions.rs` from thirteen to fifteen, and
the match refused to compile until all three roles placed each one, which was
the point. §5 of `docs/features.md` moved with them, and so did the row in
`.claude/stale-homes.md`. `crates/api/src/gates/mod.rs` keeps no permission
count of its own by design: its only count is the reads, because a fine
permission checked in a service gets no gate row.

## The expense clock, still worth fixing

`expenses.created_at` takes `DEFAULT (CURRENT_TIMESTAMP)` from
`crates/core/migrations/2026-09-10-000008_suppliers_purchases_expenses/up.sql`,
which SQLite answers in UTC while Algiers is an hour ahead. No shift figure
depends on it any more, so it stops being load-bearing here, but it is still
wrong for anything reading an expense by the hour. It ships in this
migration in the 000013 shape: stamp on write, and move the rows written so
far by the hour they are short.

## The awkward cases, each with what the file does about it

- **A shift across midnight.** A session is a shift, not a day. The
  dashboard's day and month figures keep their calendar definition; the
  shift report reads the shift window. Both are true at once, and each
  screen says which question it answers.
- **Closed by a different user.** `closed_by` is its own column and the
  audit row names both. A cashier closes their own and nobody else's
  (ruling 10), and the check is inside `services::shifts::close` rather than
  on the route, because one route serves both cases.
- **A sale with no shift open.** Accepted and tagged (ruling 2). This is the
  answer that keeps the fixtures as they are: seven desktop e2e specs
  (`till`, `till-credit`, `till-facture`, `till-cashier`,
  `till-reversals-and-quotations`, `settlement`, `scanner`), `just seed`, the
  demo recordings and the Maestro flows all ring sales with no shift open.
  Refusing would have meant opening a shift in every one of them.
- **The phone's queue replaying after close.** The sale belongs to the shift
  open when it reaches the server (ruling 3, corrected the same day it was
  written). `issued_at` is not on the wire: `crates/api/src/dto/sales.rs:388`
  says the moment a sale happened is the server's to say, and `:461`
  hardcodes `issued_at: None` on the way in. A sale rung offline at 18:00 and
  synced at 19:30, after that cashier closed at 19:00, is therefore tagged
  rather than counted. Its cash was physically in the drawer at the count, so
  the close reads over by that amount and the tagged row is what explains it.
  The replay goes through `issue_idempotent` and must not be dropped in
  silence.
- **Two tills in one shop.** Rule 5 of `docs/architecture.md` says exactly one
  desktop is the server and the phone is a thin client ringing into it. The
  drawer is still per person: two people on one desktop each have their own
  shift, which is why the partial unique index is on `(shop_id, opened_by)`
  and not on `(shop_id)`. A physical second box is a `drawers` table and a
  wider index later, not a redesign. Named here so nobody builds it early.
- **Cash handed back.** Ruling 5. `cash.rs`'s `refunds` stops being a
  hard-coded zero and the cash paragraph's "nothing records cash handed back"
  goes. Three assertions pin today's zero and each is part of this change
  rather than collateral: `crates/core/tests/cash_service.rs:161` and `:343`,
  and the proptest invariant at `crates/core/tests/cash_prop.rs:153`. A task
  that loosens one instead of replacing it with a positive test of the new
  behaviour has not done the work.
- **A handover.** Not modelled, ruling 1b. No transfer row and no second
  stored figure. A cashier who hands their takings to the owner counts what
  is left and writes why in the note; the next person's shift opens with what
  they were handed as its opening cash. Both counts come out right with no
  new table. What is given up, named rather than found later: the file never
  says the two entries are the same money, so a theft dispute has two counts
  rather than one figure two people signed.

## The tasks

**T2: the migration, the models, the repos.** Files:
`crates/core/migrations/2026-09-21-000017_shifts/{up,down}.sql`,
`crates/core/src/schema.rs`, `crates/core/src/models/shift.rs`,
`crates/core/src/repos/shifts.rs`,
`crates/core/src/services/expenses.rs` (stamping),
`crates/core/tests/migration.rs`. Migration.

Done: the revert loop in `migration.rs` passes; a second open shift for the
same user is refused by the file while one for a different user is accepted;
a close whose counted figure differs from the expected one with no note is
refused by the file; an expense written at 00:30 on the shop clock is stored
at 00:30.

Three things that done-line does not say on its own, and which the task owes:

- Every constraint is proven through a raw `INSERT`, not through
  `services::shifts`, using `migration.rs`'s own `insert_with` and
  `insert_with_all` helpers. The service will repeat each check for a kinder
  message, and a test that goes through it passes with the index and the
  CHECKs deleted.
- The note CHECK is proven on both sides: a differing count with a note is
  accepted, an equal count without one is accepted. The refusal alone is
  satisfied by an unconditional `note IS NOT NULL`, which would break every
  clean close.
- The expense clock fix is proven on the rows written before the migration,
  not only on new writes, and on the way back down.
  `crates/core/tests/migration.rs:2280`,
  `the_audit_log_moves_onto_the_shop_clock_with_the_rows_already_in_it`, is
  the shape: seed a row at a UTC boundary, migrate, assert the stored value
  moved by the hour, revert, assert it moved back. Without it, a migration
  that stamps new rows and leaves the old ones alone, which is the bug this
  section exists for, passes.

`dz-money-builder`. Size M.

**T3: the service and the audit rows.** `services::shifts::{open, close,
open_for, sales_outside_a_shift, report}`, `cash::takings_for` and the
user-filtered repo queries under it, and the three audit actions. Files:
`crates/core/src/services/shifts.rs`, `cash.rs`, `audit.rs`, `documents.rs`,
`mod.rs`, `crates/core/src/repos/cash.rs`,
`crates/core/tests/shifts_service.rs`, and, once that suite crossed the
1200-line test limit, `crates/core/tests/shifts_who_may_close.rs` with the
fixtures both suites need in `crates/core/tests/common/shifts.rs`.

Spec: the cash position paragraph of §1, rewritten in this PR to say what a
shift adds; the fiscal rules table gains no row because no document changes
what it charges.

Done: a fixture with an opening cash figure, two cash sales and one cash debt
payment by the shift's own user, plus a card sale, a cash expense, a supplier
payment in cash and a second cashier's cash sale in the same window, none of
which may appear in the expected figure. The expected figure is written by
hand in the test and never computed by the code under test; the difference is
negative when counted is short; `REACHES_PAST_A_SIBLING` unchanged.

The second cashier's sale is the point of the fixture. Without it the test
passes against a shop-wide sum, which is the defect the money lens found.

The window comparison gets its own cases, because a fixture whose rows all
sit inside one shift passes with the comparison inverted: a sale at exactly
`opened_at` is in, a sale at exactly `closed_at` is out, and a sale whose
`issued_at` falls after its ringer's last shift closed with none open is
tagged as belonging to no shift.

`dz-money-builder`. Size M.

Two things T3 settled while it was built, written down because neither reads
off the code:

A clock that steps backward refuses the open. `open` rejects an `opened_at`
at or before that person's own last `closed_at`, so a shop PC whose clock
drifts back after sleep cannot open a till until the clock passes that
moment, and the refusal says which moment it is waiting for. Nothing else
writes `shifts`, and T4 keeps the moment off the wire, so a wrong clock is
the only caller that can reach the guard, which is what it is for. Importing
old shifts would need its own service function, and none is planned.

The other shop's rows in
`takings_for_answers_one_person_over_one_window_and_never_the_shop` are rows
only the fixture can write. Every user update reads the row through its own
`shop_id` and writes that same `shop_id` back, so nobody moves shops and one
user id never holds rows in two of them. The test stays anyway: shop 2's rows
against user 1 are the only way to tell the `shop_id` filter from the
`user_id` one, and with every row in one shop either filter can be deleted
and the test still passes.

**T4: the routes, the gates, the permission.** `POST /till/shifts`,
`POST /till/shifts/{id}/close`, `GET /till/shifts/open`,
`GET /till/shifts/{id}`; the gate rows; the permission; the tag hook for
ruling 2. There is no movements route: ruling 1 removed the movement. Files:
`crates/api/src/routes/till.rs`, `crates/api/src/dto/till.rs`,
`crates/api/src/gates/table.rs`, `crates/api/src/router.rs`,
`crates/core/src/services/permissions.rs`, `crates/core/src/services/sales.rs`,
`crates/api/tests/till_api.rs`, `crates/api/tests/route_gates.rs`,
`docs/features.md` §5.

`route_gates.rs` is on that list on purpose: its own test hardcodes which GET
paths may carry a permission at all, and neither till read is on it, so
gating `GET /till/shifts/{id}` on `see_reports` means widening that
allow-list in the same PR.

`routes/till.rs` stays one file. `crates/api/tests/one_handler_decides.rs`
refuses a route split into a folder, because its walk reads one directory
deep and skips every `mod.rs`, which is exactly where a folder route's
handler would live.

Neither moment reaches the wire. `NewShift::opened_at` and `TillCount::at`
are both `Option`, and the route passes `None` for each, the way
`NewSaleDto` hardcodes `issued_at: None` (`crates/api/src/dto/sales.rs:388`
says why: when a thing happened is the server's to say). A DTO field for
either is a client that can backdate a window and move an expected figure,
so neither DTO grows one. T3 refuses a moment in the future and one that
reaches back into the same person's last closed shift, but those guards are
the floor and not a licence to accept the field.

Done: `route_gates.rs` green with the new rows; a cashier's refusal names the
permission; a sale rung with no shift open returns 200 with the sale and
writes the `till.sale_outside_shift` audit row, and no request path can
refuse it; neither till DTO carries a moment, proved by the DTO test rather
than by reading. Size M.

**T5: opening and closing at the till.** The open popup on the first sign-in
of the day, pre-filled with the last close's counted figure and carrying the
"do not ask again" box (ruling 2b), and the close modal, which shows the
expected figure before the cashier types what is in the drawer (ruling 4) and
requires a note the moment the two differ.

`till.tsx` is pinned at 905 and may not grow, so every line goes in
`apps/desktop/src/routes/-till/session.tsx`, wired from `till.tsx` in one
import and one element. Files: the part file, `till.tsx` (a few lines),
`packages/shared/src/client/till.ts`, the three dictionaries.

Done: `till.test.tsx` is pinned at 1405, so new tests go in
`-till/session.test.tsx`, and they prove open, close, a short count shown
negative, the note field appearing and blocking the close when the figures
differ, and the refusal wording; the app under `just e2e` opens a shift,
sells, closes, and the difference printed matches the fixture.
`dz-builder`. Size M.

**T6: a refund that leaves the drawer, and the shift figure on screen.**
Ruling 5: a reversal may be settled in cash, so `Outgoings.refunds` stops
being a hard-coded zero. Both paths that reach it are in scope, the avoir
against a facture and the cancelled cash ticket, which is where an anonymous
customer's refund lands because there is no ledger to credit. Then the
dashboard's cash panel shows the opening cash and the expected figure while a
shift is open, in a component file because `dashboard.tsx` is pinned, and the
same panel sits beside the month's cash on `expenses.tsx`.

Files: `crates/core/src/services/cash.rs`, `avoir.rs`, `cancellation.rs`,
`crates/core/tests/cash_service.rs`, `crates/core/tests/cash_prop.rs`,
`apps/desktop/src/components/CashPanel.tsx` (lifted out of `expenses.tsx`,
which shortens it and lowers its entry), `dashboard.tsx` (imports only).

Before any of it, one thing the money lens found and the ruling does not
cover. `avoir::issue` credits the ledger for the whole amount, then settles
the facture it is written against, then the customer's other unpaid papers
oldest first, and only what none of them can take becomes credit the shop
holds. Settling in cash on top of that unchanged path pays twice: a customer
with a 20 000 DA avoir and a separate 15 000 DA unpaid facture would have
that facture cleared on the ledger and be handed the whole 20 000 in notes.
So cash settlement replaces the ledger legs for the amount handed over rather
than sitting beside them, and the migration's own CHECK says a row of
`kind = 'avoir'` may carry no `payment_mode`
(`crates/core/migrations/2026-09-09-000006_debt_payment_mode_check/up.sql`),
so the schema moves too. The anonymous cancelled ticket has no ledger row at
all and is the simpler arm.

Done: a cash refund lowers the day's cash figure and the open shift's
expected figure by the same centimes; a customer with another unpaid document
is not credited twice, proven by a test that fails against the ledger path
left unchanged; the three assertions pinning `refunds` at zero are replaced
by ones pinning the new behaviour rather than loosened; a partial avoir
refunds its share and no more; the stamp is still never given back; and the
cash paragraph of `docs/features.md` loses its "nothing records cash handed
back" sentence in the same PR.

One wire gap T5 hit and worked around rather than inventing a number for.
`ShiftReportDto` (`crates/api/src/dto/till.rs:85`) carries the shift, the
takings, the expected figure, the difference and the window's end, and nothing
that says what was rung while nobody held a shift. Ruling 2 accepts and tags
such a sale rather than refusing it, so the figure exists in the audit log and
has no way to reach a screen. Until it does, the close modal can only show a
drawer that reconciles against the shift's own window and stays silent about
the rest of the day, which is the honest shape but not the useful one. T6 adds
the field beside `takings` and the close screen reads it, because T6 is already
the task that moves what the cash panel answers.

This is the only task in the loop that changes a money figure the dashboard
already answers, so it goes to `dz-money-builder` and takes the extra layer
the quality gates ask for on money. Size M.

**T7: the closing sweep.** The shift report readable after close (a list
under `/till/shifts`), the e2e with a real cashier per the M4 rule, and the
docs sweep (`docs/features.md` §1 and §5, `docs/architecture.md`'s error
table, `.claude/stale-homes.md`, the `README.md` screens list). Done: the
e2e names the refusal codes apart; `stale-check` finds nothing.
`dz-builder`. Size S.

Built 2026-09-21 on `feat/the-shift-list-and-a-cashier-in-the-e2e` (list,
screen, e2e; docs sweep after T6 merges). Two things the review left
open on purpose. The list's default window is "today" by the shop clock
(`clock::now().date()`, Algiers); a slip to UTC would only show between
00:00 and 01:00 Algiers time, and no test pins that hour because the API
tests have no injectable clock (the same gap as the two wall-clock gates
in `gates-has-two-wall-clock-tests`). And the browser suite on `main`
went red with T5: the open dialog sits over `/till` in 27 specs that
never open a shift, and `just gates` does not run `just e2e`, so nothing
said so; the fix is in the sign-in fixture (`apps/desktop/e2e/auth.ts`
opens a shift at the door), on its own branch.

## How it is run

Two branches at once at most. `just ci <branch> full` on the mirror before
each merge, because a cash figure is on the path in T2, T3 and T6. The three
lenses per PR as always, and the advisor before merging T3 and T4, because
this is the phase with the most ways to be quietly wrong.
