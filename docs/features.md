# Features: what dz-pos does

Normative. If a screen or a rule is not here, it is not in scope. The
competitor teardown, the Algerian facture field list and the market notes
live in `research/` (competitor material under
`research/competitors/2026-09-07-lumina-and-market/`); this file says
what *we* build, and links there for *why*.

Status legend: **v1** ships in the first release; **later** is agreed but
not scheduled; **open** waits on a decision listed at the bottom.

## Scope

- One shop, one SQLite file, desktop first. The phone is a thin client of
  the desktop (or of a hosted server later). Every table carries `shop_id`
  from day one even though v1 only ever has one value.
- Languages: Arabic (RTL), French, English on every screen and every
  printed document, independently selectable (UI language ≠ print language).
- Out of v1: cloud mode, phone-only offline, product variants/colours,
  bundles, promotions, AI invoice scanning, e-invoicing. Each is either
  parked in the architecture notes or listed under open decisions.

## Build order

Each step is usable on its own and ships with its tests before the next
starts. `docs/roadmap.md` holds the milestones, what each one demos and
what blocks it; this list is the order in one glance.

1. Money core: centimes, TVA, stamp, amount in words, the first migration,
   `crates/api` with products.
2. A cash sale with a printed 80mm ticket: products, till, the ticket
   series, stock ledger, the user on every row and the audit log, settings
   with the régime fiscal, three languages, daily backup.
3. Customers with a debt ledger and credit limit, and the invoice model
   with every legal field: facture, avoir, proforma, statement.
4. Stock in: suppliers, purchases, expenses, dashboard, Excel.
5. Users and roles.
6. First release: installer, updater, signing, versioned migrations.
7. LAN mode: one desktop serves, phones and second tills pair by QR.
8. Cloud mode, only after Anouar decides (open decision 1).

The sections below keep their original numbering; it names the area, not
the order.

## 1. Inventory baseline (v1)

**Product.** Name, barcode (unique, optional, auto-generated numeric if
blank), category, unit of measure (piece, kg, litre, box), cost price,
selling price, wholesale price (optional), quantity on hand, low-stock
threshold, TVA rate (see open decision 3), active flag. Batches/lots: later.

**Supplier.** Name (unique inside the shop), phone, address, RC, NIF, NIS, AI,
notes, and an `active` flag the way a product has one. The opening debt is not
a column: it is the first `opening` row of the supplier ledger, so the balance
has one source and correcting it later is an `adjustment` movement a comptable
can read. There is no credit limit and no warning threshold, and no
`party_kind`: those are what a shop grants a buyer, and nothing it hands a
supplier is a document it issues. A second fiche under one name is refused on
the name, because two fiches would read at a counter as one party carrying two
balances.

**Supplier debt.** Append-only per supplier, the mirror of the customer ledger
(§2): a movement is one of `opening`, `purchase`, `payment`, `return` or
`adjustment`, it raises what the shop owes or lowers it, never both and never
neither, and the balance is the sum of the table rather than a stored number.
It may go below zero, which is an advance sitting with the supplier and is
named as one rather than shown as a minus. A movement carries how it was paid
exactly when it is a payment: `cash` or `card`, and null on every other kind.

A payment settles the supplier's open orders oldest first and stops where the
money does; what no order can take stays on the balance, which is what a
payment against an opening balance is. What is still owed on one order is
derived from the ledger and the allocations rather than stored, so nothing has
to be written back to a purchase and no column can disagree with the sum. A
payment above what is owed is refused with the outstanding figure, the way the
customer side's is. A correction is an `adjustment` movement, optionally
noted; downwards it settles the orders oldest first, upwards it is debt no
order carries.

The suppliers screen offers no delete (the ledger and the orders hold the
fiche). Closing a fiche that still carries something (a balance either way, or
an order still asking to be paid) needs a reason, and the reason goes into the
audit log beside the balance and the number of orders left open. The fiche
stays usable after it: payments and corrections still land on a closed fiche,
and a purchase is what it refuses (T3). One supplier has an address of its
own, `/suppliers/$id`, and the route and the list's expanded row render one
and the same component, so the two cannot drift. The account over a range of
days is answered as JSON (`GET /suppliers/{id}/statement?from=&to=`): a
printed statement is a paper a customer is handed, and the shop's own copy of
what it owes is a screen.

**Purchase.** Supplier, supplier's document number, date, lines (product,
quantity, unit cost), transport and extra costs, amount paid now, due date
for the rest. Saving a purchase moves stock in and adds the unpaid part to
the supplier's debt. A purchase can be received in parts.

**Sale (till).** Lines (product, quantity, unit price, line discount),
global discount, payment mode (cash; credit on the customer's ledger; card
on a TPE with no integration, decided 2026-09-08), amount tendered,
change. Anonymous sale is allowed; a credit sale requires a customer.
Saving a sale moves stock out and, if credit, adds to the customer's debt.
Every sale is a fiscal document (see §3) even when it is a simple ticket.
The till screen adds one rule the spec does not name: a product whose unit
is piece or box is sold in whole units, so a typed "1,5" is refused on the
line before the sale is posted (kg and litre take it). The core does not
enforce this; a half box is not something a receipt can say, and nothing
else was catching it.

**Credit at the till.** A credit sale names a customer; without one it is
refused on the `customer_id` field, because the ledger is per customer and
money owed by nobody has nowhere to sit. The customer has to be active and
of the shop. A closed fiche is named on no document at all, cash and card
included: closing it says the shop has stopped trading with that customer,
and a paid ticket made out to them would say otherwise. What the credit
limit is tested against is the balance the sale would leave behind,
`old_balance + net_to_pay`, not the
basket: a customer 100,00 under their limit cannot buy 200,00 on credit
however small each basket is. A null credit limit is no limit at all and a
zero one is no credit at all, which are two different answers. Passing the
limit refuses the sale with the code `credit_limit`, which carries the
balance after and the limit so the till can say by how much; the refusal
happens before the document number is taken, so a blocked sale burns none.
A null warn threshold is no warning; reaching one is enough to warn, and a
warned sale still goes through, so the answer carries `near_limit` beside
the document rather than refusing it.

