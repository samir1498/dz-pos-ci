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
    status: 'done'
  - id: 'C3b'
    desc: 'Patient notes are doctor-only: a ViewPatientNotes permission for the owner, notes hidden and preserved for everyone else'
    status: 'done'
  - id: 'C5'
    desc: 'The appointment book: one doctor, a fixed slot length, a refusal when two patients take the same slot'
    status: 'done'
  - id: 'C5b'
    desc: 'Book tools for the desk: working hours, absence blocks that move what they hit, visit types with durations, next free slot, no-show mark, a printed day list with walk-ins'
    status: 'done'
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

## Samir, 2026-09-23 15:20: notes are doctor-only

"Of course doctor only, no one else needs that info." A third permission,
`ViewPatientNotes`, held by the owner (the doctor) only. Without it, the
patient DTO carries no `notes` field, and a write of the file keeps the
stored notes as they are, so a receptionist editing a phone number cannot
erase them, and a write that sends `notes` is refused with 403. The
server enforces it, not only the screen. Built as its own change after C5
(C3b below), before the screens.

Built and merged as C3b (PR 167). Open for Samir: a refused notes write
leaves no audit row; a receptionist probing the field leaves no trace.

Samir, 2026-09-23 15:59: the appointment's short reason for the visit is
not covered by that ruling; the receptionist books and reads it.

## Research before the rest (Samir, 2026-09-23 14:56)

The law comes before the book and the screens: a deep search on Algerian
law for a cabinet, logged in `context/research/` first, as was done for the
shop's tax rules. Three pages are being written: who may see the notes,
medical-practice law (dossier, ordonnance, certificat, software approval),
and health data plus fees (loi 18-07, hosting, timbre, CNAS). C4 finishes;
C5 onward waits for them.

### What the three pages found, and what it changes here

Pages: `context/research/20260923-who-sees-the-medical-notes-in-a-cabinet.md`,
`context/research/20260923-algerian-law-for-a-doctors-cabinet.md`,
`context/research/20260923-health-data-and-fees-for-a-doctors-cabinet.md`.

- Notes access: WinMed, Doctolib and the CNIL all keep clinical notes from
  the secretary. Recommended: a doctor-only permission for the notes field,
  before the screens (C6). Waits on Samir.
- A dossier médical per patient is a legal duty (loi 18-11 art. 26, 178),
  so the patient file is required, not optional.
- No clinical logic (dose calculators, diagnostic hints): loi 18-11 art. 212
  counts such software as a medical device needing ANPP registration.
- A remplaçant signs under his own name (art. 171): printed papers, when
  they come, need more than one practitioner identity per cabinet.
- Health data is sensitive (loi 18-07 art. 18); a relay hosted abroad needs
  prior ANPDP authorisation (loi 25-11 art. 45 bis 13). Local-first stays
  the default; the relay is a decision for later, with the lawyer.
- The droit de timbre applies to a médecin's fee note (Code du timbre art.
  100 as reformed in 2025), and a daily livre des recettes et dépenses is
  required (CIDTA art. 31 bis). Both belong to the fee slice, not this plan.
- Several article numbers come from secondary sources because the official
  PDFs were scans; each page marks which.

## Samir, 2026-09-23 16:00: no patient app, the doctor runs the book

No patient app. Appointments are managed by the doctor's desk, with basic
tools that plan ahead, catch problems and edge cases, and do the simple
calculations for the doctor. What those tools are comes from
`context/research/20260923-how-a-cabinet-runs-its-appointment-book.md`
(cabinet software and any Algerian rule on appointments), written before
the book grows past C5; the calendar screen stays a basic one.

### The book tools (C5b), from the research

`context/research/20260923-how-a-cabinet-runs-its-appointment-book.md`
compares WinMed, Doctolib and the others. The tools every product shares,
in order of value:

1. Weekly working hours: open and close ranges per weekday (a lunch break
   is two ranges), and a whole day closed. Booking outside them is refused.
2. Absence blocks: a period the doctor is away (a morning, a congress, a
   holiday). Creating one lists every appointment it hits and offers to
   move each to the next free slot or cancel it.
3. Visit types with their own length (first consultation, follow-up,
   certificate), replacing the single slot length as the booking's length;
   the grid stays the setting.
4. Next free slot: from a date and a visit type, the earliest start inside
   working hours, outside blocks, not overlapping; "see again in 15 days"
   is this search from today plus 15.
5. A no-show mark on a past appointment, kept on the row.
6. A printed day list holding both booked patients and the walk-in queue.

Holidays: the fixed-date ones can be offered as closed days; the Hijri ones
are confirmed only the evening before (loi 23-10), so the desk closes those
days by hand with an absence block. Friday is the legal rest day for the
private sector (loi 90-11 art. 33); Saturday is a working-hours choice.
Left out of the first version: fill-rate caps, waiting list for a freed
slot, per-patient no-show counts, SMS, recurring series, urgent slots (the
walk-in queue covers those).

Samir, 2026-09-23 16:11: the Algerian week. Sunday to Thursday are working
days, Friday and Saturday the weekend; the week view runs Sunday to
Saturday.

### What C5b settled when it was built (PR 168)

Defaults taken while building, each one Samir can overturn:

- A shop with no working hours set refuses nothing. A saved week needs at
  least one open range.
- Moving an appointment keeps its own length rather than taking the
  current slot setting.
- A no-show mark blocks moving or cancelling until it is cleared.
- Hours and visit types are written with EditSettings. Blocks, moves and
  no-shows are written with EditPatients. The day list and the next-free
  search need ViewPatients. Reading the hours, blocks and visit types needs
  nothing.
- Deleting a block or a visit type removes the row, and the audit log keeps
  the trace. Appointments copy a type's length rather than pointing at it.
- The day list returns data. Printing it belongs to C6.
