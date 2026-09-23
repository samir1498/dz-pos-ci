---
title: 'Modules, cloud sync and how small the first clinic module is'
date: 2026-09-23
status: 'draft'
tldr: 'Notes from a conversation with Samir on 2026-09-23. Each trade is a
  crate and each customer package is a cargo feature list. Full ports and
  adapters is too much for an offline app; ports go at the plug points
  only. Lumina-style cloud sync and a patient booking app are the same
  missing piece, a cloud relay, and belong in a later module of their own.
  The first clinic module stays small: patients, a waiting-room queue, a
  plain appointment book, with an open-source calendar component rather
  than a hand-built one.'
---
# Modules, cloud sync and how small the first clinic module is

Samir raised these on 2026-09-23 while the crate split was running. Nothing
here is built yet. The split plan
(`context/plans/20260922-a-kernel-crate-and-retail-as-the-first-module.md`)
records the one ruling among them; the rest is direction for the plans that
come after it.

## How a trade becomes a package

Each trade is a crate: `crates/kernel` is what every trade shares,
`crates/retail` is the shop, and a `crates/clinic` will be the doctor. Cargo
refuses an import across a wall that is not declared, so the shop cannot
reach into the clinic and the kernel cannot reach into either.

A cargo feature decides at build time whether a crate is compiled in at
all. It looks like a feature flag but works on whole modules: a build
without `retail` has no shop code in it to switch back on. One repo gives
several installers: a shop's is built with `retail`, a doctor's with
`clinic`, a clinic that also sells products with both. This is Anouar's
"we prepare the package for him and send it" (2026-09-22), and why nothing
is loaded at runtime.

## Ports and adapters: only at the plug points

Ruled by Samir, 2026-09-23 09:20 (recorded on the split plan). No trait in
front of every repo and no dependency injection through every service: the
app is offline SQLite with no second database to swap in, and its tests
already run against a real file. Ports go where a module tells the shared
part what it brings: its permissions, routes, migrations, audit actions and
what backup counts. The first of these already exists: backup verification
and the support bundle take the shop's counts from their caller since the
kernel cleanup (PR #158). The rest are drawn after the clinic module exists,
so each trait comes from two real modules. The crates are the DDD-lite
bounded contexts; grouping each crate's files by domain (the large DTO file
first) is a follow-up after the split.

## Cloud: one relay for two needs

Lumina's connection modes are local network, cloud and hybrid; its cloud
syncs data between a business's branches while each till keeps working on
its own local data. That is local-first plus a sync layer, not a hosted
database, so it does not change the ruling above.

A patient app is the other need. A patient booking from home is on the
internet, while the cabinet's PC is offline on its own network, so a cloud
service has to hold bookings and sync them down. That is the same piece
branch sync needs: a relay server plus an outbox of changes each site
pushes and pulls. Build it once, as its own module and its own port, and
both uses share it.

What it costs when it comes: uptime, accounts, and personal-data duties
for patient records held on a server, none of which an offline app carries
today. Whether to host anything at all is a decision for then.

The one thing worth doing before it: rows use auto-increment ids, so two
sites would both create patient 42 and collide on sync. New clinic tables
(patients, appointments) should use ids that stay unique across machines
from their first migration. Retail's tables would need the same change
before branch sync, which is a migration of its own.

### The doctor's own PC as the server: a tunnel

Samir, 2026-09-23 10:34: could the doctor host it himself on a connection
with no stable address? Yes, with a tunnel rather than DHCP or port
forwarding. `cloudflared` (Cloudflare Tunnel, free) opens an outbound
connection, so a changing IP and carrier NAT on 4G do not matter, and the
cabinet gets a fixed name such as `cabinet-x.dinar.app`; Dinar could ship
it beside the app and set it up at install. Tailscale Funnel does the same
tied to a Tailscale account; frp is open source but needs a relay server of
our own; dynamic DNS (DuckDNS, No-IP) is cheapest but fails behind carrier
NAT and needs a forwarded port. The catch: bookings only work while that PC
is on and online, so power cuts and a PC off at night close the booking
page. A small cloud-held booking inbox the PC syncs from avoids that, which
is the relay above at the cost of one small server. The tunnel is the cheap
first version; the relay is the robust one.

## The first clinic module, kept small

A full clinic is a large product: recurring appointments, overlap rules,
working hours, public holidays including the ones that move with the Hijri
calendar, Ramadan hours, reminders, no-shows. The first module's job is to
prove a second trade plugs in without touching the shop, not to match
clinic software. In order:

1. The patient file. Both the queue and the book point at a patient, so it
   comes first. The clinic research found that a cabinet's patient file is
   a medical history with no debt in it, unlike a shop's customer file.
2. A waiting-room queue in arrival order. The cabinet-day research found
   that many cabinets work this way, and it has almost no rules: add a
   patient, call the next, mark them seen.
3. A plain appointment book: one doctor, a fixed slot length, day and week
   views, a refusal when two patients get the same slot. No recurrence, no
   reminders, no multi-doctor rota in the first version.

Build versus buy for the calendar screen: open-source components already
solve it, and one should be chosen against the desktop stack before that
slice starts. Candidates: FullCalendar (its core is MIT), Schedule-X,
react-big-calendar. If recurrence comes later, the `rrule` libraries (a
Rust crate and a JS one) implement the iCalendar repeat rules rather than a
hand-written one.

## Sources

- Samir, conversation of 2026-09-23 between 09:17 and 10:30.
- `context/research/20260922-what-clinic-software-provides.md`, the
  features clinic software sells and the appointment book worked example.
- `context/research/20260922-a-doctors-cabinet-day-on-paper.md`, the
  walk-in queue and the appointment book as the two common shapes.
- The Lumina teardown notes in `context/plans/20260917-architecture-fixes-without-a-domain-split.md`
  and the 2026-09-20 loop, for its local, cloud and hybrid modes.