The owner can pass the block by sending the sale again with `override`.
That sale is written like any other and an audit row records it
(`document.issue_override`, with the balance, the limit and the document it
produced). There are no roles in the app until §5 lands, so the API accepts
an override from anyone; the log is what carries the accountability in the
meantime.

A cash or a card sale may also name a customer. The document then carries
the buyer block and the balance triple with a `remaining_debt` of nothing,
and no ledger movement is written: nothing is owed. The buyer block is
snapshotted onto every document that names a customer, a ticket included,
so the facture switch has nothing to backfill.

**Stock movements.** Append-only ledger: every change to quantity on hand
is a row with type (purchase, sale, adjustment, return, opening), quantity,
unit cost, reference to the source document, user, timestamp. Quantity on
hand is derived from this ledger and cached on the product.

A recount re-derives that cache and reports what it found. It runs once per
shop day from the daily job, after the backup, so a correction the owner
disagrees with is inside that day's copy; the settings screen asks for one
at any time. The ledger is the truth, so a cache the movements do not
explain is written back to the ledger's sum rather than left for someone to
fix, and each correction is one audit row `stock.drift` naming the product,
the cached quantity, the ledger quantity and the difference. Those rows are
the whole record of a recount: there is no table of runs, and the panel
reads the drifts of the last run back out of the log by the day the run was
marked under. A run that finds nothing still marks the day and writes no
row.

**Expense.** Category (seeded: rent, electricity, water, salaries,
transport, maintenance, other), amount, date, note. The seven categories
are seeded per shop and a shop adds none of its own in v1; the row carries
the category's key and the desktop holds its label in the three languages,
so a shop switching language rewrites no row. The amount is above zero and
the day is a day on the shop's calendar. An expense is written once: it is
never edited and never deleted, and the table carries no cancellation
block, so a wrong one is a row a comptable reads and asks about. The
screen lists one month at a time with the month's total.

**Dashboard.** Today's and this month's sales, gross margin (sales minus
cost of goods sold from the ledger), expenses, cash position, low-stock
list, top products, outstanding customer debt, outstanding supplier debt.

The cash position is derived on every read and stored nowhere. It is what
moved into and out of the drawer. Over a day or a month on the shop's
calendar, cash in is what the tickets and factures still standing and paid
in cash came to, at `net_to_pay`, which is what the customer handed over
and so includes the droit de timbre, plus the cash payments on
`debt_ledger`; cash out is the cash payments on `supplier_ledger` plus the
expenses of those days. The stamp inside the sales figure is answered again
on its own, so a screen that wants the shop's own takings can subtract the
tax it is collecting for the state; it is a part of the sales figure and
never a second one to add. A cancelled document is
out of the sales figure rather than subtracted from it, and a refund counts
nothing: an avoir credits the customer's ledger and brings the goods back
on `return` movements, and nothing in the file says the drawer opened for
it. The card figure has the same shape on the way in and none on the way
out, because money paid to a supplier by card moves the bank account rather
than the till. The figure is a net movement over the period and not the
money in the drawer: there is no opening float and no count at close, so it
goes below zero on a day that paid out more than it took.

Two things the figure cannot yet say. Nothing records cash handed back over
the counter, so the position is off by any refund a shop actually paid out.
And a cancelled sale leaves the day it was sold on and appears on no other:
a ticket rung up on Monday and annulled on Wednesday is out of Monday's
takings, which is right for Monday and wrong for the drawer on Wednesday.
The fiscal rules table gains no row for any of this, because nothing here
changes what a document charges.

**Backup.** Automatic daily copy of the SQLite file, keep 30, restore from
the settings screen. Export products, sales, customers, suppliers to Excel;
import products from Excel with a downloadable template.

## 2. Customers and debt (v1)

**Customer.** Name, phone, address, RC, NIF, NIS, AI (all optional for a
private person, required for a company the moment an invoice is issued to
them), credit limit, warning threshold, opening debt, notes.

The stored fiche differs from that list in four places (migration
`2026-09-09-000002_customers`). It carries a `party_kind`, `company` or
`consumer`, and an `active` flag the way a product does. The name is
required, because the buyer block prints it and the list is ordered by it.
The opening debt is not a column: it is the first `opening` row of the debt
ledger, so the balance has one source and correcting it later is an
`adjustment` movement a comptable can read. A null credit limit is no limit
and a zero one is no credit at all; a null warning threshold is no warning.

`party_kind` is a field on the fiche, never inferred from whether an RC was
typed in. Loi 04-02 art. 10 decides ticket against facture by who the buyer
is, and the facture's party-identifier check asks a different set of fields
of a company than of a consumer, so an inference would flip the rule the
moment somebody clears a field.

The customers screen opens a company fiche with neither RC nor NIF: the
identifiers are asked for when a facture is issued to it, not to open its
record. It offers no delete (the ledger holds the fiche); a shop that has
stopped dealing with somebody clears `active`. Closing a fiche that still
carries something (a balance either way, or a document still asking to be
paid) needs a reason, and the reason goes into the audit log beside the
balance and the number of papers left open, because a shop that stops
trading with a customer who owes it money has taken a decision. The fiche
stays usable after it: payments, corrections, avoirs and cancellations all
still land on a closed fiche, and only a new sale or a proforma is
refused. The list shows one tag per
customer, and when two apply the order is over the limit, then no credit at
all (a zero limit), then the warning threshold. A negative balance is named
rather than shown as a minus: the list tags the row and the fiche labels the
figure `crédit`, and the amount is always drawn positive. "Créance -1 000,00"
is not a sentence said at a counter, and the sign alone is easy to miss on a
line of figures.

