---
title: 'TanStack Table under DataTable, and the phone''s forms'
slug: 'tanstack-table-under-datatable-and-the-phone-s-forms'
status: 'paused'
category: 'frontend'
created: 20260916
tldr: 'Put @tanstack/react-table under the desktop''s DataTable so 22 screens get sorting, filtering and paging without each one hand-rolling it; bring the phone''s three forms onto @tanstack/react-form. Not before the MVP demo.'
priority: 35
tasks:
  - id: 'T1'
    desc: 'react-table under DataTable with getCoreRowModel only; no screen changes, rendered markup identical'
    status: 'pending'
  - id: 'T2'
    desc: 'sortable?: true on Column, getSortedRowModel, header button with aria-sort; documents list only'
    status: 'pending'
  - id: 'T3'
    desc: 'move the screens that hand-roll a sort onto it, one PR each, deleting local sort state as each lands'
    status: 'pending'
  - id: 'T4'
    desc: 'filtering where a search box already exists; server-side for query-backed lists, getFilteredRowModel only for in-memory ones'
    status: 'pending'
  - id: 'T5'
    desc: 'pagination, only if a real shop''s list proves long enough to need it'
    status: 'pending'
  - id: 'T6'
    desc: 'phone''s pair / sign-in / tendered forms onto @tanstack/react-form, sharing one validation statement with the desktop'
    status: 'pending'
acceptance:
  - 'After T1 the Column<Row> interface is byte-identical and no file under src/routes changed; if a screen had to change, T1 was done wrong'
  - 'The count of hand-written sort/filter expressions in apps/desktop/src/routes/*.tsx falls from 41 toward zero, measured the same way each PR'
  - 'A sortable header announces its direction to a screen reader (aria-sort), and a money column stays right-aligned and tabular after sorting'
---
# TanStack Table under DataTable, and the phone's forms

Deferred on purpose: nothing here is needed for the MVP demo video. Start it
after the demo is cut.

## What is actually true today (checked 2026-09-16, not assumed)

The earlier read that "only TanStack Query is used" is wrong. Counted in
`apps/desktop/src`:

- `@tanstack/react-router` — 35 files, the whole routing tree (`src/routes/*`,
  `main.tsx`, `__root.tsx`). In use.
- `@tanstack/react-form` — 7 files (`settings`, `settings_.users`, `products`,
  `customers_.$id`, `-customers/fiche`, `suppliers`, `expenses`). In use.
- `@tanstack/react-query` — 50 files. In use.
- `@tanstack/react-table` — **not installed anywhere.** This is the only gap,
  and it is the only thing this plan migrates.

The phone (`apps/mobile`) is a separate story: expo-router (TanStack Router
does not target React Native), react-query, and three forms written on plain
`useState`.

## The gap worth closing

`apps/desktop/src/components/DataTable.tsx` is 127 lines over
`components/ui/table.tsx` (113). It is good — a money column is right-aligned
and tabular by declaration, the header band comes from the token roles once,
an empty list renders the caller's empty state inside the frame — and an
eslint rule keeps every screen off bare `<table>`. 22 files use it.

What it does not do: sorting, filtering, pagination, column visibility,
virtualisation. So 41 sort/filter expressions are hand-written across
`src/routes/*.tsx`, each screen deciding for itself what a sorted column
means and none of them sharing a header affordance.

That is the migration: `@tanstack/react-table` goes *underneath* `DataTable`,
not in front of it. The `Column<Row>` interface — `id`, `header`, `money`,
`numeric`, `cell` — stays exactly as it is, so 22 screens do not change on
the day the row model changes. Sorting is opt-in per column
(`sortable: true`), and a screen that asks for nothing renders what it
renders today.

## Order of work

Each step is a PR on its own and `just gates` has to pass between them.

1. `@tanstack/react-table` in `apps/desktop`, `useReactTable` +
   `getCoreRowModel` inside `DataTable` only. Zero screens change, the
   snapshot of every list is identical. This step is the whole risk: if the
   rendered DOM moves, stop here.
2. `sortable?: true` on `Column`, `getSortedRowModel`, a header button with
   the aria-sort the accessibility check wants. Turn it on for one list
   (documents) and leave the other 21 alone.
3. Move the screens that already hand-roll a sort onto it, one PR per screen,
   deleting the local state as each lands. The 41-expression count is the
   number to watch come down.
4. Filtering, only where a screen has a search box today. Server-side where
   the list is a query (`/products?q=`), `getFilteredRowModel` only for a
   list already fully in memory.
5. Pagination last, and only if a real shop's list is long enough to need it.
   A shop with 300 products does not.

## The phone, separately

Once the desktop is done, `apps/mobile`'s `pair`, `sign-in` and the till's
tendered box move onto `@tanstack/react-form`. Today they are `useState` with
a hand-written `disabled={token.trim() === ""}`. The win is not the state —
it is sharing one validation statement with the desktop rather than two.

`@tanstack/react-table` has no place on the phone: the till's product list is
a `FlatList` of cards, not a grid.

## What this plan deliberately does not do

- No TanStack Router on the phone. expo-router is the React Native answer and
  the rewrite already landed on it.
- No `useDeferredValue` anywhere yet. Product search should be a server query
  with `placeholderData: keepPreviousData`, which is what stops a list
  flashing; deferring a client-side filter is the answer to a problem we do
  not have.
- No replacement of `DataTable`'s public shape. If a screen has to change,
  the step was wrong.
