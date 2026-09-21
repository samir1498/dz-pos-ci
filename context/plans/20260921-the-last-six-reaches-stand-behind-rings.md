---
title: 'The last six reaches stand behind rings'
slug: 'the-last-six-reaches-stand-behind-rings'
status: 'active'
category: 'architecture'
created: 20260921
tldr: 'Three of the six pairs are blocked by an import ring and three are not, corrected on 2026-09-21 by reading the imports. The shape the loop file proposed does not survive the walk that counts reaches; the loop file own one-operation rule picks the shape instead.'
priority: 70
tasks:
  - id: 'T1'
    desc: 'This page: what actually blocks each of the six, what the walk counts, and the two shapes with their costs'
    status: 'done'
  - id: 'T2'
    desc: 'Close the blind spot in the raw-SQL walk and the handler walk: a folder is invisible to both'
    status: 'done'
  - id: 'T3'
    desc: 'The purchases door onto supplier_debt: three repo calls, no ring in front of them'
    status: 'done'
  - id: 'T4'
    desc: 'The ownership lift at documents.rs:145, which frees customers -> documents and debt -> documents together'
    status: 'done'
---

# The last six reaches stand behind rings

`REACHES_PAST_A_SIBLING` in `crates/core/tests/services_go_through_services.rs`
is down to four rows over six pairs, from seventeen over eleven. The loop
file plans the rest as branch d, "the mutual pair", with the debt rows added
to it on 2026-09-21. That framing is wrong in a way worth writing down before
anybody builds: it is not that nobody got to these six. Every one of them has
a ring standing in front of it.

## What blocks each pair

| Pair | The ring in front of it |
|---|---|
| `customers -> documents` | `documents.rs:23` imports `services::customers`, for one call |
| `debt -> documents` | `documents.rs:23` to `customers.rs:22` to `debt` |
| `debt -> customers` | `customers.rs:22` imports `services::debt`, four calls, one a write |
| `supplier_debt -> purchases` | `purchases.rs` imports `services::supplier_debt`, seven calls |
| `supplier_debt -> suppliers` | `suppliers.rs:23` imports `services::supplier_debt`, five calls |
| `purchases -> supplier_debt` | nothing. Corrected 2026-09-21, see below |

**The row with no ring.** This page first said `supplier_debt.rs` imports
`services::purchases` and called the purchases pair mutual. It does not.
`supplier_debt.rs:30` imports `repos::purchases`, and its only sibling
services are `audit`, `clock` and a field helper. `purchases.rs` is the one
that imports `services::supplier_debt`, and calls it in seven places
already. So the three `debt_repo::{balance, append}` calls at
`purchases.rs:754-775` are not held up by anything: they are a door that was
never added, in a function whose own doc says why it does not go through
`supplier_debt::pay` (that one refuses money above the balance, and this
path is allowed to). `services::supplier_debt` gains the door that writes
the entry this path needs, `purchases` calls it, and the row leaves with no
ring touched. That is T3, and it is the shape of T4 branches a, b and c
rather than anything new.

In four of the six the sibling's service already has the exact function the
reaching service wants. `services::suppliers` has `get` at line 82 and
`supplier_belongs_to_shop` at line 88, which is precisely what
`supplier_debt.rs:800` and `:814` reach into `repos::suppliers` for. The
call cannot be written, because writing it closes a ring
`RINGS_STILL_OPEN` refuses to grow.

## Why the shape the loop file names does not work

The loop file says to do what `services::pricing` did: put the piece both
sides need in a module below both. That worked for pricing because
`pricing.rs` contains no `repos::` at all. It is arithmetic.

What these six need moved is not arithmetic. It is ownership predicates and
single-row reads: does this customer belong to this shop, does this
supplier, fetch this purchase. Every one of them queries a table.

`no_service_reaches_a_repo_that_is_not_on_the_list` counts a reach as any
`repos::Y` named in `services/X.rs` where Y is not X and not on the
four-name no-service list. A new `services::ownership` holding those
predicates therefore arrives as a new row of its own,
`("ownership", ["customers", "documents", "suppliers"])`, and the constant
is an exact-equality assertion that refuses growth in either direction. The
shared module does not remove reaches. It relabels them and adds one.