**The fiche.** One customer has an address of their own, `/customers/$id`, so
a screen that has a customer id can hand the operator the fiche instead of a
list to search. The till's credit refusal links to it. The route and the
list's expanded row render one and the same component, so the two cannot
drift.

The fiche holds the identity and the credit fields, the ledger newest first,
the payments newest first with what each one settled, and three things the
operator can do: take a payment, correct the balance, and print. It offers
the statement over a range and the debt slip, each rendered by the core and
shown in a sandboxed frame, which is the same HTML the printer is handed.

**Debt ledger.** Append-only per party (customer or supplier): document
reference, amount owed added, amount paid, running balance.

A movement is one of `opening`, `sale`, `payment`, `avoir` or `adjustment`.
It raises the debt or lowers it, never both and never neither, and the
balance is the sum of the table rather than a stored number. It may go below
zero: a customer who overpays, or one an avoir credits past what they owe, is
owed money, and the screens name that a credit. A movement carries how it was
paid exactly when it is a payment: `payment_mode` is `cash` or `card` on every
payment and null on every other kind, and the table refuses it either way
round (`2026-09-09-000006`). A made-up `cash` on an opening balance or on a
sale would read as money that moved, and a payment saying nothing would be
money whose road through the drawer nobody can retrace. Settling a document
out of credit the customer already holds writes no payment at all: it is an
allocation against the credit rows the ledger is already carrying. The
ledger reads newest first and the row id breaks a tie inside one second, so
two movements written in the same second read back in the order they were
written.

**Payments.** Money handed over against the account, in cash or on a card,
optionally noted. A payment of nothing is refused, and so is a payment above
what is owed: the balance is read inside the payment's own transaction and
the refusal carries what is still outstanding, so the form can say how much
can be taken without asking the balance a second time. It is not a document
and takes no number; whether a payment on account needs a numbered receipt of
its own is R8, and the `quittance` kind waits on that answer.

A payment settles the customer's issued documents oldest first, oldest by
the day the paper was issued and not by the row id, and stops where the money
does. A cancelled document takes none of it: whatever its `remaining_debt`
column still says, money handed over settles the paper that still stands.
What no document can take stays on the balance as credit.

**Corrections.** A correction is an `adjustment` movement, optionally noted: a
mistyped opening balance, a goodwill gesture, a rounding a comptable wants
off the account. A correction of nothing is refused. Downwards it settles the
customer's documents oldest first exactly as a payment does, because it is
money off the papers; upwards it is debt no document carries, so there is
nothing to place.

**The till.** The till warns at the threshold and blocks at the limit
(§1, credit at the till); blocking is overridable, and the audit log carries
who overrode it until roles land (§5).

**One clock.** The core stamps every movement and every document on the
shop's calendar: Algeria is UTC+1 all year with no daylight saving, so a sale
rung up at half past midnight is already on the new day
(`crates/core/src/services/clock.rs`). A service that writes a dated row
takes the moment as an argument wherever a test has to choose it, so the
wall clock stays out of the fixtures. There is one clock and the server
holds it: `GET /clock` answers `{ "today": "YYYY-MM-DD" }` on that same
calendar, and a screen that needs a day asks for it rather than reading the
machine's. A browser reads the zone the machine is set to, which on a
laptop carried across a border, or simply set wrong, is another day
entirely: a statement asked for "to today" would then close before that
evening's movements, and a régime change would be dated to a day the ledger
has not reached. The answer is never cached, because a day is the one thing
that changes without anyone changing it.

**Statement.** A per-party statement over a date range, rendered by the core
on A4 in the three languages: the opening balance the customer carried into
the range, every movement inside it with its running balance, and the closing
balance in figures and in words. Its range is inclusive at both ends. A range
with no movement still prints both balances and says the range was empty. A
closing balance the shop owes prints the amount positive and says whose way it
goes. A company's identifiers are printed and a consumer's are not (décret
05-468 art. 3-2). `GET /customers/{id}/statement?from=&to=&lang=`.

**Debt slip.** What a customer asks for at the counter: the shop's header,
the customer's name and, for a company, its identifiers; the balance in
figures and in words; and the last ten movements, newest first, each with its
day, its kind, the number of the document it names, its debit, its credit and
the running balance. It is 80 mm paper on the ticket's mechanics, and it
carries no TVA recap and no droit de timbre because it is not a sale. A line
in each language says on its face that it has no fiscal value. The balance is
the whole ledger's, never the sum of the ten rows shown, so a slip handed to a
customer with a long account is right about what they owe. A customer who owes
nothing gets a slip that says so.
`GET /customers/{id}/debt-slip?lang=`.

## 3. Invoice model (v1)

One document type with a `kind`: `ticket` (till receipt), `facture`,
`proforma`, `bon_de_livraison`, `avoir` (credit note), `bon_de_reception`.
All kinds share the same lines and totals; only numbering, legal blocks and
stock effect differ. The column admits a seventh value, `quittance`, the
stamped receipt for a payment on account; nothing issues one until the
comptable says whether such a payment needs its own numbered document (R8),
and the value is admitted now because adding a kind once there are documents
means rebuilding the table.

**Seller block** (from store settings): name, RC, NIF, NIS, AI, address,
phone, fax, email.

**Buyer block** (from the customer at issue time, snapshotted): name, RC,
NIF, NIS, AI, address.

