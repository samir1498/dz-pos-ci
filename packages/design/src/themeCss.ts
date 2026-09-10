// The one stylesheet the desktop loads: the vendored faces, the token
// custom properties for both themes, and the Tailwind v4 `@theme` map that
// turns those tokens into utility classes.
//
// Generated, never edited: `pnpm --filter @dzpos/design gen:theme` writes
// apps/desktop/src/theme.css and `themeCss.test.ts` fails the gates when the
// checked-in file and this source disagree, the way `just types-check` does
// for the generated DTOs.
//
// Why a map rather than the raw token names. Tailwind v4 reads two of its
// namespaces off names this package already uses for something else:
// `--color-*` is the colour namespace (so `--color-primary` lines up by
// itself) but `--text-*` is the *font size* namespace, and the tokens have
// both `--text-primary` (a colour) and `--text-md` (a size) under it. Left
// alone, `text-primary` would compile to `font-size: <a colour>`. So the
// colour roles that do not already sit in `--color-*` are aliased into it,
// and only the sizes keep `--text-*`.

import manifest from "../assets/fonts/manifest.json" with { type: "json" };
import { toCss } from "./css";
import { borderColor, color, fontFamily, fontSize, radius, shadow, surface, textColor } from "./semantic";

/** Where the generated file lands, from the repository root. */
export const THEME_CSS_PATH = "apps/desktop/src/theme.css";

/** From that file's own directory to the vendored woff2 files. */
const FONT_DIR = "../../../packages/design/assets/fonts";

interface FontFace {
  readonly file: string;
  readonly family: string;
  /** `400`, or `100 700` for a variable face. */
  readonly weight: string;
  readonly unicodeRange: string;
}

/**
 * Aliases from a token name to the Tailwind colour key. A pair whose two
 * halves are equal is an identity: the token already sits in Tailwind's
 * namespace and only has to be declared, not redefined.
 */
const SURFACE_KEYS: Readonly<Record<string, string>> = {
  bg: "bg",
  card: "card",
  raised: "raised",
  hover: "hover",
  selected: "selected",
  scrim: "scrim",
  inverse: "inverse",
};

/** `fg` rather than `primary`: `--color-primary` is the teal button. */
const TEXT_KEYS: Readonly<Record<string, string>> = {
  primary: "fg",
  secondary: "muted",
  tertiary: "faint",
  disabled: "fg-disabled",
  "on-inverse": "on-inverse",
  danger: "fg-danger",
  success: "fg-success",
};

const BORDER_KEYS: Readonly<Record<string, string>> = {
  default: "line",
  strong: "line-strong",
  focus: "focus",
  danger: "line-danger",
};

const alias = (map: Readonly<Record<string, string>>, key: string): string => {
  const found = map[key];
  if (found === undefined) {
    throw new Error(`no Tailwind name for ${key}; add it to themeCss.ts`);
  }
  return found;
};

const line = (name: string, value: string): string => `  ${name}: ${value};`;

/**
 * Keys whose token name already equals the Tailwind key. These go in a
 * `@theme reference` block: Tailwind builds the utilities and emits no
 * variable of its own, so `--color-primary: var(--color-primary)` never
 * becomes a declaration referring to itself.
 */
const referenceLines = (): string[] => [
  ...Object.keys(color).map((k) => line(`--color-${k}`, `var(--color-${k})`)),
  ...Object.keys(fontFamily).map((k) => line(`--font-${k}`, `var(--font-${k})`)),
  ...Object.keys(fontSize).map((k) => line(`--text-${k}`, `var(--text-${k})`)),
  ...Object.keys(radius).map((k) => line(`--radius-${k}`, `var(--radius-${k})`)),
  ...Object.keys(shadow).map((k) => line(`--shadow-${k}`, `var(--shadow-${k})`)),
];

/**
 * Keys that had to be renamed. These go in a `@theme inline` block, so the
 * utility carries the token variable itself (`background-color:
 * var(--surface-card)`) and no `--color-card` has to exist at runtime.
 */
const inlineLines = (): string[] => [
  ...Object.keys(surface).map((k) => line(`--color-${alias(SURFACE_KEYS, k)}`, `var(--surface-${k})`)),
  ...Object.keys(textColor).map((k) => line(`--color-${alias(TEXT_KEYS, k)}`, `var(--text-${k})`)),
  ...Object.keys(borderColor).map((k) => line(`--color-${alias(BORDER_KEYS, k)}`, `var(--border-${k})`)),
];

const fontFaceBlock = (face: FontFace): string =>
  [
    "@font-face {",
    `  font-family: "${face.family}";`,
    "  font-style: normal;",
    `  font-weight: ${face.weight};`,
    "  font-display: swap;",
    `  src: url("${FONT_DIR}/${face.file}") format("woff2");`,
    `  unicode-range: ${face.unicodeRange};`,
    "}",
  ].join("\n");

export const fontFaces = (): readonly FontFace[] => manifest;

const HEADER = `/* Generated from packages/design/src by \`pnpm --filter @dzpos/design gen:theme\`.
   Do not edit: packages/design/src/themeCss.test.ts fails the gates when this
   file and the token source disagree, the way just types-check does for the
   generated DTOs.

   Three parts: the vendored faces (no font is fetched over the network, the
   shop counter has none), the token custom properties for Comptoir and
   Registre, and the Tailwind v4 theme map. */`;

/**
 * The three declarations a themed app cannot start without. Everything else
 * a screen wears is a utility class; these are on the document itself, so a
 * theme switch repaints the page behind the app rather than a box inside it.
 */
const BASE = `/* ---- Base. The document, not a component. ---- */
html {
  font-family: var(--font-sans);
  background: var(--surface-bg);
  color: var(--text-primary);
}`;

export const toThemeCss = (): string =>
  [
    HEADER,
    "",
    "/* ---- Faces. Vendored under packages/design/assets/fonts. ---- */",
    ...fontFaces().map(fontFaceBlock),
    "",
    "/* ---- Tokens. The source is packages/design/src/semantic.ts. ---- */",
    toCss().trimEnd(),
    "",
    `/* ---- Tailwind v4. \`reference\` declares the keys whose variable this
   file already carries; \`inline\` carries the value into the utility for the
   roles Tailwind's namespaces have no room for under their token name. ---- */`,
    `@theme reference {\n${referenceLines().join("\n")}\n}`,
    "",
    `@theme inline {\n${inlineLines().join("\n")}\n}`,
    "",
    BASE,
    "",
  ].join("\n");