## The two shapes, and what each costs

**Shape A, break the rings at the thin end.** Four of the six rings exist
for one call each or close to it. `documents.rs` uses exactly one thing from
`customers`, the ownership predicate at line 145. Lift that one operation to
the caller that needs the answer, or have `documents::issue` take a customer
it has already been told belongs to the shop, and `documents` stops
importing `customers`. That frees `customers -> documents` and
`debt -> documents` at once, because the chain through `customers` is what
made the second a ring. Cost: the check moves up to every caller of
`issue`, and a caller that forgets it is a customer of another shop on a
document. That is the failure the predicate exists to prevent, so the move
has to make forgetting impossible rather than merely unlikely, which means
a type that can only be made by passing the check.

**Shape B, let the constant carry one exemption.** Accept a shared
repo-touching module and give it a row in `REACHES_PAST_A_SIBLING` with a
comment saying what it is. Cost: the constant stops meaning "every reach
that skips a sibling's rules" and starts meaning "every reach except the
ones we decided were fine", which is the property that made it useful. It
also cannot then be deleted, and deleting it is the stated end of this work.

Shape A is what gets built, and it is not a ruling Samir has to give: the
loop file's own pre-flight line for this branch says to check "what
`customers.rs` uses from `debt` and what `documents.rs:135` uses from
`customers`: if either is one operation, lifting that one above both is
cheaper than a new module." `documents.rs` uses exactly one operation from
`customers`, so the loop file resolves itself on that reading. Shape B also
cannot be the answer on its own terms, because it ends with a constant that
can never be deleted and deleting it is the stated end of this work. If
Samir later prefers B, nothing built under A is wasted: the rows A removes
are gone, and there is nothing left to exempt.

## What is actually reachable, and what stays

Shape A plus the door above take three of the six pairs out:

- `purchases -> supplier_debt`, on the door, no ring involved (T3).
- `customers -> documents` and `debt -> documents`, both freed by lifting
  the one ownership call at `documents.rs:145` (T4).

Three stay, and each is thick rather than blocked by an accident:
`debt -> customers` (four calls into `services::debt` from `customers.rs`,
one of them the ledger write at line 129), `supplier_debt -> purchases`
and `supplier_debt -> suppliers` (five calls from `suppliers.rs`). So
`REACHES_PAST_A_SIBLING` goes from four rows over six pairs to two rows over
three, the `customers` and `purchases` rows leave whole, and the constant is
not deleted. The trailer verb for this work is `progress`, not `close`, and
the loop file's "five rows leave, the constant is deleted with them" is
wrong for the same reason the table above was.

## A blind spot in the neighbouring walks

Written wrong the first time and corrected here.
`the_walk_cannot_be_stepped_around` does refuse a folder under
`crates/core/src/services/`, at its own last assertion, so the reach walk
and the ring walk are covered.

The two walks beside them are not. `repos_own_the_queries.rs` reads the same
directory one deep and keeps only a `.rs`, so a service at
`services/foo/mod.rs` is one no rule in that file applies to, and that file
holds the sharper rule: its own comment says a statement escaping it "can
write, and it writes outside every repo and every `shop_id` filter".
`crates/api/tests/one_handler_decides.rs` has the same shape over
`crates/api/src/routes/`, where a handler in a folder would decide a
permission with nothing watching. `crates/api/src/gates/` is a folder
already, so this is a shape somebody reaches for without meaning to evade
anything. T2 gives both the assertion the first walk already had.

## What is not in question

The rows leave by moving a door, never by widening the list. Shorter is the
only direction either constant goes. `debt.rs:581` writes
`documents_repo::set_remaining_debt` and `remaining_debt` is a field of the
§3 totals table, so whatever door it ends up behind is `pub(crate)`, like
`avoirs_of` and `list_in_range` that #128 added, and never `pub`.