**Lines:** product snapshot (name, barcode), quantity, unit price HT, line
discount, TVA rate, line total HT.

**Totals**, all in centimes:

| Field | Definition |
|---|---|
| `total_ht` | Σ line totals HT after line discounts |
| `discount` | global discount applied to `total_ht` |
| `subtotal_ht` | `total_ht − discount` |
| `tva` | Σ per rate: `round(subtotal_ht_at_rate × rate)` |
| `total_ttc` | `subtotal_ht + tva` |
| `stamp` | droit de timbre, see below |
| `net_to_pay` | `total_ttc + stamp` |
| `amount_in_words` | `net_to_pay` written out in the print language |
| `old_balance`, `remaining_debt`, `total_debt` | from the ledger at issue time |
| `payment_mode` | cash, card, credit in v1; cheque and transfer are parked (the column admits them) |

The balance triple, in the words that decide it. `old_balance` is what the
customer owed before this document. `total_debt` is what they owe after it,
which is `old_balance` plus the whole of what this document put on the
account. `remaining_debt` is this document's own unpaid part at issue:
`net_to_pay` on a credit sale, less any credit the customer was already
holding that settled it there and then; zero on a cash or a card one; and on
an avoir the negative of its own `net_to_pay`, which is its whole effect on
the account. All three are read from the ledger when the document is issued
and stored on it, so a reprint six months later prints the balance the
customer was handed and never a sum of today's ledger. A facture's
`remaining_debt` comes down as the paper is settled; an avoir's never does,
because the paper states what that credit note was worth on the day it was
written. A document that names no customer stores none of the three.

`remaining_debt` is the one of the three that moves after the document is
issued, and it moves for both of the things that lower what a customer owes:
a payment, and a correction downwards. Both settle the customer's documents
oldest first, so Σ `remaining_debt` over a customer's issued documents is
never more than the ledger balance, and a document never goes on asking for
an amount the ledger says is no longer owed.

Because the limit is tested on `total_debt` and not on the basket, a
customer whose balance is negative may buy on credit up to what the shop
already holds for them, whatever the limit says. A deposit or an avoir
leaves `old_balance` below zero; a 200,00 basket against a 300,00 credit
balance leaves `total_debt` at -100,00, which is past no limit, not even a
limit of zero. That is the same rule read from the other side, not an
exception to it: what the block asks is what the customer will owe.

A negative balance is named on the screens rather than shown as a minus:
the customers list tags the row and the fiche labels the figure `crédit`,
and the amount is always drawn positive. "Créance -1 000,00" is not a
sentence said at a counter, and the sign alone is easy to miss on a line of
figures.

**Facture or ticket at the till.** Loi 04-02 du 23 juin 2004 art. 10, as
rewritten whole by loi 10-06 du 15 août 2010 art. 3, decides it, and it
decides it by who the buyer is, not by an amount. There is no threshold in
the text.

- A sale to a consumer is a `ticket`. « Les ventes de biens ou les
  prestations de services faites au consommateur doivent faire l'objet d'un
  ticket de caisse ou d'un bon justifiant la transaction. » That is the
  till's default and it needs nothing from the customer.
- A sale to an agent économique carrying on an activity listed in art. 2
  (production, distribution, services, artisanat, pêche, agriculture) is a
  `facture`, and the buyer is under a matching duty to ask for one.
- Any buyer who asks turns the sale into a `facture`: « Toutefois, la
  facture ou le document en tenant lieu doit être délivré si le client en
  fait la demande » (art. 10 al. 3; décret 05-468 art. 2 repeats it from the
  facture side).
- The document is due « dès la réalisation de la vente », so the choice is
  made at the till, before the sale is saved, and never by a later reprint.

A till cannot tell a consumer from a professional on its own, so the
operator decides: the sale screen carries a one-tap switch from ticket to
facture that pulls in the buyer block. A facture to a consumer needs only
« ses nom, prénom(s) et adresse » (décret 05-468 art. 3-2, last alinéa); a
facture to a trader needs the party identifiers of the row below. The till
sends the choice as `kind` on the sale (`ticket` or `facture`, ticket by
default), and a facture is refused unless both halves carry what the row
below asks: the shop's own RC and NIS from its settings, and from the fiche
either the buyer's RC and NIS or, for a consumer, a name and an address. The
refusal names the side and the identifiers that are missing, and it happens
before a number is taken, so neither series gaps over it. A facture whose
`net_to_pay` is zero (a basket a discount emptied, an exchange settled line
for line) is issued today like any other and takes its number, because no
text read so far says a document has to be worth something; confirm with the
comptable (R8) before a shop leans on it.

Numbering: per kind, gapless, assigned at issue and never reused; a
cancelled facture keeps its number and is marked "facture annulée"; an
avoir is its own kind with its own series. The series restart at 1 each
year and the number carries the year, the common practice in Algeria and
not a rule of the decree (decided by Samir on 2026-09-10, built in M3 as
its first task); the comptable (R8) confirms the practice, not the choice.

**Avoir.** A facture is never edited and never deleted, so a shop that has
to carry money back writes an avoir: a second numbered document out of the
avoir series, naming the facture it credits. It may be partial. Each of its
lines names one line of the facture and carries a quantity no greater than
what is left on that line once every earlier avoir is counted, and the
running total of avoirs on one facture never passes that facture's
`total_ttc`. Its TVA is per rate on its own lines, computed the way a
sale's is. It never carries the droit de timbre: the stamp is paid on money
that changed hands and is not refunded with the goods, so a whole avoir of a
stamped cash facture comes to that facture's `total_ttc` and not to its
`net_to_pay`. The goods go back on `return` stock movements naming the
avoir.

