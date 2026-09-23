---
title: 'The first clinic module: patients, a waiting queue, an appointment book'
slug: 'the-first-clinic-module-patients-queue-appointments'
status: 'active'
category: 'feature'
created: 20260923
tldr: 'Phase B of the restructure loop. A crates/clinic beside crates/retail, switched on by a clinic cargo feature, holding three things in order: the patient file, a waiting-room queue in arrival order, and a plain appointment book for one doctor. No money, no printed paper, no fund as payer. Ids unique across machines from the first migration. Its job is to prove a second trade plugs in without touching the shop; the plug-point traits are drawn after it, from two real modules.'
priority: 80
tasks:
  - id: 'C1'
    desc: 'This page: the slices, the defaults taken, and what is left out'
    status: 'done'
  - id: 'C2'
    desc: 'crates/clinic exists behind a clinic feature; a clinic-only build (no retail) compiles and signs a user in'
    status: 'done'
  - id: 'C3'
    desc: 'The patient file: table, service, audit actions, two permissions, api routes and gate rows behind the feature'
    status: 'done'
  - id: 'C4'
    desc: 'The waiting queue: add a patient, call the next, mark seen, one day at a time'
    status: 'pending'
  - id: 'C5'
    desc: 'The appointment book: one doctor, a fixed slot length, a refusal when two patients take the same slot'
    status: 'pending'
  - id: 'C6'
    desc: 'Desktop screens for C3 to C5 behind a build switch, with the calendar component chosen before the book screen'
    status: 'pending'
  - id: 'C7'
    desc: 'The shop screens on desktop and phone behind the same build switch'
    status: 'pending'
---
# The first clinic module: patients, a waiting queue, an appointment book

The split plan (`context/plans/20260922-a-kernel-crate-and-retail-as-the-first-module.md`)
turns the shop into one module on a kernel. This page is the second module,
kept as small as a second module can be while still proving the claim: a
trade plugs in without a line of the shop changing. Scope comes from
`context/research/20260923-modules-sync-and-the-clinic-scope.md` and the
two clinic research pages it cites.

## What the module holds

The patient file comes first because the queue and the book both point at a
patient. A cabinet's patient file is identity plus notes, with no debt shape
(`context/research/20260922-what-clinic-software-provides.md`), so it is the
clinic's own table and not a differently typed `customers` row: the paper
test found the identity columns would fit and the money columns would not.

The queue is the shape many cabinets actually run
(`context/research/20260922-a-doctors-cabinet-day-on-paper.md`): patients
in arrival order, called one by one, marked seen. Almost no rules.

The book is one doctor, one slot length set in settings, day and week views,
and a refusal when a second patient takes a booked slot, enforced in the
service and by a unique index so a race cannot double-book.

## Defaults taken, each one Samir can overturn

- **Ids.** Clinic tables use a text id that stays unique across machines
  (a UUID v7, which also sorts by creation time), from the first migration.
  The cloud relay and the patient app need it; retail keeps its integer ids
  until branch sync, which is its own migration. The alternative, an integer
  plus a machine prefix, is smaller on disk and harder to get right when a
  machine is restored from a backup.
- **Migrations.** One shared ordered list, per D5 of
  `context/plans/20260921-whether-dinar-becomes-a-core-and-modules.md`: a
  shop install carries the clinic tables empty, and a clinic install the
  shop's.
- **Permissions.** The permission enum stays one list, per Samir's ruling of
  2026-09-21, so the clinic adds `ViewPatients` and `EditPatients` to it. The
  queue and the book reuse them rather than adding more in the first
  version.
- **The tenant.** The kernel's `shops` table and `shop_id` column are the
  tenant, whatever the trade. Renaming them is a schema change across every
  table and is not in this plan.

## What is left out

No money: no fee, no receipt, no fund as payer. Whether the droit de timbre
applies to a medical fee note is unsourced, and the paper test found the
fund that owes a cabinet has no table anywhere. No printed paper: no
ordonnance, no certificat. No recurrence, no reminders, no multi-doctor
rota. No cloud relay and no patient app. Each is its own plan when it comes.

## The slices

**C2: the crate, empty.** `crates/clinic` depending on the kernel, a
`clinic` feature on `crates/core` and `crates/api` beside `retail`, and a
`just check-clinic-only` in gates that builds and tests
`dzpos-api --no-default-features --features clinic`. Size S.

**C3: the patient file.** Migration (patients: id, shop_id, names, sex,
date of birth, phone, free notes, archived_at, created and updated stamps),
repo, service (create, update, search by name or phone, archive), audit
actions, the two permissions, routes and gate rows under
`#[cfg(feature = "clinic")]`, DTOs with `just types`. Tests in
`crates/clinic/tests`. Size M.

**C4: the queue.** `queue_entries` (id, shop_id, patient_id, day,
arrived_at, called_at, seen_at, left_at), a service that refuses calling a
patient twice or marking seen before called, routes. Size S.

**C5: the book.** `appointments` (id, shop_id, patient_id, starts_at,
slot_minutes, status, cancelled_at), the slot length in settings, the
double-booking refusal with a unique index on (shop_id, starts_at) for live
rows, day and week reads. Size M.

**C6: the screens.** A Vite build flag decides which modules' routes exist
in the desktop bundle. Patients list and fiche, the queue, the book. Before
the book screen: a build-versus-buy check of Schedule-X, FullCalendar (MIT
core) and react-big-calendar against the desktop stack, bundle size and
licence, written into this page. Size L.

**C7: the shop's screens behind the same flag,** desktop and phone, so a
clinic build ships no till. The phone's pair, sign-in and settings screens
are shared. Size M.

## Done when

A build with `--features clinic` and no retail installs, creates a patient,
queues and books them, and shows no shop screen or route; the default shop
build is unchanged, proved by `just gates`.

## Open for Samir

Every role holds `ViewPatients` and `EditPatients`, so a receptionist reads
the whole file including the free-text notes, which may carry medical
history. A doctor-only notes tier is a third permission; say so before the
queue and book screens are built on this one (PR #164).
