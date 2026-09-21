---
title: 'Whether Dinar becomes a core and modules'
slug: 'whether-dinar-becomes-a-core-and-modules'
status: 'active'
category: 'research'
created: 20260921
tldr: 'Anouar wants a shared core with a module per trade, doctors first, the way Odoo does it. This page is how that gets decided rather than argued: what is already known and measured, the one question that changes the answer, the two things to find out, the one day of work worth doing whichever way it goes, and the spike that prices the rest. It is a decision plan, not a build plan. Nothing here says yes and nothing says no.'
priority: 85
tasks:
  - id: 'D1'
    desc: 'The one question for Anouar: how many trades, over what horizon, and is any of them sold before it is built'
    status: 'pending'
  - id: 'D2'
    desc: 'What a clinic day actually is, one page, from Samir research rather than from imagination'
    status: 'pending'
  - id: 'D3'
    desc: 'The paper test: express one billed consultation in the document model that exists today, and record where it tears'
    status: 'pending'
  - id: 'D4'
    desc: 'Draw the kernel line as a rule with a test, no code moved, worth doing either way'
    status: 'pending'
  - id: 'D5'
    desc: 'The composition spike: can migrations, the permission match and the gate walk survive being split per module'
    status: 'pending'
  - id: 'D6'
    desc: 'The decision, written against criteria set down before the evidence arrives'
    status: 'pending'
---
# Whether Dinar becomes a core and modules

Anouar, 2026-09-21: keep the core, add a module per trade, doctors first,
the way Odoo does it. His argument in his own words is that managing admins
and settings is shareable, that customers and patients are the same module
with different strings, and that invoices are the same. Later the same
morning he added that once it is truly modular it becomes easy to work out
what a profession needs.

Samir's answer was that it is close to a new app, and that he will research
what doctors need before anything is designed.

This page exists so the decision is made once, on evidence, and so nobody
has to reconstruct today's measurements later. **Nothing is decided here.**
No plan is void, no retail task is dropped, and no page elsewhere is
rewritten as though the split were happening.

## What is already known, measured 2026-09-21 rather than guessed

**The repo.** 44 793 lines of code across 366 files, duplication 1.9%,
comment density 26.9%, about fifty lines of genuinely dead code in the whole
project, quality gate passing. Tests are roughly equal in size to product
code. The repo is fourteen days old, 1 117 commits, and has no users yet.

**What is actually shareable, by line, in the core services.** About a
quarter is trade-independent: users, sessions, permissions, audit, settings,
preferences, pairing, backup, the support bundle. A little over half is a
shop and nothing else: sales, purchases, avoir, both ledgers, stock,
pricing, the till, the dashboard. The rest is schema-shaped and gets
rewritten per product anyway: export, import, the seeder.

That ratio is the whole argument in one line. **The shareable quarter is the
cheap quarter.** Users, settings and audit are the part a competent
developer rebuilds in a week. The expensive half is the domain rules, and
those are exactly what differs between a shop and a clinic.

**Where Anouar is right.** The money kernel is real reuse: integer centimes,
checked arithmetic, TVA, stamp duty, rounding half away from zero, and the
printed layouts. It is already one module with one owner and it is pinned by
fixtures shared with the TypeScript side. Auth, roles, audit and the
`shop_id` column on every table but `shops` are a tenant skeleton worth
keeping. And a fiche with a name, a phone and a balance is the same table
whether the person is a customer or a patient.

**Where "invoices, same" needs testing rather than assuming.** Dinar's
invoice is a fiscal document, not a receipt: gapless numbering, IFU against
réel, avoir, stamp duty on cash. Whether a doctor's fees carry the same TVA
and stamp treatment is a question for a comptable, not for us. D3 is how
that gets answered on paper before anybody writes a module system for it.

**What Odoo proves, both ways.** It proves the model works, over about
twenty years. It also shows the price: a dynamic ORM, models extendable at
runtime, a module registry with dependency resolution, and views defined as
data rather than code. Odoo's shared core is double-entry accounting plus
partners plus products, which genuinely is the same in every trade, and that
is why it holds there. Odoo did not start modular for verticals it hoped to
sell; it was an ERP first, the accounting core proved itself, and the
verticals came after. The order matters.

## The two shapes, and only one is affordable

**Build-time verticals.** A `dzpos-kernel` crate holding money, auth, users,
roles, audit, settings, printing, backup and the sync plumbing, then sibling
crates per trade depending on it, one binary built per trade. Still
compiled, still typed, no runtime loader. This is a workspace rearrangement.