On the ledger an avoir reverses the unpaid part of the facture it was
written against first, whatever that facture's age; what that facture cannot
take spreads over the customer's other unpaid documents oldest first, the
way a correction downwards does; and only what no paper can take at all
becomes credit the shop is holding. The spreading step is what keeps one
account to one figure: a customer told they hold 1 000,00 and owe 2 000,00 on
an open facture at the same time is being handed two numbers to net out by
hand. It is also the only way a balance goes below zero. A credit balance is
not paid out in v1: a payment against nothing owed is refused, and the credit
settles the next credit sale instead.

**The closing avoir.** Avoirs on one facture add up to that facture, less
the droit de timbre it never refunds. They do not do so by themselves: the
tax on an avoir is rounded once on that avoir's own base, so three one-unit
credit notes against three units at 0,50 with 19 % come to 180 centimes
against a facture of 179, and at 0,80 they come to 285 against 286. So the
avoir that takes the last quantity off a facture, the closing avoir, is not
computed from its own lines at all. It is the facture minus every avoir
before it, field by field: HT, the discount, the subtotal, each rate's base
and tax, the TTC, and the stored lines the same way. The earlier partials
keep the slice arithmetic, which is right on their own paper, and their
running total is capped at the facture's `total_ttc` for the reason given
above: the stamp is never given back.

That cap is read per rate as well as on the total: a partial avoir never gives
back more base, more TVA or more remise at one rate than the facture still has
at that rate. Without it the subtraction goes below zero on a paper that cannot
carry it. Two partials that between them take a whole rate group can round
their tax to a centime more than the facture charged there, and a partial that
takes a group's whole HT can leave behind the centime of remise the facture put
on that group, and either way the closing avoir is asked for a base or a tax of
minus one centime at a rate whose goods have all come back. The centime stays
on the partial that rounded it, where it is one centime of rounding on a paper
that is already rounding.

A closing avoir can come to nothing. What is left of a facture may be a
quantity worth no centime, and the credit note that takes it back is then
written for 0,00: the goods go back on the shelf on it and it moves no debt at
all, because a movement of nothing moves nothing.

**Credit consumed at issue.** A customer holding credit who buys on credit
has the new document settled out of that credit as it is issued, inside the
sale's own transaction: an allocation from the credit onto the new document,
and `remaining_debt = net_to_pay − consumed`. The paper is what the customer
pays against, so it must not ask for money the shop already has. The sale's
own ledger movement is still the whole `net_to_pay`, because the credit is
already a movement of its own and writing only the unsettled part would
count it twice.

**Cancellation.** A document is annulled, never deleted: it keeps its number
and its row, and stores when it was annulled, by whom, why, and the avoir
the cancellation issued when it issued one. A ticket or a cash facture owed
nobody anything, so only the goods come back, on a `return` movement naming
the document itself. A facture that put money on a customer's account,
whether still owed or since paid, is undone by a whole avoir written in the
same transaction; a facture already credited in full is annulled with no
second avoir. A ticket that put money on an account is undone without one: an
avoir is written against a facture, so the goods go back and a single credit
ledger row of kind `avoir` names the ticket, settling the ticket's unpaid part
first and then the customer's other papers oldest first, with no number burned
out of the avoir series. The row is the whole of the ticket and not its unpaid
part, so a customer who had already paid something on it ends holding that
money rather than having paid for goods they gave back. Only a ticket and a
facture are annulled, by an allowlist rather than by naming what is refused,
and a document is annulled once. Cancelling a ticket that has already been
printed stays allowed until roles land (§5), and the audit log carries who
did it. A cancellation is refused without a reason, and the core answers what
a cancellation will do before it is taken: `GET /sales/{id}` carries a
`cancel_effect` beside the document, one of `nothing_to_reverse`,
`stock_back`, or `stock_back_and_avoir` with the amount that avoir would
carry. It is a union and not a word beside a nullable amount, so the amount
cannot go missing on the one shape that has one. The confirm on the screen
reads that field and nothing else: a facture whose goods have all come back
on earlier avoirs still carries debt, was still sold on credit and still
names a customer, and cancelling it does nothing at all.

**Proforma.** A quotation: made out to a named customer, priced the way the
till prices a basket, numbered out of the proforma series, and moving
nothing at all: no stock movement, no ledger row, a balance triple of three
zeros. It carries the droit de timbre the facture would carry, because a
facture that comes to more than the customer was quoted is a quotation that
was wrong. It requires a customer but not the party identifiers décret
05-468 art. 3 asks of a facture: a quotation comes before the paperwork.

### Fiscal rules: current assumptions

Each row names the fixture that pins it and the source that decides it.
"assumption" means nobody has read the law for it yet; a source cites code,
article and edition. Primary sources and findings live in
`research/legal-fiscal/`. **Confirm each with a comptable before the
first release.**

