---
title: 'Closing the Lumina gaps: the loop after the teardown'
slug: 'closing-the-lumina-gaps-loop'
status: 'active'
category: 'loops'
created: 20260920
tldr: 'Finishes what the weekend loop left open (four architecture tasks, the whole-loop review, the ESC/POS facture), then closes the three gaps the teardown found in our own spec: a print language the shop keeps, till shifts with an opening cash figure and a count, and Arabic on a cheap thermal head. Nothing is blocked: Samir took all nine rulings on 2026-09-20 and they are written at the top. A shift is per user with no drawers table, a sale rung with no shift open is tagged rather than refused, and the raster is built for 80 mm and golden-filed here because no printer exists yet to prove it.'
roadmap: 'catching-lumina'
---
# Closing the Lumina gaps: the loop after the teardown

Samir, 2026-09-20: turn the ranked gap summary into a roadmap an unattended
loop can work one task at a time. The input is
`research/competitors/2026-09-20-lumina-teardown/gap-summary.md`. The loop
before this one is `context/loops/20260917-catching-lumina-weekend-loop.md`;
its guardrails, its `ctx:` trailer shape and its "who does what" hold here
unchanged and are not repeated.

`research/README.md` governs everything below and was obeyed in writing it:
nothing from the teardown becomes source code. A Lumina screen is named only
as evidence that a shop expects a thing. Where this file says how to build
something, the design comes from `docs/features.md`, from the code that is
here, and from public standards (the ESC/POS raster command is Epson's, not
theirs). No file of theirs was read to write a task.

## Rulings Samir took, 2026-09-20

Answered in conversation, written here so the build does not ask again.
All of them are settled, including ruling 10 and the correction to ruling 3
that the first pass missed; see "Rulings still open" below, which says none
are. This line said 4 to 9 were still waiting until 2026-09-21, which was
true for the hour between the first three answers and the rest.

**The word.** A till session is called a **shift**, never a session.
`crates/core/src/services/sessions.rs` is the auth session, and
`sessions -> users -> sessions` is one of the three import rings still open.
A second meaning on that word in the same crate costs a reader every time.
`crates/core/src/print/facture.rs:31` already says "a till is reconciled by
shift", so the word is the repo's own.

**Ruling 1, whose shift.** A shift belongs to a user, not to the shop. One
drawer per user, and the drawer is the user: there is no `drawers` table. A
shift row is user, opened at, opening cash, closed at, counted, difference,
note. At most one open shift per user; shifts of different users overlap,
because each person's cash is physically their own. What the shift answers is
"whose count is short", not "whose sales", which `user_id` on the document
already answers.

**Ruling 1b, a handover.** Not modelled. There is no transfer row and no
second stored figure. A cashier who hands their takings to the owner counts
what is left and writes why in the shift's note; the next person's shift opens
with what they were handed as its opening cash. Both counts come out right
with no new table. What is given up: the file never says the two entries are
the same money, so a theft dispute has two counts rather than one figure two
people signed. Samir, on the shop floor: in Algeria the owner usually takes
the takings himself, so the note is for the days he cannot.

The dashboard does not read shifts at all in v1. `cash::position` keeps
deriving exactly what it derives today, from sales, the two ledgers and
expenses. Nothing about a shift enters it, so no handover can move the shop's
cash figure. This is the rule that keeps a 15 000 DA handover from reading as
the shop losing 3 000 on a day it took 12 000.

**Ruling 2, a sale with no shift open.** The sale rings. It is never refused.
`outcomeOf` in `apps/mobile/lib/outcome.ts:63` queues only on a null status or
a 5xx, so a 422 refusal on the phone throws the sale away after the customer
has handed over cash, and the phone rings cash only. Refusing protects no
number either: `documents::issue` takes the document number inside the
transaction after every check, so a refusal before it gaps nothing.

The sale is tagged as belonging to no shift. The close screen shows that cash
as its own line rather than folding it into the difference, otherwise a
cashier who never opened looks short by their whole morning.

**Ruling 2b, the shift opens at sign-in.** That makes ruling 2 a rare case
instead of the normal one. The first sign-in of the day raises one popup:
"Opening the till. Cash in drawer: [amount]", pre-filled with what the last
close counted, and a "do not ask again" box that opens silently at the
remembered amount from then on. One popup, not two: opening the shift and
stating the cash in the drawer are the same act. Unlocking after the idle
lock is not a sign-in and opens no second shift, so a till that locks every
fifteen minutes does not produce a dozen shifts a day.

Identity is free: `users::verify_pin` (`crates/core/src/services/users.rs:420`)
takes a `user_id`, because the pad picks a name off the list and the PIN is
checked against that row. PINs therefore need not be unique, and are not made
so: with four digits and a handful of staff, refusing a taken PIN says
somebody uses it.

The ticked box has one visible cost. If the owner empties the change overnight
and the shift opens at last night's figure, that close reads short by it. The
cashier sees it at the count and can correct it, so it is visible rather than
silent.

**Ruling 3, the phone's offline sale replayed late.** It belongs to the shift
open when it reaches the server. If none is open then, it is tagged as
belonging to no shift, under ruling 2.

Corrected the same day it was written. The first version of this ruling said
the sale belonged to the shift open when it was *rung*, and that nothing new
was needed to know which. That was wrong on the facts.
`crates/api/src/dto/sales.rs:388` says it in its own words: "`issued_at` is
not on the wire: the moment a sale happened is the server's to say, on the
shop's calendar, and a till with a wrong clock would otherwise date a fiscal
document." Line 461 hardcodes `issued_at: None` on the way in, and the
`createdAt` in `apps/mobile/lib/queue.ts` is local queue bookkeeping that
never reaches the server.

Satisfying the first version meant putting a client-controlled timestamp on
the sale wire, which reopens exactly the hole that comment closed, and buys a
worse one: a phone controlling that field could place a cash sale outside
every one of its own shifts, taking that cash off the count, with the only
trace in an audit log `see_audit_log` reserves to the owner. Arrival time is
the server's own and cannot be steered from the counter.

The cost, named rather than found later. A sale rung offline at 18:00 and
synced at 19:30, after that cashier closed at 19:00, is tagged rather than
counted. Its cash was physically in the drawer at the count, so the close
reads over by that amount and the tagged row is what explains it. The shift
report shows tagged sales beside the difference for that reason.

**Ruling 4, the count at close.** The expected figure is shown before the
cashier types what is in the drawer, not after. Samir, 2026-09-20: following
Lumina here, because confirming a figure is better than guessing one.

Checked by eye on `shots/android-fr-caisse-close-session-modal.png`: their
close sheet shows `Solde theorique` at the top, then a `Solde reel a la
fermeture` field, then a live `Caisse equilibree` line as the cashier types,
then an optional note whose placeholder asks for the reason for the gap.

One difference from theirs. Their note is optional whatever the count says.
Ours is required the moment the counted figure differs from the expected one,
because a difference with no reason is the one row an owner cannot act on.
Equal counts need no note.

**Ruling 4b, what the close leaves behind.** Opening and closing a shift each
write an audit row through `services::audit::record`, carrying the opening
cash, the expected figure, the counted figure, the difference and the note.

