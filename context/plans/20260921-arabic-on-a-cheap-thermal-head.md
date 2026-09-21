---
title: 'Arabic on a cheap thermal head'
slug: 'arabic-on-a-cheap-thermal-head'
status: 'active'
category: 'feature'
created: 20260921
tldr: 'A cheap 80 mm head has no single-byte table for Arabic, so the ticket in Arabic prints boxes today. The fix is to stop sending text: draw the same ticket lines into a 1-bit bitmap and send it as a raster, which every head with a raster command prints with no codepage at all. This page is the build-or-buy call for the three commodity jobs (bidi, shaping, glyph rasterising), the font that gets vendored, and the four tasks carried from the closing-gaps loop.'
priority: 80
roadmap: 'catching-lumina'
tasks:
  - id: 'T1'
    desc: 'The page, with the build-or-buy call: which crates, which font, what it costs the build, and that it compiles for the Windows release target'
    status: 'done'
  - id: 'T2'
    desc: 'The raster writer and its goldens: print::raster draws the ticket lines into a bitmap, escpos.rs emits GS v 0 bands, and the dump writes PNG goldens a reviewer opens'
    status: 'done'
  - id: 'T3'
    desc: 'When the raster is used: a thermal_mode preference beside print_lang, Arabic always rasters, the route and the printing panel read it, the phone inherits it through the spool'
    status: 'done'
  - id: 'T4'
    desc: 'The 80 mm facture down the ESC/POS path, re-homed from the archived layouts plan, with a line model drawn from facture_view.rs and every field §4 requires'
    status: 'done'
---
# Arabic on a cheap thermal head

Phase 3 of `context/loops/20260920-closing-the-lumina-gaps-loop.md`, carried
whole into `20260921-a-clean-context-then-the-till-finished.md`. Ruling 8
there sets the head width: 576 dots is the default and a preference, and the
work does not wait on it.

## What exists and why it prints boxes

`crates/core/src/print/escpos.rs` (371 lines) selects codepage 19 with
`ESC t 19`, which is PC858 / ISO 8859-15, and sends every character outside
that table as UTF-8. French fits in one byte per column. Arabic does not fit
in any single-byte table a cheap head ships with, and a head handed UTF-8 it
cannot map prints a box per byte. The spec's own thermal bullet
(`docs/features.md` §4) says as much: "a cheap head will need a different
codepage". There is no codepage to switch to that helps, because the problem
is not the table but that Arabic letters change shape with their neighbours
and run right to left, and a text-mode head does neither.

What makes a fix cheap is that the ticket already exists as a **line model**.
`print/ticket.rs` builds the lines and the text path merely encodes them. The
same lines drawn into a 1-bit bitmap and sent with `GS v 0` print on any head
that has a raster command, which is nearly all of them, and need no codepage
at all. The numbers stay the same numbers: T2's test asserts the raster's
source lines equal, string for string, the lines the text path emits, so a
total cannot differ between the two paths.

## The three commodity jobs, and what was looked at (2026-09-21)

Drawing Arabic needs three things done right, none of which we should write:
the bidi algorithm (which runs of a line go right to left), shaping (which
glyph each letter takes given its neighbours, and the ligatures), and glyph
rasterising (turning outlines into pixels at a given size). The global rule
says look before hand-rolling, so this is what is on crates.io, checked on
2026-09-21 against the registry's own API:

| Crate | Version, date | Downloads | Licence | Does |
|---|---|---|---|---|
| `unicode-bidi` | 0.3.18, 2024-12 | 507 M | MIT / Apache-2.0 | the bidi algorithm; **already a dev-dependency here**, used by the print tests |
| `rustybuzz` | 0.20.1, 2024-11 | 31 M | MIT | HarfBuzz shaping ported to Rust, complete for Arabic, no C |
| `ab_glyph` | 0.2.32, 2025-09 | 43 M | Apache-2.0 | load a TTF from bytes, scale, position and rasterise glyphs |
| `cosmic-text` | 0.19.0, 2026-04 | 8.8 M | MIT / Apache-2.0 | all three in one, plus multi-line layout and a font database |
| `swash` | 0.2.10, 2026-07 | 13 M | Apache-2.0 / MIT | shaping and rendering, what cosmic-text renders with |
| `fontdb` | 0.24.0, 2026-07 | 34 M | MIT | in-memory font database with system font lookup, what cosmic-text finds fonts with |
| `escpos` | 0.20.0, 2026-09 | 72 k | MIT | an ESC/POS driver with raster image support |

