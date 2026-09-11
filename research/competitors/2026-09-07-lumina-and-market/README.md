# Algerian inventory/POS product — competitor research

New product line, separate from ObserveOne. Anouar asked for an inventory
management product for the Algerian market, built against an incumbent that is
selling well. Working codename **dz-pos** — placeholder, not the product name.
Code: [Dinar-dz/dz-pos](https://github.com/Dinar-dz/dz-pos) (private,
scaffolded 2026-09-07 from MonStock `33d2ab0`). Decided 2026-09-08: the org
was created and the repo transferred into it (Anouar, `context/progress/now.md`);
the product name itself is still open, see below.

Started 2026-09-07.

## Documents

- [`brief-for-anouar.md`](brief-for-anouar.md) — the one-page version for Anouar:
  what Lumina is, the market, what we have, decisions made, and the six
  questions we need answered.

- [`lumina-pos-teardown.md`](lumina-pos-teardown.md) — full feature map of
  **Lumina POS** (FikraDevs, Oum El Bouaghi). Built from the vendor's own screen
  recording plus static analysis of the shipped Android APK and the unpacked
  Windows installer. Includes the complete Algerian facture field list and their
  droit-de-timbre implementation.
- [`monstock-gap-analysis.md`](monstock-gap-analysis.md) — what
  [MonStock](https://github.com/samir1498/MonStock) already has, what it is
  missing against Lumina, and the order to close the gap in.
- [`architecture-notes.md`](architecture-notes.md) — stack, the six rules that
  make SaaS-vs-offline a deployment choice, local/cloud connection, the parked
  phone-only-offline path (uniffi, not a TS rewrite), build order, and the
  testing gaps a JS-only list misses.

## Decisions taken so far

- ~~Extend MonStock rather than greenfield.~~ Reversed 2026-09-07 after
  Anouar's answer: start from scratch, document features and architecture
  first, tests from the first commit. MonStock is a read-only reference.
  The repo keeps the tooling scaffold only; spec lives in its `docs/`.
- Desktop on Tauri, not egui — RTL and HTML invoice rendering are both far
  cheaper there. Rust core is the reason Tauri beats Wails/Go.
- Mobile on Expo / React Native, developed against a real phone over Tailscale,
  built via EAS.
- The phone is a full client, not a companion, so `monstock-core` sits behind
  one HTTP service layer from day one. Tauri commands and the mobile app are
  equal callers. Whether that server is local or hosted then becomes a
  deployment choice rather than a rewrite.
- LAN discovery via mDNS (`mdns-sd`) with QR pairing — no IP address ever typed.
- SQLite everywhere, one file per shop if cloud happens — never a second DB
  backend. Money in integer centimes. Phone-only-offline parked; if it ever
  happens it's the Rust core via uniffi inside the RN app, never a TS rewrite.
- Drop `monstock-desktop` (egui); gut `monstock-cli` to `seed`.

## Open, waiting on Anouar

- SaaS/cloud vs offline-first licence, and the pricing model that follows.
  Lumina is 12,000 DZD one-time, lifetime, sold over the phone. Still open:
  `docs/architecture.md` parks the SaaS-vs-offline choice as "open decision 1,
  after M6" (checked 2026-09-11).
- Product name and whether a new GitHub org gets created. Decided 2026-09-08:
  the org question is closed (`context/progress/now.md`). The name is not:
  Anouar proposed "Dinar" the same day, but the repo is still
  placeholder-named (`CLAUDE.md`, "Placeholder-named product").
- Who sells it and to whom. MonStock shipped five releases and got no feedback;
  Lumina's moat is a phone number in Oum El Bouaghi, not its code. Still open.

## Not yet researched

See [`market-landscape.md`](market-landscape.md) — a dozen Algerian players
surfaced during the name check (Fatoura is distributed by Algérie Télécom),
plus the name-availability table. Daftar/Hanout/Mizan are all taken.