| Rule | Assumption | Fixture | Source |
|---|---|---|---|
| Money representation | integer centimes; no float anywhere in core; rates are integer basis points, 0 to 10 000 (1900 = 19 %), a rate above one whole is refused | `money_no_float`, invalid rates in `tva_rounding_once_per_rate` | design choice, not law |
| Rounding | integer centimes; TVA per rate group on the group's HT subtotal, rounded once, half away from zero, to the centime. A design choice: no text prescribes facture rounding | `tva_rounding_once_per_rate` | CTCA 2026 art. 80bis → CIDTA 2026 art. 324 governs the tax return (base to the lower dinar / ten dinars, duty to the nearest 10 centimes), not the document. Comptable to confirm facture practice |
| Line quantity and line total | a quantity is an integer number of thousandths of the unit (1500 is 1,5 kg), so a product sold by weight or by volume never needs a float. The line gross is `unit_price × qty_milli / 1000` rounded to the centime, half away from zero, once per line and before the line discount is taken off it. An assumption: no text says how a weighed line is rounded. Confirm with the comptable (R8) | `line_total_fractional_qty` | design choice, not law |
| Global discount spread | the global discount is allocated to the rate groups in proportion to each group's HT subtotal; each share is rounded down to the centime and the centimes left over go to the group with the largest HT subtotal, the lower rate winning a tie, never past that group's own HT (what it cannot take rolls to the next largest). The group bases always sum back to `total_ht − discount` and none is negative, so no centime is invented or lost. An assumption: no text says how a global discount splits across rates. Confirm with the comptable (R8) | `discount_spread_largest_remainder` (the `totals_cases` array in `tva_rounding_once_per_rate`) | design choice, not law |
| TVA rates | 19 % standard, 9 % reduced, 0 % exempt; rate per product, defaulted from category | `the_rate_defaults_from_the_category`, `an_explicit_rate_beats_the_category_default` | CTCA 2026 art. 21 (19 %), art. 23 (9 %, list by tariff line) |
| Régime fiscal | shop-level, dated setting `ifu` or `réel`. IFU: single price per product, no TVA rate, no HT/TTC, no TVA line on any document; réel: the TVA rows above. Documents keep the regime they were issued under | `an_ifu_ticket_names_no_tax_in_any_language`, `an_ifu_facture_names_no_tax_in_any_language` | CIDTA 2026 art. 282 ter (8 M DA threshold), 282 sexies (rates); CTCA 2026 art. 2-12 (out of TVA scope), art. 64 (must not mention TVA) |
| Droit de timbre | cash only (electronic exempt); nothing at 300 DA or less; tranches = ceil(amount / 100 DA); 1 DA per tranche up to 30 000 DA, 1,5 DA up to 100 000 DA, 2 DA above, the whole amount at its band's rate (no progressivity); minimum 5 DA; no cap. The base is `total_ttc`, the amount before the stamp itself: the text says "amount" and names no base, so this is an assumption to confirm with the comptable (R8). Open: half-dinar on odd tranches at 1,5. It is exact in centimes and only matters if the tax must be paid in whole dinars | `stamp_progressive_tranches` | Code du timbre 2026 art. 100-I and 258 quinquies; DGI circular 14/MF/DGI/LF.2025 (examples 1–3) |
| Amount in words | French, Arabic and English generators, dinars and centimes | `words_{fr,ar,en}_golden` | décret 05-468: total TTC "en chiffres et en lettres"; Arabic wording not yet sourced |
| Printed wording | the words a document prints live in the core, three languages per key. The Arabic is unreviewed by a native speaker, exactly like `words_ar`, and the Arabic goldens say so in their own header comment | `fixtures/print/*/ar*.html`, `the_dictionary_lists_every_key_once`, `every_key_is_written_in_all_three_languages` | none yet; R6 covers both this and the amount in words |
| Party identifiers | `facture`: seller RC + NIS (+ NIF, AI as on every facture in circulation), buyer RC + NIS, or name + address when the buyer is a consumer; stamp and signature blocks; `ticket`: seller identity only | `a_facture_to_a_company_carrying_its_identifiers_is_issued_in_the_facture_series`, `a_company_buyer_without_a_nis_refuses_the_facture_and_burns_no_number`, `a_facture_to_a_consumer_asks_for_a_name_and_an_address_and_nothing_else`, `a_shop_whose_settings_carry_no_nis_cannot_issue_a_facture_at_all`, `a_blank_identifier_is_as_missing_as_no_identifier_at_all` | décret 05-468 art. 3 and 4; NIF/AI from tax texts, article to cite (R3) |
| Avoir | a credit note is its own kind and its own series, may be partial, never carries the droit de timbre, and its TVA is per rate on its own lines. The running total of avoirs on one facture never passes that facture's `total_ttc` (the stamp is never given back, so capping on `net_to_pay` would leave the partials room to eat it), and a line is never credited past what earlier avoirs left on it. An assumption on the stamp: the Code du timbre taxes the payment and says nothing about a reversal, so not refunding it is a reading to confirm with the comptable (R8) | `a_whole_avoir_credits_the_facture_takes_its_own_number_and_carries_no_stamp`, `a_line_cannot_be_credited_past_what_earlier_avoirs_left_on_it`, `the_running_total_of_avoirs_never_passes_what_the_facture_asked_for`, `an_avoir_prints_no_stamp_and_one_that_carries_a_stamp_is_refused`, `fixtures/print/facture_a4/*-avoir.html` | Code du timbre 2026 art. 100-I for the stamp; décret 05-468 art. 10 for the series |
| Partial avoir discounts | a partial avoir credits the same share of the line discount and of the global discount as it credits of the line and of the basket, each rounded down to the centime. Rounded down because a discount is what the customer was not charged, and rounding it up would credit a centime nobody paid. A whole avoir carries the whole of both with no rounding, so it reproduces the facture's `total_ttc` to the centime. An assumption: no text says how a discount splits across a partial reversal. Confirm with the comptable (R8) | `a_partial_avoir_prorates_the_discounts_of_the_line_it_credits`, `a_partial_that_empties_a_rate_gives_back_that_rate_s_remise` | design choice, not law |
| Closing avoir | the avoirs on one facture add up to that facture less the droit de timbre. They do not do so line by line: the tax on each is rounded once on its own base, so slices of a facture sum to a centime either side of it. The avoir that takes the last quantity off the facture is the facture minus the avoirs before it, every field and every line, and the partials are capped at the facture's `total_ttc`. A design choice, not law: no text says how a reversal in parts rounds, and it is the sum a comptable reads that decides it. Confirm with the comptable (R8) | `the_avoirs_on_a_facture_add_up_to_it`, `three_one_unit_avoirs_add_up_to_the_facture_they_credit`, `two_partials_that_finish_a_facture_reproduce_every_field_of_it`, `two_partials_never_give_back_more_tax_at_a_rate_than_was_charged` | design choice, not law |
| Numbering | one uninterrupted chronological series per document kind and per year: each series restarts at 1 on 1 January of the shop's calendar and the printed number carries the year, `FA-2026-000001`; a cancelled document keeps its number, is marked "facture annulée" and stores when, by whom and why it was annulled; numbers never reused, and a cancelled number is never handed out again | `two_tickets_take_the_number_after_the_last`, `each_kind_counts_in_its_own_series`, `a_refused_line_burns_no_number`, `the_proforma_and_the_facture_series_do_not_touch`, `the_series_restarts_at_one_in_the_new_year_and_the_old_one_keeps_its_numbers`, `an_avoir_prints_the_year_of_the_facture_it_credits_and_not_its_own` | décret 05-468 art. 10; the yearly reset is common practice, not the decree (Samir, 2026-09-10; R8 confirms the practice) |
| Cancellation | a document is annulled, never deleted: it keeps its number and its row and stores when, by whom and why. Only a ticket and a facture are annulled, and each once. A cash ticket or a cash facture owed nobody anything, so only the goods come back; a facture that put money on an account is undone by a whole avoir in the same transaction, and a credit ticket by a single `avoir` ledger row and no number out of the avoir series. An assumption on the ticket: décret 05-468 governs the facture and says nothing about reversing a till receipt, so undoing one without a numbered document is a reading to confirm with the comptable (R8) | `a_cash_ticket_is_cancelled_the_stock_comes_back_and_the_number_stays`, `a_facture_carrying_debt_is_cancelled_through_a_whole_avoir`, `a_credit_ticket_is_cancelled_by_a_ledger_row_and_not_by_an_avoir`, `only_a_ticket_and_a_facture_are_cancelled`, `a_facture_already_credited_in_full_is_cancelled_without_a_second_avoir` | décret 05-468 art. 10 for the kept number |
| Debt slip | the paper a customer is handed at the counter carries the balance and the last ten movements, no TVA recap and no droit de timbre, and says on its face in each language that it has no fiscal value. A design choice, not law: no text names such a document, and it is not one: it reports an account rather than a sale | `the_slip_says_on_its_face_that_it_proves_nothing`, `the_slip_prints_the_newest_ten_movements_and_a_balance_that_counts_them_all`, `the_slip_prints_the_identifiers_of_a_company_and_never_a_consumers` | design choice, not law |

