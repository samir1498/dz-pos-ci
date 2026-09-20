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
Rulings 1 to 3 are settled; 4 to 9 below still wait on him.

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
that was open when it was rung, not the one open when it reaches the server.
Nothing new is needed to know which: `apps/mobile/lib/queue.ts` stamps
`createdAt` on every queued request and `NewSale` already carries an optional
`issued_at`.

If that shift has since closed and been counted, the sale is tagged as
belonging to no shift, under ruling 2, rather than reopening a counted shift
or landing in somebody else's. Its cash went into a drawer whose count is
already signed, and moving the sale would make that signature false.

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

## Rulings still open

None. Every question this file opened was answered on 2026-09-20 and written
above. Rulings 8 and 9 were taken without the hardware and the competitor
listing that would have settled them outright, and each says so in its own
words rather than reading as a fact.

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
  #121, #124, #125). Four stay open and are the unfinished work this loop
  starts with:
  - T4: `REACHES_PAST_A_SIBLING` in
    `crates/core/tests/services_go_through_services.rs` pins thirteen
    reaches across nine services (down from seventeen across eleven).
  - T9: the two JSX money sums are still there,
    `apps/desktop/src/routes/purchases.tsx:260` and
    `apps/desktop/src/routes/purchases_.$id.tsx:232`, each adding
    `transport_centimes + extra_costs_centimes` in the component.
  - T10: `scripts/file-sizes.json` holds 27 entries (from 32);
    `suppliers.tsx` left the list with #119. `till.tsx` (905),
    `dashboard.tsx` (659), `expenses.tsx` (668), `crates/api/src/lib.rs`
    (710) and `packages/shared/src/client.ts` (994) are pinned and may not
    grow, which shapes every task below that touches them.
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

## Phase 0: room to add, alone, before anything else

Two files sit on the line and every task below adds to them. The same reason
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

## Phase 1: the print language, and the carried-in architecture work

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
Done: no `+` between two `_centimes` fields in any `.tsx`, a lint or test
that refuses one, and `just ci <branch> full` green on the mirror.
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
- T4 branch c, the debt service and the product cost: `debt -> documents`
  is the sharpest row on the list, because `debt.rs:581` writes
  `documents_repo::set_remaining_debt`, and `remaining_debt` is a field of
  the §3 totals table ("from the ledger at issue time", the balance triple
  paragraph under it). The reads beside it (`kinds_and_numbers`, `get`,
  `belongs_to_shop`) move in the same branch or the row stays. With it
  `debt -> customers` (`belongs_to_shop`), and `import -> products` plus
  `purchases -> products` (`by_barcode`, `get`, `set_cost`, where
  `set_cost` writes a landed cost the purchase computed). Spec: the balance
  triple paragraph of §3 for `set_remaining_debt`, the Cost of goods sold
  row for `set_cost`. Four rows leave. `dz-money-builder`, and `just ci
  <branch> full` on the mirror. Size M.
