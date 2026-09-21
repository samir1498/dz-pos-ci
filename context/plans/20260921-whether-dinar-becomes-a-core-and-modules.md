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
    desc: 'Answered 2026-09-21: no trade is named and none is sold. The goal is that adding a module never breaks existing logic, which is a safety ask and not an architecture one'
    status: 'done'
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
    desc: 'Cut to one question on 2026-09-21: only the database is open, because global permissions settled the other two. Waits for a second product to have a name'
    status: 'pending'
  - id: 'D6'
    desc: 'The decision, written against criteria set down before the evidence arrives'
    status: 'pending'
  - id: 'D7'
    desc: 'The brainstorm on module shape: one subagent per candidate with pros and cons against the same constraints, then the session judges with advisor guidance; Samir listed the candidates on 2026-09-21'
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

## What Anouar answered, 2026-09-21, 10:19

Asked in effect how many trades and over what horizon, he said it does not
have to be a doctor, it can be anything, and that the important thing is to
make it modular enough that **adding new modules will not break the logic**.

That is worth reading carefully, because it changes the question twice.

**No second product is named and none is sold.** The horizon question is
answered, and the answer is that there is no second customer yet. Extension
points designed for modules nobody has specified is the ordinary way to
build an abstraction that fits nothing, and then to live with it.

**But the goal underneath is not really modularity.** It is that feature six
must not break feature two. That is a legitimate fear and a better-defined
goal than a module system, and Dinar answers it already, by a mechanism
nobody has told him about:

- a route with no permission row fails `crates/api/tests/route_gates.rs`,
  which reads the router's own source and walks it in both directions.
- a service reaching past a sibling into its repo fails an exact-equality
  assert in `crates/core/tests/services_go_through_services.rs`.
- a new permission refuses to compile until all three roles place it. That
  compile error caught a real mistake twice in September.
- a pinned file fails `scripts/file-sizes.mjs` for growing **and** for
  shrinking without the pin being lowered.
- the layer rule, the one-handler rule and the ring walk are each a test
  rather than a convention.

That is what keeps an addition from breaking what exists, and it is stronger
than module isolation, because it holds inside a module as well as between
two of them.

**The irony to put to him plainly:** a plugin system would weaken two of
these. The exhaustive permission match becomes a runtime registry and the
compile error goes away, and the gate walk has to learn to read more than
one router. The thing he wants protection from is the thing a plugin system
makes harder to check.

So D1 is answered and the plan below is not cancelled by it. D4 becomes the
task that gives him what he is actually asking for, and D2, D3 and D5 wait
for a product that has a name.

## What Samir decided, 2026-09-21

Asked how the permission list should work if Dinar ever grows a second trade,
he said a global model, and that less complexity is what he wants. Shown the
build-time middle option, where each trade crate declares its permissions and a
macro stitches them into one enum at compile time, he turned it down.

**Permissions stay one global list.** Every permission lives in the one enum in
`crates/core/src/services/permissions.rs:39`, doctor ones added to the same
list on the day they exist, and the role table stays the exhaustive match at
`:206`. No generated code, no runtime registry. A shop build carries a few
variants it never uses, which costs nothing: the whole table is about twenty
lines for fifteen permissions.

This is reversible for free. Every check in the codebase is
`permissions::require(role, Permission::X)` and reads exactly the same whether
the enum was written by hand or generated, so the generated version can be
adopted the day the hand-written list gets genuinely ugly with three trades in
it, without touching a single call site.

Two of the four costs below fall out of that decision. **Cost 2 is paid**: the
compile error that caught a real mistake twice in September survives, because
the match is still exhaustive. **Cost 3 is paid**: the gate walk only had to
learn to read more than one router if permissions split per module, and they do
not.

**Loading Rust code into the running binary is out**; that is the one
shape ruled out, not plugins as such. Samir said the same afternoon that what
he means by plugin is closer to a microservice with an API boundary, and that a
full rewrite on a stack built for plugins is on the table. So the module shape
is an open question with several candidates, and D7 below is how it gets
answered rather than argued.

**The schema is the one thing left open.** Samir said he is not sure about the
database, and it is the right thing to be unsure about: per-module migrations
that compose is the largest single piece of work in the idea. See D5 below,
which is now a much smaller question than the one it was written as.

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

**D1: the question for Anouar. Answered on 2026-09-21, see above.** The
original question was: how many trades, over what horizon, and is
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

**D5: the migration spike, cut down from three questions to one.** Samir's
decision on 2026-09-21 answered two of the three it was written for: the
permission match stays exhaustive because permissions stay global, and the
gate walk keeps reading one router for the same reason. What is left is the
database, which he said he was unsure about, and it is the part worth being
unsure about.

The question, in a throwaway branch that is never merged: can diesel
migrations be composed from more than one source directory, or does a second
trade have to put its tables in the same ordered list as the shop's. The
cheap answer, which needs no spike, is one shared list for everything: a shop
install carries the doctor tables sitting empty, and every module's migrations
live in one sequence in one repo. That costs nothing at this scale and is
what "less complex" buys. The spike is only worth running if that shared list
turns out to be unacceptable for a reason nobody has named yet.

