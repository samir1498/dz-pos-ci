# Vendored faces

The app never fetches a font. A shop counter has no internet, and a face
pulled from a CDN looks right on the machine that built it and falls back to
`system-ui` on the one that matters. These eight files are what
`apps/desktop/src/theme.css` declares, and `themeCss.test.ts` fails the gates
if that file ever names an external host.

| Family | Files | Weights | Licence |
| --- | --- | --- | --- |
| IBM Plex Sans | `ibm-plex-sans-latin.woff2`, `ibm-plex-sans-latin-ext.woff2` | 100–700, one variable file per subset | OFL 1.1, `IBM-Plex-OFL.txt` |
| IBM Plex Sans Arabic | `ibm-plex-sans-arabic-{400,500,600,700}-arabic.woff2` | four static weights | OFL 1.1, `IBM-Plex-OFL.txt` |
| JetBrains Mono | `jetbrains-mono-latin.woff2`, `jetbrains-mono-latin-ext.woff2` | 100–800, one variable file per subset | OFL 1.1, `JetBrains-Mono-OFL.txt` |

303 kB over the eight files, of which 178 kB is the Arabic face: it has no
variable build on Google Fonts, so its four weights are four files. Latin is
75 kB for both text families together because both are variable.

`manifest.json` carries the source URL, the weight axis, the subset and the
`unicode-range` of every file. `packages/design/src/themeCss.ts` reads it and
writes the `@font-face` rules, so a face added here reaches the app by
regenerating (`just theme`) and never by hand.

## Where they came from

`https://fonts.googleapis.com/css2?family=<family>:wght@<axis>&display=swap`,
requested with a desktop Chrome user agent so the endpoint answers with woff2
rather than ttf, then every `@font-face` block it returned for the subsets we
keep was downloaded to the URL it named. The endpoint already serves one file
per script, which is the subsetting `pyftsubset` would have done by hand;
fonttools is not installed on the box, and the ranges Google cuts are the
ranges the generated CSS repeats verbatim, so a browser downloads the Arabic
face only on an Arabic screen.

Arabic keeps only the `arabic` subset. The RTL font stack is
`"IBM Plex Sans Arabic", "IBM Plex Sans", system-ui`, so a Latin glyph on an
Arabic screen falls through to Plex Sans, which is the right face for it and
already loaded.