**Runtime plugins.** Rust has no stable ABI, so a loadable module means a C
boundary or an embedded scripting layer, and at that point the dynamic ORM
we were avoiding has been rebuilt by hand. Not on the table unless D1 comes
back with a number that justifies it.

## The four things that decide the cost, none of them the code move

1. **The schema.** `crates/core/src/schema.rs` is one generated file and the
   migrations are one ordered global sequence. Per-module migrations that
   compose is the largest single piece of work in the idea and has to be
   solved before anything else.
2. **Permissions.** The role check is an exhaustive match, which is why
   adding one refuses to compile until all three roles place it. That
   compile error caught a mistake twice this month. A module contributing
   permissions turns it into a runtime registry and the error goes away.
3. **The gate walk.** `crates/api/tests/route_gates.rs` reads the router's
   own source and fails a route with no gate row, in both directions. Split
   the router per module and that walk has to compose or the guarantee is
   gone.
4. **The dashboard and the reports.** They aggregate across every domain.
   Generic aggregation over module-owned tables is the ORM problem again, in
   the one place where a wrong number is a wrong number on a shop's screen.

## The prerequisite nobody has noticed

`crates/core/tests/services_go_through_services.rs` still pins three import
rings through **users, sessions, audit and preferences**. Those four are
exactly the kernel Anouar wants to share, and today they import each other
in circles. T11 of `architecture-fixes-without-a-domain-split` is the task
that cuts them. **If those rings cannot be cut, there is no kernel to
carve**, and that is a cheaper thing to find out than a refactor.

## The tasks

**D1: the question for Anouar.** How many trades, over what horizon, and is
any of them sold before it is built? This one answer moves the decision more
than anything else on this page. A module system pays for itself around the
third vertical. At one hypothetical second product, copying the repo and
deleting what does not fit is cheaper, and the shared parts can still be
lifted out later with two real cases in hand instead of one imagined one. At
five, he is right and Odoo is the right model. Blocked on him. Size S.

**D2: what a clinic day actually is.** One page, from Samir's research, in
the same shape as `docs/features.md` rows: what happens, in what order, who
touches it, what is printed, what is owed and by whom. Not a feature list
and not a comparison with Dinar. The question it has to answer is which
parts of a shop's day and a clinic's day are the same transaction, because
the code follows that answer and not the other way round. Size M.

**D3: the paper test.** Take one billed consultation from D2 and express it
in the document model that exists today: `documents`, its lines, a payment
mode, a customer fiche, the stamp and TVA rules of
`docs/features.md`. On paper, no code. Record exactly where it tears: a
field that has no meaning, a rule that does not apply, a thing a clinic
needs that has no home. That tear list is the real boundary between the
kernel and a trade, measured rather than argued. Size S.

**D4: draw the line as a rule, not a refactor.** Nothing in users, sessions,
permissions, audit, settings, preferences or the money kernel may name a
product, a sale, a supplier or a stock movement. Add it to
`crates/core/tests/services_go_through_services.rs`'s family of source-walk
tests so it fails closed like the others. No file moves and no crate is
created. This is worth doing whether or not the clinic ever happens, because
it stops the boundary getting harder to draw while the answer is pending,
and on the day of a split the line is already proven. Roughly a day.
Depends on T11 being done first, or it will fail on the rings. Size S.

**D5: the composition spike.** Take the four costs above and price them, in
a throwaway branch that is never merged: can diesel migrations be composed
from more than one source directory, can the permission match stay
exhaustive with a second crate contributing variants, can the gate walk read
two routers. A week is too long; two days and an honest answer of "this one
does not compose" is the result we want. Only start it if D1 comes back with
more than two trades. Size M.

**D6: the decision.** Written against criteria set down before the evidence,
so the evidence is not read backwards to fit a preference already held. The
criteria: the number from D1, the size of D3's tear list, and whether D5
found a cost that cannot be paid. Recorded in this file with the date and
who decided, the way the rulings are. Size S.

## What this plan does not do

It does not design the module system, it does not name the crates beyond
sketching them, and it does not touch a line of shipped code except D4's
test. A design written before D2 and D3 would be a plan for a product nobody
has looked at yet, and that is the thing most likely to be stale by the time
anybody reads it.

## Order

D1 goes out today because it is a question to a person and everything else
is cheaper once it is answered. D2 is Samir's research and runs on its own
clock. D4 waits for T11. D3 waits for D2. D5 waits for D1. D6 waits for all
of them, and the current loop's remaining work finishes regardless, because
a till that opens and closes is worth having in any trade that takes cash.