So: not started, and not startable until a second product has a name. Size S
now rather than M. Still gated on D1 naming a trade.

**D6: the decision.** Written against criteria set down before the evidence,
so the evidence is not read backwards to fit a preference already held. The
criteria: the number from D1, the size of D3's tear list, and whether D5
found a cost that cannot be paid. Recorded in this file with the date and
who decided, the way the rulings are. Size S.

## D7: the brainstorm on module shape, for after this pass

Samir, 2026-09-21, 15:29, on how the module question gets worked when the
current loop closes: consider every case, one subagent brainstorms each with
its pros and cons, then the session is the final judge with the advisor's
guidance. The `superpowers` plugin's brainstorming skill is used if it earns
its place. Nothing below is decided; it is the list to start from so the
brainstorm does not begin by rediscovering it.

**The constraints every candidate is judged against, the same for all:**

- One offline desktop in a shop, a phone on the LAN, no server. A power cut
  is an ordinary day.
- Windows only. Shops buy a PC for the till and it runs Windows 10, sometimes
  still Windows 7. Linux and macOS are not targets. This is a hard filter:
  Tauri's WebView2 stopped at version 109 on Windows 7 in January 2023, and
  Electron dropped Windows 7 at version 23, so any candidate has to say what
  it does on a Windows 7 machine, and the brainstorm's first job is to find
  out whether Windows 7 is real in the shops Anouar knows or a guess.
- Money writes stay atomic. A sale writes the document, its lines, the stock
  movement and the ledger in one transaction today
  (`crates/core/src/services/sales.rs:228`), and a shape that gives that up
  has to say how it gets it back.
- The safety Anouar actually asked for: adding a module cannot break an
  existing one. Today five source-walk tests and an exhaustive permission
  match give that; a candidate says which of those survive it.
- Less complex is what Samir wants. A candidate that is elegant and heavier
  loses to one that is plain and lighter.
- Permissions stay one global list (decided above).

**The candidates, Samir's list plus the ones it implies:**

1. **Compile-time domains.** One workspace, a crate per trade depending on a
   kernel crate, one binary built per trade with cargo features. The current
   code, rearranged. The cheapest and the one the measurements on this page
   favour.
2. **Microservice-shaped modules on one machine, one database.** Each module
   is its own process talking to the core over HTTP on localhost, all reading
   one SQLite file. Has to answer the single-writer lock and the transaction
   question.
3. **The same, with data separated per module**, in one of three ways: one
   database with a table prefix per module; one SQLite file per module
   `ATTACH`ed to the core's; or a schema per module, which SQLite does not
   have and which would mean leaving SQLite. Each is its own case because the
   foreign keys and the joins the dashboard needs behave differently in each.
4. **Keep Tauri and Rust, split the kernel from retail, and make retail the
   first module on the current stack.** The in-process version of the
   plugin idea: a trait per extension point, modules registered at startup,
   one binary. Where D4's boundary test leads if it is followed through.
5. **A full rewrite on a stack built for plugins.** The candidates worth
   naming so they are priced rather than imagined: .NET on Windows (WPF or
   WinUI, plugins as assemblies loaded at runtime, the shape most Windows
   till software already has); Electron or a plain web app with a local
   server and JavaScript plugins; a Rust core hosting **WebAssembly**
   modules (`wasmtime` or `extism`), which is the one way to get real
   runtime plugins without leaving Rust and is not the C boundary the
   earlier paragraph dismissed.
6. **Fork per trade.** Copy the repo, delete what does not fit, lift the
   shared parts out later with two real cases in hand. Not a module system
   at all, which is why it belongs on the list: it is the baseline every
   other candidate has to beat.
7. **The Odoo shape as Odoo actually does it.** One process, one database, a
   module registry, models extended at runtime by a dynamic ORM. Priced
   honestly rather than admired, because it is the reference Anouar named.

**How it runs.** One subagent per numbered case, all handed the same
constraints block above, each returning pros, cons, what it costs to get from
today's code to it, what it does on Windows 7, and the one thing that would
kill it. Two agents at a time within the four-agent cap. Then the session
reads all seven, calls the advisor on the comparison, and writes D6 against
the criteria already set down. Anything a subagent invents beyond its case is
noted and not acted on.

## What this plan does not do

It does not design the module system, it does not name the crates beyond
sketching them, and it does not touch a line of shipped code except D4's
test. A design written before D2 and D3 would be a plan for a product nobody
has looked at yet, and that is the thing most likely to be stale by the time
anybody reads it.

## Order

D1 goes out today because it is a question to a person and everything else
is cheaper once it is answered. D2 is Samir's research and runs on its own
clock. D4 waits for T11. D3 waits for D2. D5 waits for D1. D7 runs when the current loop closes and D6 waits for it and the rest,
and the current loop's remaining work finishes regardless, because
a till that opens and closes is worth having in any trade that takes cash.