That is not enough on its own. `see_audit_log` is the owner's alone
(`docs/features.md` line 891), so a manager reading the audit log is refused.
The shift list is therefore its own screen under `see_reports`, reading the
`shifts` table, and the audit rows are the owner's second copy rather than the
only one. A manager who cannot review a day's shifts cannot run a floor.

What this gives up, stated rather than discovered later: a cashier who is
short can top the drawer up to the figure on the screen, and the shift then
reads balanced. A blind count would catch that. The shift is a bookkeeping
tool here, not a fraud control.

**Ruling 5, cash handed back.** The shop records it. Today
`services/cash.rs` hard-codes `refunds: Money::ZERO` and the cash paragraph of
`docs/features.md` says outright that nothing records cash handed back over
the counter. Both change: a reversal may be settled in cash, that cash is an
outgoing, and the shift's count knows about it.

Why it cannot stay as it is: `docs/features.md:154` allows an anonymous sale,
and most walk-in customers are one. An anonymous customer has no ledger to
credit, so credit-only means a shop that hands back 3 000 DA has no record of
it anywhere and the cashier is 3 000 short at close, every time, wearing it
themselves.

Two paths reach it and both are in scope:

- An avoir against a facture, where the money half today is a ledger credit
  (`services::avoir::issue`). Settling it in cash instead is the plain case.
- A cancelled cash ticket. `cancellation::effect_of` answers `StockBack` for a
  document that put no money on an account, and that arm returns the goods and
  writes nothing about money. An anonymous cash ticket handed back is exactly
  that arm, so it is where most real refunds land.

What moves: `Outgoings.refunds` stops being a hard-coded zero and the cash
paragraph loses its "nothing records cash handed back" sentence. No fiscal
rule changes: an avoir still takes its own number, still never refunds the
stamp, and the cap on the running total against one facture is untouched. What
changes is where the money is said to have gone, not what the paper charges.

**Ruling 7, the print language.** One setting, not two. It covers every
fiscal paper and every slip: the ticket, the facture, the customer statement
and the debt slip. Lumina keeps the invoice and the receipt apart; a shop that
prints its tickets in Arabic and its factures in French is unusual, and a
second setting is a second thing that drifts out of step with the first.

The setting is independent of the screen language, which is what the Scope
list at `docs/features.md:18` has said since v1 and what §4 admits the current
mechanism does not deliver.

The default is today's behaviour: a shop that never opens the settings panel
prints in the language the till is being used in, passed on the call as
`?lang=fr|en|ar`. It does not fall back to French. A fresh shop running an
Arabic till would otherwise print French paper until somebody found the panel,
and nobody would connect the two.

The barcode label and the import template keep following the caller's
language and are not driven by this setting. Neither is a fiscal paper, and a
label carries a name, a price and a barcode that are already on the shelf.

**Ruling 8, the thermal printer.** Unanswerable today and built anyway,
with the assumption named rather than hidden. There is no shop and no printer
in anyone's hand, so no hardware can confirm the raster.

The raster path is built for 80 mm at 576 dots, which is what a cheap
Algerian thermal head almost always is, and the width is a shop setting so a
58 mm head at 384 dots is a number rather than a second code path. The Arabic
renders to golden PNG files, so a reviewer checks the shaping without a
printer, the same way the ESC/POS byte dump is read today.

The phase's done-line says in plain words that no printer has confirmed it,
and the task stays open until one does. This follows the discipline the fiscal
rules table already uses for the droit de timbre base: build it, name the
assumption, and never claim a proof nobody has.