**The choice: `rustybuzz` + `ab_glyph`, with `unicode-bidi` promoted from a
dev-dependency to a runtime one.** Three focused crates, each doing one of
the three jobs, all pure Rust, none reaching for system fonts, and the font
comes in with `include_bytes!` the way a golden fixture does.

Why not `cosmic-text`: it is the right answer for an editor and the wrong
size for a ticket. It brings `fontdb` (a system font database we would
configure to find nothing), `swash` (a second rasteriser), its own line
layout (which the ticket already has: fixed columns, one line per row), and a
font fallback chain we do not want, because a ticket that silently falls back
to another font has changed how a number looks. It is the heaviest build of
the candidates and most of what it builds sits unused.

Why not the `escpos` crate: `escpos.rs` is golden-filed in three languages
and nine fixtures, and the raster command it needs (`GS v 0` with its four
header bytes, then the bands) is about thirty lines beside it. Replacing a
proven module with a driver to gain thirty lines is the wrong trade, and the
loop already ruled it out. Its existence is worth knowing for one reason: its
image path proves the raster route is what everyone else does on cheap heads.

## The font

One Arabic face and one Latin face, both OFL 1.1, vendored under
`crates/core/fonts/` and loaded with `include_bytes!`, the way the desktop
vendors Plex through `@fontsource` rather than trusting the machine.

- **Noto Naskh Arabic Regular** for Arabic. Naskh is the printed newspaper
  style, the one a receipt is read in, and Noto's coverage of Arabic
  presentation forms and ligatures is complete. One weight, roughly 150 KB.
- **IBM Plex Mono Regular** for the Latin runs inside an Arabic ticket
  (product codes, the amounts, the date) and for French and English when T3's
  preference says raster. The screen already prints amounts in Plex Mono, so
  the paper and the screen agree on the shape of a digit. Roughly 100 KB.

Digits: Arabic tickets print **Western digits** (0-9), not Eastern
Arabic-Indic ones, which is what `print/strings.rs` does today and what an
Algerian receipt carries. So a number is a Latin run in an Arabic line, which
is exactly the case the bidi algorithm exists for and the reason
`unicode-bidi` becomes a runtime dependency.

## What it costs, to record before T2 starts

- **Build time, measured 2026-09-21** in T2's worktree with the three
  crates added (`rustybuzz =0.20.1`, `ab_glyph =0.2.32`, `unicode-bidi
  =0.3.18`, which pull in `ttf-parser`, `owned_ttf_parser`,
  `ab_glyph_rasterizer` and four small `unicode-*` crates):
  **`just clippy` cold 29.1 s, warm 28.1 s** (`time just clippy`, WSL box,
  shared build folder). Cold is the run that compiled the new crates from
  nothing with our three crates touched; warm is the same command straight
  after. Both numbers carry `desktop-dist`'s pnpm build, about 14 s of
  each; cargo's own part was 14.6 s cold against 12.1 s warm, so the new
  dependencies cost roughly **2.5 s** on a build that was rebuilding our
  crates anyway — far inside the minute this page said would be worth
  arguing about. The one number that looks bigger is the very first run in
  a fresh worktree (1 min 20 s), and that is the price of switching
  worktrees, not of these crates: `just claim` touches every source file
  when another checkout used the folder last.
- **Windows.** Nothing here links C, so the release target
  (`windows-latest` in `.github/workflows/release.yml`) needs no toolchain
  change. Nobody on this box can prove that: it is the mirror's Windows job
  on this branch, before merge, and not a claim T2 gets to make.
- **Binary size.** Two fonts, roughly 250 KB, plus the crates. Under a
  megabyte on a Tauri binary that already carries a webview.
- **Print time.** A 576-dot-wide raster of a 40-line ticket is about 576 × 1
  200 bits, 86 KB, sent in bands. Over USB or LAN that is well under a
  second; over a slow serial link it is a few seconds, which is the one case
  where text mode was faster and why T3 keeps text mode as the default for
  French and English.

