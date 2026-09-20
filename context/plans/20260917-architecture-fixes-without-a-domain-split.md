---
title: 'Architecture fixes without a domain split'
slug: 'architecture-fixes-without-a-domain-split'
status: 'active'
category: 'refactor'
created: 20260917
tldr: 'Every problem the architecture review found has a fix that costs one file or one function. None of them needs the layer split turned into domain modules, and the dto.rs split has to land before the Lumina branches start adding wire types to it.'
priority: 75
tasks:
  - id: 'T1'
    desc: 'This page: the fix for each finding and the one thing deliberately not done'
    status: 'done'
  - id: 'T2'
    desc: 'crates/api/src/dto.rs, 3071 lines, becomes dto/{domain}.rs matching routes/, with dto/mod.rs re-exporting everything so no import anywhere else changes. Lands first and alone'
    status: 'done'
  - id: 'T3'
    desc: 'Break the two service cycles: documents imports avoir while avoir imports documents, sales imports proforma while proforma imports sales. The shared types move to a module both import'
    status: 'pending'
  - id: 'T4'
    desc: 'The nine services that reach past a sibling service into its repo go through the service instead; the missing functions get added to the sibling. Sharpest case is services/debt.rs:25,27 writing through repos::customers and repos::documents'
    status: 'pending'
  - id: 'T5'
    desc: 'services/stock.rs:12 calls services::audit::record instead of repos::audit, so the audit row gets the shop clock'
    status: 'pending'
  - id: 'T6'
    desc: 'routes/products.rs:34 redact_cost: the permission decision moves out of the handler, or the page says why this one stays and gates.rs stops citing it as the thing route gating avoids'
    status: 'pending'
  - id: 'T7'
    desc: 'docs/architecture.md:129 gains the exception it is missing: backup and support_bundle run SQL outside repos, deliberately, because database administration has no aggregate'
    status: 'pending'
  - id: 'T8'
    desc: 'Frontend: apps/mobile/lib/basket.ts:72 uses parseAmountToCentimes; the three as assertions in the phone become zod or type guards; suppliers.tsx at 1086 lines becomes routes/-suppliers/ like -till/ and -customers/; PAYMENT_METHODS and the balance label move to components/ instead of being copied'
    status: 'pending'
  - id: 'T9'
    desc: 'The two JSX money sums, purchases.tsx:260 and purchases_.$id.tsx:232, stop adding transport and extra costs in the component; the API answers the total. Money work, so the money builder and a mirror CI run'
    status: 'pending'
  - id: 'T10'
    desc: 'Burn the file-size list down. Samir, 2026-09-17: no huge files. The gate is in and pins 32 of them; the ones worth splitting first are suppliers.tsx at 1086, products.tsx at 929, customers_.$id.tsx at 898 and documents.tsx at 724, because a screen that long is the one nobody reads before changing it. The Rust services on the list are a separate argument and stay pinned for now'
    status: 'pending'
acceptance:
  - 'No file in crates/api is over a thousand lines and no import outside dto/ changed'
  - 'No service imports a sibling that imports it back'
  - 'No service calls a repo that belongs to another domain'
  - 'No component computes a total the API could answer, and no f64 sits anywhere near an amount in any app'
  - 'scripts/file-sizes.json is shorter than the 32 entries it started with, and no entry grew'
---
# Architecture fixes without a domain split

The architecture review of 2026-09-17 asked whether dz-pos is a modular
monolith or a mess. It is neither: it is a layered split, models to repos to
services to print, executed the way `docs/architecture.md:126-139` prescribes,
with `pub(crate) mod repos` at `crates/core/src/lib.rs:14` turning "nothing
outside this crate touches diesel" into a compile error rather than a review
comment.

So the question this page answers is the one Samir asked next: can the
findings be fixed without restructuring into domain modules. Every one of them
can, and this is how.

Restructuring is not on the table and not because it is hard. It would touch
every file in `crates/core`, contradict a written rule, and buy a property
nobody has needed yet. The findings are individually cheap; the restructure is
the only expensive option in the room.

## The file-size gate, added 2026-09-17