## 4. Printing (v1)

- Templates: `ticket_80mm`, `facture_a4`, `statement_a4`, `debt_slip_80mm`,
  each in ar/fr/en. HTML rendered by the core, not the UI, so desktop and
  any server print the same bytes. `facture_a4` is one template for the
  three kinds that share a facture's blocks: it titles itself facture,
  avoir or proforma, and A5 is a `Paper` argument that changes the `@page`
  size line and nothing else, so there is no separate `facture_a5`,
  `proforma_a4` or `avoir_a4` file to keep in step. What an avoir does not
  share with a facture is a title, the line naming the facture it corrects,
  a words line saying avoir, and the stamp row it never carries, against a
  second copy of the parties, the lines, the totals and the signatures.
  `barcode_label` and `bon_de_livraison_a4` are parked, the second with the
  facture récapitulative (see Later); the `kind` stays in the model.
- Every template × language is pinned by a golden file against a fixed
  fixture, and a template change is a reviewed golden diff.
  `ticket_80mm` is one basket sold four ways, three languages each:
  `fixtures/print/ticket_80mm/{fr,en,ar}.html` is réel and cash,
  `{fr,en,ar}-ifu.html` is the IFU, `{fr,en,ar}-card.html` is réel and card,
  which has no stamp row and neither half of the change, and
  `{fr,en,ar}-credit.html` is the credit ticket, which says so, carries no
  cash row and closes on the debt. Pinned by
  `crates/core/tests/print_ticket.rs`, which reads back the TVA rate of
  every recap row and of every line as well as every amount.
  `UPDATE_GOLDENS=1` rewrites the goldens and fails the run on purpose, and
  every amount in a golden is parsed back out of the file, by a reader that
  shares no code with the formatter, against the document's stored totals,
  so a golden that drifts from the money cannot be accepted by regenerating
  it. Every template below is pinned the same way.
  `fixtures/print/facture_a4/{fr,en,ar}.html` is a réel facture on credit
  to a company, with the balance triple and no droit de timbre;
  `{fr,en,ar}-cash.html` is cash to a consumer, with the stamp and no
  balance block; `{fr,en,ar}-ifu.html` is the IFU. Pinned by
  `crates/core/tests/print_facture.rs`, which also reads back the TVA rate
  of every recap row and of every line, the balance triple, the escaping of
  a product name that carries markup, and the words line against
  `amount_in_words` of the stored net.