## The tasks

**T1: this page.** Done 2026-09-21.

**T2: the raster writer and its goldens.** `print::raster` turns the
ticket's lines into a bitmap at the head's width and `escpos.rs` emits it in
`GS v 0` bands. The dump decodes `<raster WxH>` and writes the bitmap out as
PNG, so `fixtures/print/ticket_80mm_escpos/ar*.png` are goldens a reviewer
opens. Files: `crates/core/src/print/raster.rs`,
`crates/core/src/print/escpos.rs`, `crates/core/fonts/`,
`crates/core/tests/print_ticket_escpos.rs`, the fixtures. Spec: §4 Thermal
bullet, rewritten in this PR. Done: the test asserts the raster's source
lines equal, string for string, the lines the text path emits for the same
fixture, so a number cannot differ between the two; no line is clipped at
the width; the three Arabic goldens exist and the French and English bytes
are unchanged; the build-time line above is filled in. `dz-money-builder`:
printed totals become pixels here and the line-equality test is what keeps
them honest. Size L, the one task in the loop that may take a whole night.

Done 2026-09-21, merged as `373c149`. `print::raster` and `print::bidi`
(every Latin run inside an Arabic line is wrapped in an isolate before
the bidi algorithm, the same fix the HTML ticket took on 2026-09-12; the
first goldens printed `10,00-` and `% 19` without it), `print::png`, the
`GS v 0` band writer, fonts vendored, five Arabic pictures including
credit and credit-held. The isolate test goes red without the fix; the
IDAT payload and the band header are pinned by hand. Nothing routes a shop
to this path yet: that is T3.

**T3: when the raster is used.** A `thermal_mode` preference (`text` or
`raster`, default `text`) beside `print_lang`; Arabic always rasters,
because there is no single-byte path for it that a cheap head has; French
and English raster when the preference says so. The route reads it, the
printing panel shows it, the phone inherits it through the desktop's spool.
Files: `crates/core/src/services/preferences.rs`,
`crates/api/src/routes/sales.rs`, `crates/api/src/routes/settings.rs`,
`crates/api/src/gates/table.rs`, the panel on `settings.printing.tsx`,
`packages/shared/src/client/settings.ts`. Done:
`/sales/{id}/ticket/escpos?lang=ar` answers raster bytes whatever the
preference, `?lang=fr` answers text until the preference flips, and the
spool file name carries the mode. `dz-builder`. Size S.

**T4: the 80 mm facture down the ESC/POS path**, the archived layouts plan's
T7, re-homed. The raster sidesteps both reasons it was parked (bidi on a
roll, the ISO 8859-15 table). It needs a line model for the facture the way
`ticket.rs` has one, drawn from `facture_view.rs`, with every field §4
requires of a facture. Files: `crates/core/src/print/facture_roll.rs`,
`crates/api/src/routes/sales.rs`, goldens under
`fixtures/print/facture_roll_80mm_escpos/`. Spec: §4 facture field list and
the totals table; the same amounts-parsed-back rule the HTML goldens obey,
applied to the line model before it is drawn. `dz-money-builder`. Size M.

T3 and T4 merged together 2026-09-21 as `ca584b0` (PR 152): the preference,
the gated route, the panel, the escpos facture route and the roll's line
model with its goldens under `fixtures/print/facture_roll_80mm_escpos/`.
What the review added before the merge: an API test that stores each wire
and matches both escpos routes against the core's own render, a test that
reads each closing figure off the row its own label opens (the multiset
check alone let a TTC and net swap through), a unit test on the wrap branch
of `Items::row`, and the 404 on the facture escpos route for a missing or
another shop's document. Ruled and left: no audit row on the setter, like
the layout and language setters beside it. What no machine here can prove
stands as written below.

## What no machine here can prove

The goldens are bitmaps and the dump is a PNG, so a reviewer sees the shape
of the Arabic and can say whether a ligature is wrong. What nobody here can
see is paper: whether a given cheap head honours `GS v 0` at 576 dots,
whether its darkness setting makes Naskh legible at that size, and whether
the cut lands after the last band. That is the printed-ticket line on
`docs/release-checklist.md`, Samir's, with a real head in the room.
