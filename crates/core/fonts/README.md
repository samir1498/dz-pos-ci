# The two faces the raster ticket draws with

A cheap thermal head has no single-byte table for Arabic, so the Arabic
ticket is drawn into a bitmap instead of sent as text
(`crates/core/src/print/raster.rs`,
`context/plans/20260921-arabic-on-a-cheap-thermal-head.md`). Drawing needs
outlines, and outlines have to come from somewhere the build owns: both
files below are vendored here and loaded with `include_bytes!`, the way the
desktop vendors Plex through `@fontsource` rather than trusting whatever the
shop's Windows PC happens to have installed. Nothing here reads a system
font, so the paper looks the same on every machine.

Both are SIL Open Font License 1.1, whose full text sits beside each file.
Neither is modified: these are the bytes the release asset shipped.

| File | Release | Tag | Bytes | SHA-256 (first 16) |
|---|---|---|---|---|
| `NotoNaskhArabic-Regular.ttf` | [notofonts/arabic](https://github.com/notofonts/arabic/releases) | `NotoNaskhArabic-v2.021` | 158 580 | `c34fdbd98af4dbc4` |
| `IBMPlexMono-Regular.ttf` | [IBM/plex](https://github.com/IBM/plex/releases) | `@ibm/plex-mono@2.5.0` | 173 052 | `7c6fbddca4b700be` |

- Noto Naskh Arabic came from `NotoNaskhArabic-v2.021.zip`, path
  `NotoNaskhArabic/unhinted/ttf/NotoNaskhArabic-Regular.ttf`. Unhinted
  because `ab_glyph` rasterises outlines and ignores TrueType hinting
  entirely, so the hinted build (247 336 bytes) would be 89 KB of
  instructions nothing in this binary reads. Licence:
  `NotoNaskhArabic-OFL.txt` (4 382 bytes), the release's own `OFL.txt`.
- IBM Plex Mono came from `ibm-plex-mono.zip`, path
  `ibm-plex-mono/fonts/complete/ttf/IBMPlexMono-Regular.ttf` — the complete
  face, not one of the `split/` subsets, because a ticket carries French
  accents, a `×` and a `%` and the subsets divide those across files.
  Licence: `IBMPlexMono-OFL.txt` (4 456 bytes), the package's own
  `LICENSE.txt`.

Plex Mono is what fixes the column budget: every glyph in it advances
600/1000 of an em, so 42 of them are the head's width by construction and
the raster has the same 42 columns the text path counts in
(`escpos.rs`'s `WIDTH`). Naskh is drawn at the same em size beside it.