Samir asked for a no-huge-files rule while T2 was merging. It is `just sizes`,
a ratchet over `scripts/file-sizes.json`: 600 lines for a source file, 1200 for
a test file, the 32 already over it pinned at today's length, none of them
allowed to grow, and an entry that comes back under the limit has to be
deleted. Nothing new joins the list.

That turns T8's `suppliers.tsx` split from a good idea into something the gate
will keep. It also means every task on this page now has a second obligation:
if it shortens a pinned file, it lowers or deletes that file's entry in the
same commit, or the gate fails.

## Why T2 goes first and alone

`dto.rs` is 111 wire types and 65 `From` impls in one file, for every domain
at once, while `routes/` next door has been split by domain from the start.
It is the file every feature touches.

The Lumina work adds wire types for a print layout setting and for whatever
the later gaps need, on branches that run in parallel. Every one of those
branches appends to `dto.rs`, and they all collide in the same place. Split it
before they start and each branch adds its type to its own domain file. Split
it afterwards and the split itself conflicts with everything in flight.

The split is mechanical and has no behaviour in it: `dto/mod.rs` re-exports
every type, so not one import outside the folder changes. Its own proof is
that `just gates` passes with zero edits anywhere but `crates/api/src/dto*`.

## The two cycles, and what breaking them means

`services/documents.rs:25` imports `avoir` and `services/avoir.rs:47` imports
`documents`. `services/sales.rs:33` imports `proforma` and
`services/proforma.rs:26` imports `sales`. Rust allows cycles inside a crate
so these compile fine; what they cost is that neither half can be read, tested
or moved without the other.

The fix is not to sever a dependency that is genuinely there. It is to find
what the two actually share, usually a type and a small rule, and give it a
module both import. If after looking there is no shared kernel and the
dependency really runs both ways, write that on this page with the reason and
close the task. A cycle somebody understood is a different thing from one
nobody noticed.

## The one thing deliberately not done

The review found no test seam anywhere: zero traits in either crate, every
service taking a concrete `&mut SqliteConnection`, so every service test is an
integration test against a migrated temp file.

That stays. SQLite against a temp file is fast, and a fake connection would
test that the fake behaves like SQLite. Traits and manual DI here would be
work with a payoff nobody has asked for. The day a service needs a unit test
that cannot open a database, this becomes worth revisiting; that day has not
come in nine milestones.

This is written down because it is the finding most likely to be
rediscovered, and the second most likely to be fixed by reflex.

## T8, the half that landed on 2026-09-20

The phone's two non-negotiable breaches are closed (#118). `readTendered`
calls `parseAmountToCentimes`, `formatCentimes` is re-exported from
`@dzpos/shared` rather than copied, and the three `as` assertions are
runtime guards. Type guards, not zod: the phone has no zod dependency and
three small readers do not earn one. The one assertion left is the wire in
`bodyOf<T>`, exempted in place with a comment.

Four things that pass taught more than the fix did and are worth keeping:

The formatter and the parser were coupled and neither knew it. The shared
formatter groups thousands with U+202F, and the phone's old regex rejected
that, so moving one without the other would have killed the Exact button on
every basket over 1000 dinars. A round trip through both is pinned in
`apps/mobile/lib/basket.test.ts`.

A guard can erase what it was written to protect. The first queue check
demanded a `sessionToken` key, and that field arrived with the Expo SDK 57
rebuild (838e4ee), so a sale rung offline before that update would have
been dropped on the first read after it, with nothing in the queue count to
say it had gone. A missing signer is filled in as null.

`formatCentimes` throws on anything that is not a safe integer, and the
phone's own `call<T>` hands a body back with no schema behind it. Any
screen that formats a number straight off the wire is one server bug away
from taking the till down, and the phone carries no error boundary. The
till's change now goes through `readChange` first.

The test lens caught both blob fixtures short of several fields at once, so
deleting the check the test named left it green. Mutation confirmed it. A
row per field is the shape that holds.

Still open in T8: `suppliers.tsx` at 1086 lines becomes `routes/-suppliers/`,
and `PAYMENT_METHODS` plus `supplierBalanceLabel` move to `components/`
instead of sitting in both `suppliers.tsx:73-80` and
`-customers/parts.tsx:24,49`. That half is also T10's largest file.