**Ruling 9, the unit list.** Ten units, up from four. Today's `piece, kg,
litre, box` gains `gramme`, `quintal`, `carton`, `paquet`, `casier` and
`douzaine`.

What is evidence and what is judgement, kept apart. The teardown recorded
Lumina's list as "quintal alongside kg, g, litre, and the packaging units a
grocery uses" (`findings.md` line 237) and never enumerated the packaging
ones; no screenshot of that dropdown was taken. So `gramme` and `quintal` come
from the competitor, and the four packaging units are a reading of how an
Algerian grocery talks, taken by Samir on 2026-09-20. A quintal is 100 kg and
is how grain, flour, potatoes and feed are bought here.

The one consequence. `docs/features.md:157` says a product whose unit is piece
or box is sold in whole units, so a typed "1,5" is refused. The four packaging
units join that rule: a carton, a paquet, a casier and a douzaine are whole or
they are nothing. `gramme` and `quintal` stay fractional like kg. No
arithmetic changes either way, because a quantity is already stored in
thousandths.

**Ruling 10, who opens and closes.** A cashier opens and closes their own
shift. Anyone else's is manager and owner. Samir, 2026-09-20.

Ruling 2b depends on it: the shift opens at the first sign-in of the day, and
the person signing in at a till is usually the cashier. A permission they do
not hold would break that on every counter.

One route serves both cases, so the coarse gate and the fine check are
separate, the shape `DiscountAboveThreshold` and `ChangePriceAtTheTill`
already use under the `Sell`-gated `POST /sales`: `gates.rs` gates the close
route on `open_and_close_till`, which all three roles hold, and
`services::shifts::close` calls `permissions::require` for the
close-someone-else case when `opened_by` is not the caller.

The two reads are gated apart, because one is a cashier's own till and the
other is management. `GET /till/shifts/open` answers the caller's own open
shift and needs no permission, since T5's close modal cannot show the
expected figure without it. `GET /till/shifts/{id}` and the day's list are
`see_reports`, because "whose count was short" is a management figure.
`crates/api/tests/route_gates.rs` hardcodes which GET paths may carry a
permission at all, and neither till path is on that list, so T4 widens it and
names that file among the ones it touches.

## Rulings still open

None. Every question this file opened was answered on 2026-09-20 and written
above, including two the first pass missed: who may open and close a shift
(ruling 10), and what a replayed offline sale attaches to once it was clear
that `issued_at` is not on the wire (ruling 3, corrected the same day).

Rulings 8 and 9 were taken without the hardware and the competitor listing
that would have settled them outright, and each says so in its own words
rather than reading as a fact.

## Where the weekend loop actually stands, checked against the repo

The loop file of 2026-09-17 says Phase 1 is invoice layouts, Phase 2 the
phone, Phase 3 the architecture fixes, Phase 4 a whole-loop review. The repo
on 2026-09-20 says:

- Phase 0, the dto split: merged (#109).
- Phase 1, invoice layouts: the plan is done and archived
  (`context/plans/archived/20260917-invoice-layouts-a-shop-can-choose.md`).
  The picker (#111), the compact A4 (#112), the half sheet (#112), the
  80 mm roll facture (#113) and its e2e (#114,
  `apps/desktop/e2e/zzzz-facture-layouts.spec.ts`) are all on main. The
  one thing left is T7, the 80 mm facture down the ESC/POS path, parked
  on purpose. It is re-homed below as the thermal plan's last task, so a
  parked task does not sit in an archived page.
- Phase 2, the phone: the plan is done (`status: 'done'`, #115 to #117,
  #122). What is left needs a handset in Samir's hand, not a task here:
  Arabic layout on a real device, which side the `دج` suffix lands on
  beside Latin digits under RTL, and the Maestro flows that still assert
  English sentences (`apps/mobile/maestro/*.yaml`). Listed under "waits for
  Samir" at the end.
- Phase 3, architecture: T2, T3, T5, T6, T7 and T8 are done (#109, #118 to
  #121, #124, #125). Two stay open and are the unfinished work this loop
  starts with:
  - T4: done on 2026-09-21. `REACHES_PAST_A_SIBLING` in
    `crates/core/tests/services_go_through_services.rs` is down to two rows
    and three reaches (#132, #133, #135, #136), from seventeen across eleven
    when it was written. What is left is thick rather than accidental and
    each row says why: `debt -> customers` is four calls from `customers.rs`
    into `services::debt`, one of them the ledger write, and the two
    supplier rows are five calls from `suppliers.rs`. The constant is not
    deleted; see `plan:the-last-six-reaches-stand-behind-rings`.
  - T9: done, #129. `PurchaseDto` answers `extras_centimes` and both screens
    print it; neither names `transport_centimes` or `extra_costs_centimes`
    any more.
  - T10: `scripts/file-sizes.json` holds 26 entries (from 32);
    `suppliers.tsx` left the list with #119 and
    `packages/shared/src/client.ts` left it when the client split shipped.
    `till.tsx` (905), `dashboard.tsx` (659), `expenses.tsx` (668) and
    `crates/api/src/lib.rs` (710) are pinned and may not grow, which shapes
    every task below that touches them.
  - T11: `RINGS_STILL_OPEN` pins three rings through `users`, `sessions`,
    `audit` and `preferences`. Its page is not written.
- Phase 4, the whole-loop review on the sharper model and the standup:
  nothing in the log or in `context/` shows it ran. It is open and is this
  loop's closing phase, over both loops' merges.

So the board is not empty: four tasks and a review carry in, and they are
placed before or between the gap work below rather than after it.

## The ranking, tested

The summary ranks till sessions, then a stored print language, then Arabic
on a cheap head, each because our own spec admits the gap. Every citation
was checked and holds: the cash position paragraph (`docs/features.md`
lines 287 to 310) says "no opening float and no count at close" and
"nothing records cash handed back"; the Scope bullet (line 18) says UI
language and print language are independently selectable while §4 (lines
835 to 839) says there is no separate print-language setting in v1, and §8
(line 1106) repeats it; the Thermal bullet (lines 861 to 867) says a cheap
head will need a different codepage. Nothing in the summary was found wrong.

As a ranking by the size of the admitted gap, the summary is right and this
file keeps it: till sessions is the biggest hole a shop meets every evening.

As a build order for an unattended loop it is wrong, because the
discriminator that matters tonight is what can start with no ruling and no
device in hand. The print language needs neither: one preference key, one
route, one panel, and the six print routes reading the key when the caller
names no language. Till sessions cannot cut a schema until rulings 1 to 6
are answered, and the schema is where every other task in that phase hangs.
The Arabic raster can be built and golden-filed here, but whether it prints
is a question only a printer answers, and the only printer is the one Samir
has held since M1 T6.

So the order is: room to add, then the print language and the carried-in
architecture tasks while Samir answers the rulings, then till sessions, then
the raster, with the rings and the file splits as filler whenever an agent
slot is free.

## Phase 0 (done 2026-09-20): room to add, alone, before anything else

Both slices merged: the gates table is a folder (#127) and the shared client
is split by domain (#126). What follows is the record of what was decided,
kept because the reasoning still governs what may be added to those files.

Two files sat on the line and every task below adds to them. The same reason
the dto split went first last time.

`architecture-fixes-without-a-domain-split` / T10, first two slices.

**T10 slice 1: `crates/api/src/gates.rs` gets a folder.** It is 600 lines
and `scripts/file-sizes.mjs` refuses `count > 600`, so the first new gate
row (the print language's `PUT /settings/print-language`) fails `just
sizes`. Move the table to `gates/table.rs` and the in-file tests (line 465
on) to a `#[cfg(test)] mod tests;` sibling at `gates/tests.rs`, not to
`crates/api/tests/`: an integration test cannot see `pub(crate)`, and the
sibling keeps the visibility as it is. `scripts/file-sizes.mjs` classes a
file under `src/` as source, so the sibling has the 600 limit and is far
under it. Files: `crates/api/src/gates.rs`, `crates/api/src/gates/`. Done:
`just gates` green with no gate row added or changed, and no file under
`gates/` over 400 lines. `dz-builder`. Size S.

**T10 slice 2: `packages/shared/src/client.ts` splits by domain.** Pinned
at 994; every task here adds a method (set print language, open session,
paid out, close session). `client/{sales,settings,customers,...}.ts` with
`client.ts` re-exporting so no import elsewhere changes. Files:
`packages/shared/src/client.ts`, `packages/shared/src/client/`,
`scripts/file-sizes.json` (the entry goes). Done: `npx tsc --noEmit` in
`apps/desktop` and `apps/mobile` clean, vitest count unchanged, entry
deleted. `dz-builder`. Size S.

## Phase 1 (done 2026-09-21): the print language, and the carried-in architecture work

All of it merged: the shop stores the language its paper prints in (#130)
and the six fiscal papers print in it (#131), with the carried-in reach work
in #128, #132, #133, #135 and #136. `REACHES_PAST_A_SIBLING` is down to two
rows. What follows is the record of what was decided.

Plan slug `a-print-language-the-shop-keeps`. The plan page is written and
committed to `main` (context bookkeeping) before any builder commits, or the
`ctx:` hook burns the first attempt. Up to three branches at once.

**T1: the page.** What the setting is, what reads it, what does not, the
ruling 7 answer once it comes. Session, straight to `main`.

**T2: the key, the route, the panel.** A `print_lang` key in `preferences`
(the table has no CHECK; the service parses `fr|en|ar` the way
`preferences::theme` parses a theme, and an unknown value reads as no
preference). `services::preferences::{print_lang, set_print_lang}`,
`routes::settings::set_print_lang`, a gate row under `edit_settings`, a
`PrintLangDto` in `crates/api/src/dto/settings.rs`, a `PrintLanguagePanel`
beside `FactureLayoutPanel.tsx` on `settings.printing.tsx`. Files:
`crates/core/src/services/preferences.rs`, `crates/api/src/routes/settings.rs`,
`crates/api/src/dto/settings.rs`, `crates/api/src/gates/table.rs`,
`crates/api/src/router.rs`, `packages/shared/src/client/settings.ts`,
`apps/desktop/src/components/settings/PrintLanguagePanel.tsx`,
`apps/desktop/src/routes/settings.printing.tsx`, the three desktop
dictionaries. No migration. Done: a `PUT` persists per shop and survives a
restart; a cashier gets `forbidden` with `edit_settings`; the panel test
mutates and reads back. `dz-builder`. Size S.

**T3: the print routes read it.** The six document routes (`/sales/{id}/ticket`,
`/ticket/escpos`, `/ticket/print`, `/sales/{id}/facture`,
`/customers/{id}/statement`, `/customers/{id}/debt-slip`) take `?lang=` as
an override, the way `?layout=` already wins for a preview at
`routes/sales.rs:293`, and read `preferences::print_lang` when it is absent.
What prints when nothing is set is the third part of ruling 7; until it is
answered the desktop keeps passing its UI language, so nothing a shop sees
changes, and the setting wins over it once one is stored. The T1 page
writes the precedence down in one line (`?lang`, then the preference, then
the fallback ruling 7 names) and the core test pins all three. Labels and the import template keep the
caller's language until ruling 7 says otherwise. Files:
`crates/api/src/routes/sales.rs`, `crates/api/src/routes/customers.rs`,
`packages/shared/src/client/sales.ts`, the desktop print callers
(`grep -rn 'lang=' packages/shared/src/client`), `docs/features.md` §4 the
print-language bullet and §8 the Language paragraph, both rewritten in the
same PR. Spec row: `amount_in_words` in the totals table, "`net_to_pay`
written out in the print language", satisfied by the existing goldens per
language, which do not change. Done: an e2e in
`apps/desktop/e2e/settings.spec.ts` sets Arabic with the UI in French,
prints a ticket and asserts the returned page is `lang="ar" dir="rtl"`; a
core test proves the override beats the setting and the setting beats the
fallback. `dz-builder`, because the words change and the numbers do not.
Size S.

Alongside, the carried-in architecture tasks, slug
`architecture-fixes-without-a-domain-split`:

**T9: the API answers the purchase's landed extras.** `PurchaseDto` gains
`extras_centimes` (transport plus extra costs, checked in the core) and the
two components print it. Files: `crates/api/src/dto/purchases.rs`,
`crates/core/src/services/purchases.rs`, `apps/desktop/src/routes/purchases.tsx`,
`apps/desktop/src/routes/purchases_.$id.tsx` (pinned at 610, this only
removes lines), `crates/api/tests/purchases_api.rs`. Spec: the Purchase
paragraph of §1 and migration 000008's `transport_centimes` and
`extra_costs_centimes` comments; no fiscal row, nothing charged changes.
Done: the two components read the total the API answers and compute none of
their own, proven by mutation rather than by shape: put the hand-rolled sum
back and the test goes red. A lint matching a literal `+` is not enough on
its own, because a subtraction of a negative, a `reduce`, a line break or the
same sum moved into a helper all walk past it. `just ci <branch> full` green
on the mirror.
`dz-money-builder`. Size S.

**T4, in four branches, `progress` on each and `close` on the last.**
Every one shortens `REACHES_PAST_A_SIBLING`; none may lengthen it.

A row is one (service, repo) pair and leaves only when every call in that
service to that repo has moved, so a branch is sized by whole rows.

- T4 branch a, reads through documents: `avoir`, `customers` and `export`
  reach `repos::documents` for `get`, `unpaid_of_customer` and
  `list_in_range`. `services::documents` gains each, the callers switch.
  Three rows leave. `dz-builder`. Size S.
- T4 branch b, `cash -> expenses` and `dashboard -> debt, supplier_debt`:
  `services::expenses::total_between` and `services::{debt,supplier_debt}::balances`
  are added and called. The sums stay where they are; only the door moves.
  Three rows leave. `dz-builder`, hands back if a sum has to move. Size S.
- T4 branch c, the product cost: `import -> products` (`by_barcode`) and
  `purchases -> products` (`get`, `set_cost`, where `set_cost` writes a
  landed cost the purchase computed). Spec: the Cost of goods sold row for
  `set_cost`. Both are ring-free: `import.rs` already imports
  `services::products`, and `products`' whole import closure (audit,
  categories, stock, users, sessions, preferences, clock) reaches back to
  neither caller. Two rows leave. `dz-money-builder`, and `just ci <branch>
  full` on the mirror. Size S.

  The debt rows this branch used to carry moved to branch d on 2026-09-21,
  on reading the import graph rather than on trying it: `customers.rs:22`
  imports `services::debt` and `documents.rs:23` imports
  `services::customers`, so routing `debt` through either sibling's service
  closes a ring the ring test refuses to grow. The row cannot leave by
  moving the door; it needs the shape branch d already carries.
- T4 branch d, the rows a ring stands in front of. Two groups, one shape.
  The mutual pair, `purchases -> supplier_debt` and `supplier_debt ->
  purchases, suppliers`. And the debt rows branch c handed over,
  `debt -> documents` and `debt -> customers`: `debt.rs:581` writes
  `documents_repo::set_remaining_debt`, and `remaining_debt` is a field of
  the §3 totals table ("from the ledger at issue time", the balance triple
  paragraph under it), with the reads beside it (`kinds_and_numbers`, `get`,
  `belongs_to_shop`) leaving in the same branch or the row staying. Routing
  any of them through the other's service makes a ring the ring test
  refuses, so this is the T3 shape: the piece both need moves to a module
  below both, the way `services::pricing` went under `sales` and `proforma`.
  Before it starts, the read that decides the shape is what `customers.rs`
  uses from `debt` and what `documents.rs:135` uses from `customers`: if
  either is one operation, lifting that one above both is cheaper than a new
  module. Five rows leave, the constant is deleted with them, and
  `RINGS_STILL_OPEN` does not grow. `dz-money-builder`, a supplier balance
  and a customer's remaining debt are both on the path. Size L.

Three, three, two and five: the thirteen rows, and the constant is deleted
with the last one. Branch c gave three of its four to branch d on
2026-09-21, for the ring reason written under it.

Before Phase 2 starts: the advisor, with the per-PR Sonnet lenses having
run on each Phase 1 merge as always, because Phase 2 builds on `cash.rs`
and `expenses.rs`. The sharper model runs once per loop, at the close, as
the 2026-09-17 ruling says; not here.

## Phase 2 (in flight): till shifts

Plan slug `till-shifts-a-float-and-a-count`. `dz-money-builder` end to end:
the schema, and every figure is centimes. Two branches at once at most.
`just ci <branch> full` on the mirror before each merge.

Rewritten 2026-09-20 after Samir's rulings above. The earlier draft of this
phase cut two tables and put `paid_in` and `paid_out` movements into
`cash::position`. He asked for the smaller shape and took it: one table, and
the dashboard untouched. What follows is that shape.

### What it does to the file

One new table in one migration, no ALTER of an existing one except the clock
fix below.

- `shifts`: `id`, `shop_id`, `opened_by` (users), `opened_at` (shop clock),
  `opening_cash_centimes >= 0`, `closed_at` nullable, `closed_by` nullable,
  `counted_centimes` nullable, `expected_at_close_centimes` nullable, `note`
  nullable. STRICT, `shop_id` everywhere, integer centimes, the rules every
  migration since 000000 follows.
- A partial unique index on `(shop_id, opened_by) WHERE closed_at IS NULL` is
  what makes ruling 1's "at most one open shift per user" a fact the file
  holds rather than the service. Per user, not per shop: shifts of different
  people overlap on purpose, because each person's cash is their own.
- A CHECK ties the close columns together: `closed_at`, `closed_by`,
  `counted_centimes` and `expected_at_close_centimes` are all null or none.
- A second CHECK: `note` is NOT NULL whenever `counted_centimes` differs from
  `expected_at_close_centimes`. Ruling 4 requires a reason for a gap and
  nothing else in the file can enforce it.

There is no `cash_movements` table and no `drawers` table. Cash handed to the
owner is the note at close, and an expense paid out of the shop's money is an
`expenses` row as it is today, recorded by a `commit_money` holder. Neither
one touches a cashier's expected figure, for the reason the next section
gives. Both were ruled today. If a shop later asks
for the individual movements, that is a table added then, not now.

`expected_at_close_centimes` is the one stored figure and it is stored on
purpose. The live position stays derived on every read (rule 4, and the cash
paragraph's "stored nowhere"). But a shift's difference is a fact about one
evening: a ticket annulled on Wednesday leaves Monday's takings, so a derived
expected figure would move the Monday count after it was signed. The snapshot
is what the closer saw; the difference is `counted - expected_at_close`, both
stored, so it is checkable and does not drift.

### What the expected figure is made of

Rewritten a second time on 2026-09-20, after the money lens read the first
version against `crates/core/src/repos/cash.rs` and found it counting other
people's sales. What follows is the corrected shape.

A shift's expected figure is:

    opening_cash + that user's cash sales + that user's cash debt payments

and nothing else. Every term is `checked_*`.

**Why the user filter is not optional.** `repos::cash` has no `user_id` in it
anywhere: `sales`, `customer_payments` and `supplier_payments` each filter on
`shop_id` and a time range and nothing more. Ruling 1 lets two cashiers hold
overlapping shifts, so a shop-wide sum handed to each of them counts the other
one's takings. Two cashiers each ringing 1 000 000 centimes between 09:00 and
14:00 would each be told to expect 2 000 000, each be recorded 1 000 000
short, each be made to write a note, and each get an audit row saying so, for
doing nothing wrong. `documents` carries `user_id` and so does `debt_ledger`,
so the filter is a where clause rather than a redesign.

**Why expenses and supplier payments are not in it.** Both are `commit_money`
work, which a cashier does not hold: they are the manager's payments out of
the shop's money, not out of this cashier's drawer. Taking them out is not
only correct, it removes a problem that had no cheap fix.
`repos::expenses::total_between` filters `expense_date`, which is a date and
not a moment, so a shift running 09:00 to 14:00 would have pulled the whole
day's expenses including one filed at 20:00 by somebody else. There is no
shift-sized slice of that column to ask for.

**Which timestamp.** `issued_at` on documents, `created_at` on `debt_ledger`.
Both are stamped by the services from the shop clock:
`services::sales::issue` sets `issued_at` to `clock::now()` when the caller
gives none (`crates/core/src/services/sales.rs:233`), and `NewSaleDto` cannot
give one (`crates/api/src/dto/sales.rs:388`, `:461`); `repos::debt` stamps
`created_at` the same way. `repos::cash::sales` already reads `issued_at`, so
a shift and the dashboard read the same column.

This is also what satisfies ruling 3 with nothing added. A sale the phone
queued offline is stamped when the server processes it, so `issued_at` is its
arrival, which is the moment the ruling attaches it to. `documents.created_at`
is left alone: it takes SQLite's UTC default and no repo stamps it, so it
would have carried the same hour error the expenses column does, and nothing
here reads it.

### The expense clock, still worth fixing

`expenses.created_at` takes `DEFAULT (CURRENT_TIMESTAMP)` from
`crates/core/migrations/2026-09-10-000008_suppliers_purchases_expenses/up.sql`,
which SQLite answers in UTC while Algiers is an hour ahead. No shift figure
depends on it any more, so it stops being load-bearing here, but it is still
wrong for anything that reads an expense by the hour. It ships in this
migration in the 000013 shape: stamp on write, and move the rows written so
far by the hour they are short.

### What it does to the services

A new `services::shifts` asks `services::cash` for its figures and never
touches `repos::cash` or `repos::expenses`, or `REACHES_PAST_A_SIBLING` grows
on its first commit, just after T4 branch b shortened it.

`services::cash` gains one function for this, `takings_for(conn, shop_id,
user_id, from, until)`, answering that person's cash sales and cash debt
payments over a window. `cash::position` is untouched: it keeps summing the
shop over a day or a month, from sales, both ledgers and expenses, exactly as
it does today. A shift is a second, narrower question put to the same module,
not a new input to the old one. That is what keeps a shift, a handover or a
refund from moving the shop's cash figure.

`shifts::sales_outside_a_shift` is a `documents` question, not a `cash` one:
which of this user's sales fall in none of their own windows. It goes through
a helper on `services::documents` rather than a query of its own, because a
`("shifts", &["documents"])` row is exactly what the burn-down list refuses,
and the plan's own guardrail says that list only shortens. `services/documents.rs`
is on T3's file list for that reason.

A sale rung while the ringer has no open shift is accepted and tagged, never
refused (ruling 2). The tag is derived, not stored: a sale belongs to no shift
when its `issued_at` falls outside every `[opened_at, closed_at)` of that
`user_id`. The close screen reads that as its own figure beside the expected
one, so a morning rung before the shift opened does not read as a shortage. A
replayed offline sale lands the same way, on `issued_at`, which is the moment
the server stamped it on arrival and not the moment the phone rang it up
(ruling 3).

Three audit actions: `till.open` (opening cash), `till.close` (expected,
counted, difference, note, and the opener's id when the closer differs), and
`till.sale_outside_shift` on a tagged sale. The features §5 audit list gains
the three.

The shift list is its own screen under `see_reports`, not a view of the audit
log, because `see_audit_log` is the owner's alone (`docs/features.md` line 891)
and a manager who cannot read a day's shifts cannot run a floor.

Permissions: opening and closing is a new permission, `open_and_close_till`,

held by all three roles under ruling 10. It shipped with T4 alongside a second
one, `close_another_persons_till`, which is Owner and Manager only and is
asked for inside the service when the closer is not the opener. The two took
`permissions.rs` from thirteen to fifteen, the match refused to compile until
all three roles placed each, §5 of `docs/features.md` moved with them, and so
did the `.claude/stale-homes.md` row. The gates module keeps no permission
count of its own: a fine permission checked in a service gets no gate row.

### The awkward cases, each with what the schema does about it

- A shift across midnight: a session is a shift, not a day. The dashboard's
  day and month figures keep their calendar definition; the shift report
  reads the shift window. Both are true at once and the page says which
  screen answers which question.
- Closed by a different user: `closed_by` is its own column and the audit row
  names both. A cashier closes their own and nobody else's (ruling 10), and
  the check is inside `services::shifts::close` rather than on the route,
  because one route serves both cases.
- A sale with no shift open: accepted and tagged, never refused (ruling 2).
  Nothing is stored on the sale: a sale belongs to no shift when its
  `issued_at` falls outside every `[opened_at, closed_at)` of its `user_id`,
  and `shifts::sales_outside_a_shift` is that one query. It writes an audit
  row `till.sale_outside_shift`.

  This is the answer that keeps the fixtures as they are. Seven desktop e2e
  specs (`till`, `till-credit`, `till-facture`, `till-cashier`,
  `till-reversals-and-quotations`, `settlement`, `scanner`), `just seed`, the
  demo recordings and the Maestro flows all ring sales with no shift open.
  Refusing would have meant opening a shift in every one of them.
- The phone's queue replaying after close: the sale belongs to the shift open
  when it reaches the server, read off `issued_at`, which is that arrival
  stamp (ruling 3). Nothing new is needed to know which, but not for the
  reason this bullet gave until 2026-09-21: `NewSaleDto` carries no
  `issued_at` at all. Its own doc says the moment a sale happened is the
  server's to say, `crates/api/src/dto/sales.rs:461` hardcodes
  `issued_at: None`, and the `createdAt` in `apps/mobile/lib/queue.ts` is
  local queue bookkeeping that never leaves the phone. If no shift is open on
  arrival, the sale is tagged as belonging to none rather than reopening a
  signed count. The replay goes through `issue_idempotent` and
  must not be dropped in silence, which T8 of the architecture plan found the
  phone doing once already.
- Two tills in one shop: rule 5 of `docs/architecture.md` says exactly one
  desktop is the server, and the phone is a thin client that rings into it.
  The drawer is still per person, not per machine: two people on one desktop
  each have their own shift, and the partial unique index is on
  `(shop_id, opened_by)` and not on `(shop_id)`. A physical second box is a
  `drawers` table and a wider index later, not a redesign; the page names
  that so nobody builds it early.
- Cash handed back: ruling 5. `cash.rs`'s `refunds` stops being a hard-coded
  zero and the cash paragraph's "nothing records cash handed back" goes.
  Three assertions pin today's zero and each is part of this change rather
  than collateral: `crates/core/tests/cash_service.rs:161` and `:343`, and
  the proptest invariant at `crates/core/tests/cash_prop.rs:153`. A task that
  loosens one instead of replacing it with a positive test of the new
  behaviour has not done the work.

### The tasks

**T1: the page.** The design above, the schema, the awkward cases as
decided. Every ruling is taken and written at
the top of this file. Bookkeeping, straight to `main`.

**T2: the migration, the models, the repos.** Files:
`crates/core/migrations/2026-09-21-000017_shifts/{up,down}.sql`,
`crates/core/src/schema.rs`, `crates/core/src/models/shift.rs`,
`crates/core/src/repos/shifts.rs`,
`crates/core/src/services/expenses.rs` (stamping),
`crates/core/tests/migration.rs`. Migration. Done: the revert loop in
`migration.rs` passes, a second open shift for the same user is refused by
the file while one for a different user is accepted, a close whose counted
figure differs from the expected one with no note is refused by the file, and
an expense written at 00:30 on the shop clock is stored at 00:30.

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
  moved by the hour, then revert and assert it moved back. Without it, a
  migration that stamps new rows and leaves the old ones alone, which is the
  bug this section exists for, passes.

Size M.

**T3: the service and the audit rows.** `services::shifts::{open, close,
open_for, sales_outside_a_shift, report}`, the `cash::position` window, and
the three audit actions. Files: `crates/core/src/services/shifts.rs`,
`crates/core/src/services/cash.rs`, `crates/core/src/services/audit.rs`,
`crates/core/src/services/documents.rs`, `crates/core/src/services/mod.rs`,
`crates/core/tests/shifts_service.rs`, and, once that suite crossed the
1200-line test limit, `crates/core/tests/shifts_who_may_close.rs` with the
fixtures both need in `crates/core/tests/common/shifts.rs`.
Spec: the cash position paragraph of §1, rewritten in this PR to say what a
shift adds; the fiscal rules table gains no row because no document
changes what it charges. Done: a fixture with an opening cash figure, two cash sales and one cash debt
payment by the shift's own user, plus a card sale, a cash expense, a supplier
payment in cash and a second cashier's cash sale in the same window, none of
which may appear in the expected figure. The expected figure is written by
hand in the test and never computed by the code under test; the difference is
negative when counted is short; `REACHES_PAST_A_SIBLING` unchanged.

The second cashier's sale is the point of the fixture. Without it the test
passes against a shop-wide sum, which is the defect the money lens found:
`repos::cash` filters on `shop_id` and a time range and carries no `user_id`
at all, so two overlapping shifts would each be told to expect the other's
takings and each be recorded short by it.

The window comparison gets its own cases, because a fixture whose rows all
sit inside one shift passes with the comparison inverted: a sale at exactly
`opened_at` is in, a sale at exactly `closed_at` is out, and a sale whose
`issued_at` falls after its ringer's last shift closed with none open is
tagged as belonging to no shift. The column is named in each of these on
purpose: `documents.created_at` takes SQLite's UTC default and would tag a
sale wrong by an hour at every shift boundary, which is the error these
boundary cases exist to catch and which they would pass against. Size M.

**T4: the routes, the gates, the permission.** `POST /till/shifts`,
`POST /till/shifts/{id}/close`, `GET /till/shifts/open`,
`GET /till/shifts/{id}`; the gate rows; the permission; the tag hook for
ruling 2. There is no movements route: ruling 1 removed the movement. Files: `crates/api/src/routes/till.rs`,
`crates/api/src/dto/till.rs`, `crates/api/src/gates/table.rs`,
`crates/api/src/router.rs`, `crates/core/src/services/permissions.rs`,
`crates/core/src/services/sales.rs`, `crates/api/tests/till_api.rs`,
`crates/api/tests/route_gates.rs`, `docs/features.md` §5.

`route_gates.rs` is on that list on purpose: its own test hardcodes which GET
paths may carry a permission at all, and neither till read is on it, so
gating `GET /till/shifts/{id}` on `see_reports` means widening that
allow-list in the same PR. Done: `route_gates.rs` green with the new rows; a
cashier's refusal names the permission; a sale rung with no shift open
returns 200 with the sale and writes the `till.sale_outside_shift` audit row,
and no request path can refuse it. Size M.

**T5: opening and closing at the till.** The open popup on the first sign-in
of the day, pre-filled with the last close's counted figure and carrying the
"do not ask again" box (ruling 2b), and the close modal, which shows the
expected figure before the cashier types what is in the drawer (ruling 4) and
requires a note the moment the two differ. `till.tsx` is pinned at 905 and may not
grow, so every line goes in `apps/desktop/src/routes/-till/session.tsx`,
wired from `till.tsx` in one import and one element. Files: the part file,
`till.tsx` (a few lines), `packages/shared/src/client/till.ts`, the three
dictionaries. Done: `till.test.tsx` (pinned at 1405, so new tests go in
`-till/session.test.tsx`) proves open, close, short count shown negative, the note field appearing
and blocking the close when the figures differ, and the refusal wording; the app under `just e2e` opens a session, sells,
closes, and the difference printed matches the fixture. Size M.

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
left unchanged; the three assertions that pin
`refunds` at zero (`cash_service.rs:161`, `:343`, `cash_prop.rs:153`) are
replaced by ones that pin the new behaviour rather than loosened; a partial
avoir refunds its share and no more; the stamp is still never given back; and
the cash paragraph of `docs/features.md` loses its "nothing records cash
handed back" sentence in the same PR. Size M.

This is the only task in the loop that changes a money figure the dashboard
already answers, so it goes to `dz-money-builder` and takes the extra layer
the quality gates ask for on money.

**T7: the closing sweep.** The shift report readable after close (a list
under `/till/shifts`), the e2e with a real cashier per the M4 rule, the
docs sweep (`docs/features.md` §1 and §5, `docs/architecture.md`'s error
table, `.claude/stale-homes.md`, `README.md` screens list). Done: the
e2e names the refusal codes apart; `stale-check` finds nothing. Size S.

Before merging T3 and T4: the three lenses per PR as always, and the
advisor, because this is the phase with the most ways to be quietly wrong.

## Phase 3: Arabic on a cheap thermal head

Plan slug `arabic-on-a-cheap-thermal-head`. Ruling 8 sets the width; the
work does not wait on it, 576 is the default and a preference.

What exists: `crates/core/src/print/escpos.rs` (371 lines) writes `ESC t 19`
and sends anything outside ISO 8859-15 as UTF-8, which is where a cheap head
prints boxes. The ticket already exists as a line model in `ticket.rs`, which
is what makes a raster possible without a browser: the same lines, drawn
into a 1-bit bitmap and sent with `GS v 0`, print on any head that has a
raster command, which is nearly all of them, and need no codepage at all.

**T1: the page, with the build-or-buy call.** Shaping Arabic, running the
bidi algorithm and rasterising glyphs are three commodity jobs and the
global rule says look before hand-rolling. Candidates to verify on
crates.io, not chosen here: `cosmic-text` (shaping, bidi and layout in one
crate, the heaviest build), or `rustybuzz` plus `ab_glyph` plus
`unicode-bidi` (which is a dev-dependency here already, and becomes a
runtime one). Either way an OFL Arabic font is vendored (Noto Naskh Arabic
or Amiri) the way Plex is. The page records the choice, the build time it
adds, and that it compiles on the Windows release target, before T2 starts.
The one thing not on the table is the `escpos` crate replacing `escpos.rs`:
that module is golden-filed and the raster command is thirty lines beside
it. Session, straight to `main`.

**T2: the raster writer and its goldens.** `print::raster` turns the
ticket's lines into a bitmap at the head's width and `escpos.rs` emits it in
`GS v 0` bands. The dump decodes `<raster WxH>` and writes the bitmap out as
PNG, so `fixtures/print/ticket_80mm_escpos/ar*.png` are goldens a reviewer
opens. Files: `crates/core/src/print/raster.rs`, `crates/core/src/print/escpos.rs`,
`crates/core/fonts/`, `crates/core/tests/print_ticket_escpos.rs`, the
fixtures. Spec: §4 Thermal bullet, rewritten in this PR. Done: the test
asserts the raster's source lines equal, string for string, the lines the
text path emits for the same fixture, so a number cannot differ between the
two; no line is clipped at the width; the three Arabic goldens exist and the
French and English bytes are unchanged. `dz-money-builder`: printed totals
become pixels here and the line-equality test is what keeps them honest.
Size L, the one task in this file that may take a whole night.

**T3: when the raster is used.** A `thermal_mode` preference (`text` or
`raster`, default `text`) beside `print_lang`; Arabic always rasters,
because there is no single-byte path for it that a cheap head has; French
and English raster when the preference says so. The route reads it, the
printing panel shows it, the phone inherits it through the desktop's spool.
Files: `crates/core/src/services/preferences.rs`,
`crates/api/src/routes/sales.rs`, `crates/api/src/routes/settings.rs`,
`crates/api/src/gates/table.rs`, the panel on `settings.printing.tsx`,
`packages/shared/src/client/settings.ts`. Done: `/sales/{id}/ticket/escpos?lang=ar`
answers raster bytes whatever the preference, `?lang=fr` answers text until
the preference flips, and the spool file name carries the mode. `dz-builder`.
Size S.

**T4: the 80 mm facture down the ESC/POS path**, the archived layouts plan's
T7, re-homed. The raster sidesteps both reasons it was parked (bidi on a
roll, the ISO 8859-15 table). It needs a line model for the facture the way
`ticket.rs` has one, drawn from `facture_view.rs`, with every field §4
requires of a facture. Files: `crates/core/src/print/facture_roll.rs`,
`crates/api/src/routes/sales.rs`, goldens under
`fixtures/print/facture_roll_80mm_escpos/`. Spec: §4 facture field list and
the totals table; the same amounts-parsed-back rule the HTML goldens obey,
applied to the line model before it is drawn. `dz-money-builder`. Size M.

Proof on paper waits for Samir; see the end.

## Phase 4: filler, whenever a slot is free

`architecture-fixes-without-a-domain-split`, T10 and T11. None blocks
anything; each is one branch; none may run alongside a Phase 2 task that
touches the same file.

- T11 page first, straight to `main`: what `users`, `sessions`, `audit` and
  `preferences` share (an audit row names a user, a session belongs to a
  user, a preference is read while one is written) and which piece moves
  below all four. Then T11 the move, one branch, `dz-builder`, done when
  `RINGS_STILL_OPEN` is empty and the constant is deleted along with the
  test's tolerance for it. Size M.
- T10, one file per branch, `dz-builder`, done when the entry leaves
  `scripts/file-sizes.json`: `products.tsx` (929) into `routes/-products/`;
  `customers_.$id.tsx` (897) into `-customers/` beside `fiche.tsx`;
  `documents.tsx` (724); `crates/api/src/lib.rs` (710), whatever is not the
  crate's front door moves out. `sidebar.tsx` (736) is vendored shadcn and
  stays. The Rust services on the list stay pinned, as the plan says.
- The support bundle's leak scan reads the clock as a leak, found
  2026-09-21 on a gates run for the walk-folders branch. `just gates` failed
  on `crates/api/tests/support_bundle.rs` with `document amount ("54.50")
  appears inside log.txt`. `log.txt` holds one line, the build header plus
  `session started <RFC3339 with nanoseconds>`, and the only `.` in it
  between two digit pairs is the seconds-and-fraction boundary. Four seeded
  document amounts are under 60 DA (43.60, 49.05, 54.50, 59.50), so a
  session starting at second 54 with a fraction opening `50` spells one of
  them. Nothing shop-owned can reach that file: `log::head_session` writes
  the header and the timestamp, `read_log` copies it verbatim. Roughly one
  run in 1500, and it costs a full gates cycle each time. One branch,
  `dz-builder`, done when the needle cannot match a clock. Two shapes to
  pick between: refuse a match whose preceding character is a digit or a
  colon, so a real `total 54.50 DA` still bites, or head the session with
  `SecondsFormat::Secs` so the line carries no `.` at all. Excluding
  `log.txt` from the scan is the one answer to refuse, because
  `services/support_bundle.rs`'s own doc names that file as where a leak
  walks out. Size S.
- No walk pins a visibility, found 2026-09-21 by the reach lens on the
  supplier door. `services::supplier_debt::hand_over` writes a supplier
  payment with no upper bound on the amount, and the only thing keeping it
  away from a route is the `pub(crate)` on it; `crates/core/src/lib.rs:13`'s
  `pub(crate) mod repos`, which is the whole layering rule, rests on the same
  one word. Both are held by the compiler today, which is real enforcement
  and catches the call rather than the edit: what nothing catches is the edit
  itself, widening either one in a diff that adds no caller and so goes
  green. One walk that reads both declarations as text would pin the pair.
  Worth doing as a pair or not at all, because pinning the narrower one and
  leaving the layering rule unpinned is the wrong way round. One branch,
  `dz-builder`. Size S.
- The seeder is timed by the wall clock, found 2026-09-21 on a gates run
  for the shifts schema branch. `crates/core/tests/seed_service.rs:306`
  asserts `took.as_secs() < 30`; the run took 30.42s and failed, and the
  same file passed in 68s alone on an idle box minutes later. The bound
  measures how loaded the box is, not how fast the seeder is, so it fails a
  branch that did not touch `seed::run` and costs a full gates cycle each
  time. Its own comment says thirty seconds is generous because the box
  builds for other worktrees, which is the admission that the number is
  about the box. Two shapes to pick between: count what the seeder writes
  (rows, or statements executed) and assert on that, which is what the test
  claims to be about and holds at any load, or keep a wall-clock bound and
  raise it to a number no loaded box reaches, which only moves the flake
  further out. One branch, `dz-builder`. Size S.

## Out of scope, and why each is parked

Every item here rests on Lumina's evidence alone. `docs/features.md` names
no gap for it, and several are decided against already. Parked, not
forgotten; the roadmap page carries the same list.

- Batches as the default (gap 3): Samir, 2026-09-17, no batches this side of
  a market ruling. `docs/features.md` § Later, agreed.
- An off-catalogue line at the till (gap 4): the Sale paragraph of §1 only
  knows a line against a product, and a free-typed line has no TVA rate, no
  category and no stock movement, so it is a fiscal question (what rate does
  it carry under réel) before it is a feature. Needs a ruling nobody asked
  for yet.
- Negative stock as a shop-wide switch (gap 5): nothing in the spec; the
  stock ledger allows a sale below zero today and re-derives daily. A toggle
  is a market question.
- Suspend and draft a sale (gap 6): the till has no such state and the
  numbering rule (a number is taken inside the transaction that writes the
  row) is untouched by a draft, so it is possible; it is not admitted as a
  gap anywhere, so it waits for a shop to ask.
- CCP as a payment mode (gap 7): cheque and transfer are parked by decision
  of 2026-09-08; a postal cheque is a cheque. Unparked with them.
- Zakat (gap 8) and capital injections (gap 9): the summary itself puts them
  below the three, and a zakat figure is a religious-law reading a comptable
  and a scholar settle, not this repo.
- Electronic scale and a cash-drawer kick (gap 10): the kick is one ESC/POS
  command and could ride on Phase 3 T3 in an hour if Samir wants it; it is
  listed here so that happens by decision rather than by drift.
- Three printers and a silent direct print (gap 11, the first half): the
  ESC/POS path already prints without a dialog to a spool or a socket; the A4
  goes through the OS dialog by design. A per-purpose printer choice is
  hardware work for the day the printer exists.
- A wider unit list (gap 12): ruling 9, cheap either way, not built until it
  is answered.

## Guardrails added to the last loop's list

The 2026-09-17 list holds in full. Four more, each from something read
today:

- A pinned file may not grow by one line. `till.tsx`, `dashboard.tsx`,
  `expenses.tsx`, `lib.rs`, `purchases_.$id.tsx` are all on
  `scripts/file-sizes.json`; new till UI goes in `routes/-till/`, new
  dashboard figures in `components/`. `just sizes` is in `just gates`, so a
  builder finds out at the end unless it reads this first.
- A new service obeys `services_go_through_services.rs` from its first
  commit: it reaches its siblings through their services, never their
  repos, and `REACHES_PAST_A_SIBLING` and `RINGS_STILL_OPEN` only shorten.
- A plan task that names a finding is a claim, not an instruction. Three of
  the four architecture statements checked on 2026-09-20 were wrong as
  written. Read the named file before the first edit; the T5 note on the
  architecture plan is the example.
- A plan page exists on `main` before the first `ctx: plan:<slug>/T<id>`
  commit against it, or the hook refuses the commit and the attempt is
  spent.

## Closing

The three lenses on the sharper model, once, over everything both loops
merged since 2026-09-17, since the last loop's Phase 4 never ran. The
session proves or drops each finding itself. Then `dz-standup`, plain words,
no task codes, so Anouar sees a shop's evening close and a ticket in Arabic
rather than a list of ids.

### The review ran 2026-09-21, 19:47 to 20:00, over `168d200..ca584b0`

Three lenses on the sharper model, 117 commits, read only while the mirror
compiled the head. Every finding was settled by the session reading the
line it named. What came back:

- Money: clean across the seams (a partial avoir plus a cancellation cannot
  exceed what came in; the two refund paths cannot both pay; the credit
  bound reads only payments; the shift and the day figure share one query).
  One low finding, left open: a cash refund handed while no shift is open
  is neither tagged nor counted in the outside-a-shift figure, because only
  the sale path calls `tag_if_outside_a_shift` (`services/sales.rs:516`).
  It waits on the design call below, since the answer decides who a refund
  outside a shift belongs to.
- Reach: clean on the table, the walk, the shop scoping and the device
  gate. One design call for Samir: handing cash back is gated on
  `correct_ledger`, a manager's permission, so a cashier at the counter
  cannot refund at all; the row is debited to the caller's own shift
  (`shifts::close` subtracts by the opener against `cash_refunds.user_id`),
  which is the manager's drawer when they hold one and nobody's when they
  do not. Either that is the shop practice, or a cashier refunds under a
  manager override the way the credit limit is overridden. The gate row's
  reason said "the ringer's shift" and now says the refunder's (`f7d0c3d`).
  Also written down, not ruled: the print-language and thermal-mode flips
  are unaudited on purpose (`services/preferences.rs:173`).
- Tests: four edges a green suite was hiding, all pinned in `f7d0c3d`
  (PR 154): the refund window's two edges, the floor of the outside-a-shift
  count and its midnight fallback, the roll's closing figures as hand
  constants, and the refusal row for a cashier reaching for a colleague's
  drawer, which the table could not record and the route now does. Not
  done: a browser case pressing Enter inside a quantity box (the focus
  claim rests on jsdom at `till.test.tsx:1374`); one Playwright case, next
  time the suite runs for a screen change.

The day's `just ci` on `main`: the full run on `ca584b0` green on all four
jobs (rust, windows, coverage, web); the Restricted job failed a shell test
on a pipe race in the test script itself, fixed in `81c6f1e`; one more run
on `f7d0c3d` closes the day. The loop is closed; what it leaves for Samir is
the refund question above, the modular decision plan, the amount in words,
and the scanner's last row.

## Filler found while building, not acted on

**The audit log cannot be searched by the person whose drawer was counted.**
`repos::audit::filtered` filters on the row's own `user_id`, and a close
writes that row under the closer. When a manager counts a cashier's drawer
the opener's id lives only inside the `after` JSON, which no filter reads. An
owner asking "what happened to Amina's till" by picking Amina in the audit
screen does not see it. Found 2026-09-21 while reviewing the till routes,
confirmed by reading the repo. Two shapes: a second indexed column for the
person a row is about, or the audit search learning to read one key out of
`after`.

## What waits for Samir, none of it a task here

- The shared lockout, across the PIN and the password. The nine rulings this
  file opened are not here: all nine were taken on 2026-09-20 and are written
  above, which "Rulings still open" already says.
- A real printer for Phase 3: the raster is golden-filed as PNG here, and
  whether a head prints it is proven the day M1 T6 is, on the laptop with a
  head on the desk.
- The phone in his hand: Arabic layout, the `دج` suffix beside Latin digits,
  and the Maestro flows rewritten to test ids while he watches them run.
- The scanner plan's last task, unchanged from the last loop.
