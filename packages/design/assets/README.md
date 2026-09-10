# The mark

One coin for two scripts. The bowl opens toward the reading side; the coin
edge closes it into a Latin D, the foot on the baseline makes it an Arabic
dal. The mark never changes. The word beside it follows the language, which
is what `apps/desktop/src/components/Wordmark.tsx` does with the language
switch: fr and en get the Latin word, ar gets the Arabic one, and the coin
stays where it is.

| File | For |
| --- | --- |
| `mark.svg` | The mark at 96 px and down to about 32 px: brass coin, ink stroke, milled edge. |
| `mark-mono.svg` | One colour, `currentColor`. Anywhere brass cannot go. |
| `favicon.svg` | The 24 px variant. The milled edge is dropped and the stroke thickened, because at that size the ring closes up. |
| `ticket-header.svg` | Pure black outline for the 80 mm roll and the A4 facture footer. A thermal head prints one colour and no halftone. |
| `wordmark-latin.svg`, `wordmark-arabic.svg` | The word on its own, both set at the same cap height and weight with the second word in brass, so the two lockups share a silhouette. |
| `lockup-light.svg`, `lockup-dark.svg` | Mark and Latin word on the Comptoir and Registre grounds. |

## The wordmarks carry live text

The four files with a word in them use `<text>`, not outlines. Converting
type to paths needs the font binaries and a tool that reads them, and the box
that drew these has neither; `pyftsubset` and the rest of fonttools are not
installed. So a renderer without IBM Plex Sans or IBM Plex Sans Arabic
substitutes its own sans-serif and the silhouette moves.

That is safe where these files are actually used and worth knowing before
they go anywhere else:

- The app does not read them. It sets the same two words in HTML with the
  token font, which is vendored beside this folder, so the desktop always
  gets the real face.
- A print house, a slide, or anything rendering the SVG on a machine without
  Plex needs the outlined version. Make it with `pyftsubset` and a path
  conversion once fonttools is on the box, and commit it as
  `wordmark-latin-outlined.svg` beside the live one rather than over it.

## Colours

Brass `#b8862b`, ink `#14211c`, paper `#f4f1e8`, stone `#f8f7f4`. They are
literal here rather than token references on purpose: an SVG opened by a
print house, a browser or a design tool has no stylesheet behind it. The
tokens carry the same values (`--p-brass-500`, `--p-ink-800`, `--p-paper-50`,
`--p-stone-50`) and `packages/design/src/primitives.ts` is where they change.