- The other three faces of `facture_a4`, same file and same mechanics:
  `{fr,en,ar}-avoir.html` is a partial avoir of two lines against the credit
  facture, naming it in a line of its own ("Avoir sur facture FA-2026-000042 du
  09/09/2026", the referenced facture's year and day and not the avoir's), with no
  stamp row, its own words line and a balance block that says "solde
  créditeur" because the customer's total closes below zero;
  `{fr,en,ar}-proforma.html` is the same basket quoted, with a line saying
  it has no accounting value, is not a facture and creates no debt, and no
  balance block at all; neither of the two says how it was paid, because an
  avoir hands money back and a proforma settles nothing, and the avoir's
  last totals row is the amount of the avoir where a facture reads "net à
  payer"; `{fr,en,ar}-annulee.html` is the credit facture
  reprinted after cancellation, keeping its number and every amount, under a
  diagonal ANNULÉE mark with the day and the reason under the number. The
  test holds that the cancelled reprint and the live facture differ in the
  heading, the mark and the cancellation line only, that an avoir carrying a
  droit de timbre and a proforma carrying a debt are refused rather than
  quietly stripped, and that A5 differs from A4 in the `@page` line on every
  one of the six cases. What the row does not carry (the referenced
  facture's number and day, the cancellation's day and reason) is handed to
  the renderer beside the document.
- `statement_a4` and `debt_slip_80mm` are the customer's two papers, on the
  same mechanics: `fixtures/print/statement_a4/{fr,en,ar}.html` is a month of
  a company's account, opening balance, movements and closing balance, pinned
  by `crates/core/tests/print_statement.rs`;
  `fixtures/print/debt_slip_80mm/{fr,en,ar}.html` is a twelve-movement account
  of which the slip prints the newest ten, pinned by
  `crates/core/tests/print_debt_slip.rs`. Both read every amount back, both
  print a company's identifiers and never a consumer's, both say in words a
  balance the shop owes and whose way it goes, and the slip carries the line
  in each language that says it has no fiscal value. The slip's balance is the
  ledger's, so the golden proves the ten rows shown do not add up to it.
- The print language is the language the till is being used in, passed by
  the caller on each call as `?lang=fr|en|ar`, on every print route:
  `GET /sales/{id}/ticket`, `/sales/{id}/facture`,
  `/customers/{id}/statement` and `/customers/{id}/debt-slip`. There is no
  separate print-language setting in v1.
- Numbers are Western digits in every language, comma decimal, thousands
  grouped with a narrow no-break space (U+202F), and no currency word on a
  line: `fixtures/money/format_centimes.json` pins the core's formatter and
  the desktop's to each other. A document's printed number is
  `{prefix}-{year}-{number:06}`, `TK-2026-000123` for a ticket: the series
  restarts at 1 each year and the number carries the year it was taken in.
- The words a document prints are the core's own dictionary
  (`crates/core/src/print/strings.rs`), not the desktop's i18n JSON: a
  server with no UI prints the same paper. The Arabic in it is unreviewed by
  a native speaker, like `words_ar` (R6), and every Arabic golden says so in
  its own header comment (`fixtures/print/*/ar*.html`).
- Under the IFU a printed document has no TVA recap, no rate on a line and
  no "HT" on its total row: it must not mention the tax at all
  (`an_ifu_ticket_names_no_tax_in_any_language`,
  `an_ifu_facture_names_no_tax_in_any_language`), and "hors taxe" names one.
  A document carrying a TVA recap under the IFU is refused rather than
  quietly stripped. The facture
  drops its "Total TTC" row for the same reason, and prints "Prix unitaire"
  where the réel one prints "Prix unitaire HT".
- Thermal: ESC/POS over USB or Bluetooth from the desktop; from the phone
  via the desktop in LAN mode. "Any printer" means the OS print dialog for
  A4/A5 and raw ESC/POS for 80mm.
- Later: a QR code on the ticket, the shop's logo, and a footer text the
  owner sets.

## 5. Users and roles (v1)

Owner, manager, cashier. Login by PIN on the till, password elsewhere.
Permissions: sell, give discount above X %, override credit block, see cost
prices and margins, edit products, edit settings, see reports. Every
document records the user. Audit log of sensitive actions (price change,
discount override, delete, settings change, a quantity on hand put back to
what its ledger sums to), an ISO-27001 control we get
for nearly free by writing it now. An owner user exists from the first
migration, so every document, ledger row and audit entry carries a user
from the first sale (build-order step 2); PIN, roles and permissions
arrive in step 5.

## 6. LAN mode (v1, after the desktop milestones)

Exactly one desktop is the server; it advertises via mDNS and shows a QR
(host, port, short-lived pairing token). A phone or a second till scans it
and is a client from then on. Clients never own stock; a client that loses
the server queues writes and replays them, server answer wins. Windows
Firewall is the known trap: detect the blocked listener on first run and
show one instruction.

## 7. Cloud mode (open)

Same core binary hosted, one SQLite file per shop, account login. Not
started until decision 1. Nothing built before it may assume it does not exist:
every query is scoped by `shop_id`, every client talks HTTP.

## Later, agreed

- Batches/lots with expiry, product variants, bundles, promotions.
- AI invoice scanning on the phone (the one modern thing the competitor has).
- Phone-only offline via the Rust core compiled into the RN app (uniffi),
  never a TypeScript reimplementation of the calculations.
- Cheque and transfer as payment modes (decided 2026-09-08: v1 is cash,
  credit, card on a TPE).
- Bon de livraison and facture récapitulative together (décret 05-468
  art. 14–17 allow the first only with the second and a wilaya
  authorisation). The `bon_de_livraison` kind exists in the model; no
  template or screen until then.

## Open decisions

1. **SaaS with an account, offline licence, or both.** Anouar. Changes
   pricing, hosting and step 8 only.
2. **Product name.** Anouar; "Dinar" proposed and the org `Dinar-dz` created
   on 2026-09-08. The name changes the bundle identifier once, before the
   first release.
3. ~~TVA per product or one global rate.~~ **Decided 2026-09-08 (Samir):
   per product, defaulted from the category.** CTCA art. 23 lists the 9 %
   goods by customs tariff line, so the rate is a property of the product.
4. **A real printed facture** from any Algerian shop, to check the field
   list and layout against paper before the golden files are frozen.