- T4 branch d, the mutual pair: `purchases -> supplier_debt` and
  `supplier_debt -> purchases, suppliers`. Routing each through the other's
  service makes a ring the ring test refuses, so this is the T3 shape: the
  piece both need (the purchase's landed total and its supplier) moves to a
  module below both, the way `services::pricing` went under `sales` and
  `proforma`. Three rows leave and `RINGS_STILL_OPEN` does not grow.
  `dz-money-builder`, a supplier balance is on the path. Size M.

Three, three, four and three: the thirteen rows, and the constant is
deleted with the last one.

Before Phase 2 starts: the advisor, with the per-PR Sonnet lenses having
run on each Phase 1 merge as always, because Phase 2 builds on `cash.rs`
and `expenses.rs`. The sharper model runs once per loop, at the close, as
the 2026-09-17 ruling says; not here.

## Phase 2: till shifts

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

There is no `cash_movements` table and no `drawers` table. Money out of the
drawer mid-shift is an expense, which `expenses` already holds; cash handed to
the owner is the note at close. Both were ruled today. If a shop later asks
for the individual movements, that is a table added then, not now.

`expected_at_close_centimes` is the one stored figure and it is stored on
purpose. The live position stays derived on every read (rule 4, and the cash
paragraph's "stored nowhere"). But a shift's difference is a fact about one
evening: a ticket annulled on Wednesday leaves Monday's takings, so a derived
expected figure would move the Monday count after it was signed. The snapshot
is what the closer saw; the difference is `counted - expected_at_close`, both
stored, so it is checkable and does not drift.

### The window, and the hour it is off by

The shift window is read on the shop clock against `created_at` of documents,
`debt_ledger` and `supplier_ledger`, all of which the services stamp.
`expenses.created_at` is not stamped: there is no clock call in
`services/expenses.rs` or `repos/expenses.rs`, so it takes the
`DEFAULT (CURRENT_TIMESTAMP)` written in
`crates/core/migrations/2026-09-10-000008_suppliers_purchases_expenses/up.sql`,
which SQLite answers in UTC. Algiers is an hour ahead, so an expense written
after 23:00 falls out of the shift that paid it.

The fix ships in this same migration, in the 000013 shape: stamp on write, and
shift the rows written so far by the hour they are short. This carries over
from the earlier draft of this phase and must not be dropped with the
`cash_movements` table it was attached to.

### What it does to the services

A new `services::shifts` reads the position through `services::cash::position`
and never through `repos::cash` or `repos::expenses`, or
`REACHES_PAST_A_SIBLING` grows on its first commit (and T4 branch b will have
just shortened it). `cash::position` gains a `Period::Between(from, to)` for a
shift window.

Expected at close is `opening_cash + cash_in.total() - cash_out.total()` over
the window, every step `checked_*`. `Takings` and `Outgoings` keep exactly the
fields they have; nothing is added to either, because no movement kind exists
to add.

The dashboard does not read shifts at all. `cash::position` over a day or a
month answers what it answers today, from sales, the two ledgers and expenses.
A shift is a second, narrower read of the same function, not a new input to it.
This is what keeps a handover from moving the shop's cash figure.

A sale rung while the ringer has no open shift is accepted and tagged, never
refused (ruling 2). The tag is derived, not stored: a sale belongs to no shift
when its `created_at` falls outside every `[opened_at, closed_at)` of that
`user_id`. The close screen reads that as its own figure beside the expected
one, so a morning rung before the shift opened does not read as a shortage. A
replayed offline sale lands the same way, on `issued_at` rather than arrival
(ruling 3).

Three audit actions: `till.open` (opening cash), `till.close` (expected,
counted, difference, note, and the opener's id when the closer differs), and
`till.sale_outside_shift` on a tagged sale. The features §5 audit list gains
the three.

The shift list is its own screen under `see_reports`, not a view of the audit
log, because `see_audit_log` is the owner's alone (`docs/features.md` line 891)
and a manager who cannot read a day's shifts cannot run a floor.

Permissions: opening and closing is a new permission, `open_and_close_till`,

because a cashier may `sell` and nothing else today and ruling 5 decides
where it sits. Adding it moves `permissions.rs` off thirteen and the match
refuses to compile until all three roles place it, which is the point. §5's
"Thirteen permissions" and `gates.rs`'s count are rewritten in the same PR;
`.claude/stale-homes.md` gains the row it does not have for the permission
count.

### The awkward cases, each with what the schema does about it

- A shift across midnight: a session is a shift, not a day. The dashboard's
  day and month figures keep their calendar definition; the session report
  reads the session window. Both are true at once and the page says which
  screen answers which question.
- Closed by a different user: `closed_by` is its own column and the audit row
  names both. Manager and owner only, from the permission table: a cashier
  may `sell` and nothing else, and closing another person's till is a
  correction.
- A sale with no shift open: accepted and tagged, never refused (ruling 2).
  Nothing is stored on the sale: a sale belongs to no shift when its
  `created_at` falls outside every `[opened_at, closed_at)` of its `user_id`,
  and `shifts::sales_outside_a_shift` is that one query. It writes an audit
  row `till.sale_outside_shift`.

  This is the answer that keeps the fixtures as they are. Seven desktop e2e
  specs (`till`, `till-credit`, `till-facture`, `till-cashier`,
  `till-reversals-and-quotations`, `settlement`, `scanner`), `just seed`, the
  demo recordings and the Maestro flows all ring sales with no shift open.
  Refusing would have meant opening a shift in every one of them.
- The phone's queue replaying after close: the sale belongs to the shift that
  was open when it was rung, read off `issued_at` and not off arrival
  (ruling 3). `apps/mobile/lib/queue.ts` stamps `createdAt` on every queued
  request and `NewSale` carries `issued_at`, so nothing new is needed to know
  which. If that shift is closed and counted, the sale is tagged rather than
  reopening a signed count. The replay goes through `issue_idempotent` and
  must not be dropped in silence, which T8 of the architecture plan found the
  phone doing once already.
- Two tills in one shop: rule 5 of `docs/architecture.md` says exactly one
  desktop is the server, and the phone is a thin client that rings into it,
  so v1 has one drawer per shop and the partial unique index says so. A
  second drawer is a second `till_id` column and a wider index later, not a
  redesign; the page names that so nobody builds it early.
- Cash handed back: ruling 6. If a paid-out covers it, nothing else moves.
  If a refund is a ledger payment going out, `cash.rs`'s `refunds` stops
  being zero and the cash paragraph's "nothing records cash handed back"
  goes.

### The tasks

**T1: the page.** The design above with rulings 5 and 6 filled in, the
schema, the awkward cases as decided. Rulings 1 to 4 are taken and written at
the top of this file. Bookkeeping, straight to `main`.

**T2: the migration, the models, the repos.** Files:
`crates/core/migrations/2026-09-2x-000017_shifts/{up,down}.sql`,
`crates/core/src/schema.rs`, `crates/core/src/models/shift.rs`,
`crates/core/src/repos/shifts.rs`,
`crates/core/src/services/expenses.rs` (stamping),
`crates/core/tests/migration.rs`. Migration. Done: the revert loop in
`migration.rs` passes, a second open shift for the same user is refused by
the file while one for a different user is accepted, a close whose counted
figure differs from the expected one with no note is refused by the file, and
an expense written at 00:30 on the shop clock is stored at 00:30. Size M.

**T3: the service and the audit rows.** `services::shifts::{open, close,
open_for, sales_outside_a_shift, report}`, the `cash::position` window, and
the three audit actions. Files: `crates/core/src/services/shifts.rs`,
`crates/core/src/services/cash.rs`, `crates/core/src/services/audit.rs`,
`crates/core/src/services/mod.rs`, `crates/core/tests/shifts_service.rs`.
Spec: the cash position paragraph of §1, rewritten in this PR to say what a
session adds; the fiscal rules table gains no row because no document
changes what it charges. Done: a fixture with a float, two cash sales, one
card sale, one customer payment, one expense, one paid-out and one
supplier payment in cash, whose expected figure is written by hand in the
test and not computed by the code under test; the difference is negative
when counted is short; `REACHES_PAST_A_SIBLING` unchanged. Size M.

**T4: the routes, the gates, the permission.** `POST /till/sessions`,
`POST /till/sessions/{id}/close`, `POST /till/sessions/{id}/movements`,
`GET /till/sessions/open`, `GET /till/sessions/{id}`; the gate rows; the
permission; the sale hook for ruling 2. Files: `crates/api/src/routes/till.rs`,
`crates/api/src/dto/till.rs`, `crates/api/src/gates/table.rs`,
`crates/api/src/router.rs`, `crates/core/src/services/permissions.rs`,
`crates/core/src/services/sales.rs`, `crates/api/tests/till_api.rs`,
`docs/features.md` §5. Done: `route_gates.rs` green with the new rows; a
cashier's refusal names the permission; the sale-without-session case
answers the code ruling 2 chose. Size M.

**T5: opening and closing at the till.** The banner (no session, or float
and running expected figure if ruling 5 shows it), the open modal, the close
modal with counted and difference. `till.tsx` is pinned at 905 and may not
grow, so every line goes in `apps/desktop/src/routes/-till/session.tsx`,
wired from `till.tsx` in one import and one element. Files: the part file,
`till.tsx` (a few lines), `packages/shared/src/client/till.ts`, the three
dictionaries. Done: `till.test.tsx` (pinned at 1405, so new tests go in
`-till/session.test.tsx`) proves open, close, short count shown negative,
and the refusal wording; the app under `just e2e` opens a session, sells,
closes, and the difference printed matches the fixture. Size M.

**T6: movements, and the figure on the dashboard and the expenses screen.**
Paid-in and paid-out from the till part, the dashboard's cash panel showing
the float and expected-in-drawer when a session is open (in a component
file, `dashboard.tsx` is pinned), the same beside the month's cash on
`expenses.tsx` (pinned). Files: `apps/desktop/src/routes/-till/movements.tsx`,
`apps/desktop/src/components/CashPanel.tsx` (lifted out of `expenses.tsx`,
which shortens it and lowers its entry), `dashboard.tsx` (imports only).
Done: a paid-out lowers the expected figure on all three screens without a
reload; a cashier without the permission sees the button refused by the
server, not hidden. Size S.

**T7: the closing sweep.** The session report readable after close (a list
under `/till/sessions`), the e2e with a real cashier per the M4 rule, the
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
  `expenses.tsx`, `lib.rs`, `client.ts`, `purchases_.$id.tsx` are all on
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

## What waits for Samir, none of it a task here

- The nine rulings above, and the shared lockout one.
- A real printer for Phase 3: the raster is golden-filed as PNG here, and
  whether a head prints it is proven the day M1 T6 is, on the laptop with a
  head on the desk.
- The phone in his hand: Arabic layout, the `دج` suffix beside Latin digits,
  and the Maestro flows rewritten to test ids while he watches them run.
- The scanner plan's last task, unchanged from the last loop.
