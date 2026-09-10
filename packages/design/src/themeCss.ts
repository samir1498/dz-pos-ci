// The one stylesheet the desktop loads on top of Tailwind: the token custom
// properties for both themes, the shadcn/ui variable set mapped onto them,
// and the Tailwind v4 `@theme` blocks that turn the pair into utilities.
//
// Generated, never edited: `just theme` writes apps/desktop/src/theme.css and
// `themeCss.test.ts` fails the gates when the checked-in file and this source
// disagree, the way `just types-check` does for the generated DTOs.
//
// Three things worth knowing before editing it.
//
// 1. The kit is shadcn/ui, so the emitted API is shadcn's names
//    (`--background`, `--primary`, `--sidebar-accent`). Our own roles stay
//    the source: `--background` is `var(--surface-bg)`, and the file a
//    designer edits is semantic.ts, not this one. A component installed by
//    the shadcn CLI in D3 then wears the Comptoir and Registre palettes
//    without a patch.
//
// 2. The shadcn block is emitted once, not once per theme, because every
//    name in it is an indirection: `[data-theme="registre"]` redefines
//    `--surface-bg` and `--background` follows on its own. That works only
//    while `data-theme` sits on the same element as `:root`, so it goes on
//    `<html>` and never on a wrapper div.
//
// 3. Tailwind reads `--color-*` as its colour namespace and `--text-*` as
//    its *font size* namespace, and our tokens use `--text-` for both a
//    colour (`--text-primary`) and a size (`--text-md`). Only the sizes go
//    into `@theme`. Names that already match a Tailwind key go through
//    `@theme reference`, which emits no variable of its own and so cannot
//    declare one pointing at itself; the shadcn names go through
//    `@theme inline`, which carries the value into the utility.

import { toCss } from "./css";
import { fontFamily, fontSize, radius, shadow } from "./semantic";

/** Where the generated file lands, from the repository root. */
export const THEME_CSS_PATH = "apps/desktop/src/theme.css";

/**
 * shadcn/ui's variable set, each pointing at the role that decides it. The
 * order follows shadcn's own so the block reads like the one its docs show.
 * `--money` and `--font-numeric` are ours: shadcn has no slot for a brass
 * accent that means "this action moves money", nor for a figure font.
 */
const SHADCN: readonly (readonly [string, string])[] = [
  ["--background", "--surface-bg"],
  ["--foreground", "--text-primary"],
  ["--card", "--surface-card"],
  ["--card-foreground", "--text-primary"],
  ["--popover", "--surface-card"],
  ["--popover-foreground", "--text-primary"],
  ["--primary", "--color-primary"],
  ["--primary-foreground", "--color-on-primary"],
  ["--secondary", "--surface-raised"],
  ["--secondary-foreground", "--text-primary"],
  ["--muted", "--surface-raised"],
  ["--muted-foreground", "--text-secondary"],
  ["--accent", "--surface-selected"],
  ["--accent-foreground", "--text-primary"],
  ["--destructive", "--color-danger"],
  // The same pair as the primary button: white on Comptoir's red 600, ink on
  // Registre's red 300. One role already says which way round the theme is.
  ["--destructive-foreground", "--color-on-primary"],
  ["--border", "--border-default"],
  ["--input", "--border-strong"],
  ["--ring", "--border-focus"],
  ["--radius", "--radius-md"],
  ["--sidebar", "--surface-sidebar"],
  ["--sidebar-foreground", "--text-on-sidebar"],
  ["--sidebar-primary", "--color-primary"],
  ["--sidebar-primary-foreground", "--color-on-primary"],
  ["--sidebar-accent", "--surface-sidebar-active"],
  ["--sidebar-accent-foreground", "--text-on-sidebar"],
  ["--sidebar-border", "--border-sidebar"],
  ["--sidebar-ring", "--border-focus"],
  ["--money", "--color-money"],
  ["--money-foreground", "--color-on-money"],
];

/**
 * Tailwind keys whose name this file already declares as a variable. They go
 * in `@theme reference`, so Tailwind builds `font-numeric` and `text-md`
 * without emitting a `--font-numeric: var(--font-numeric)` of its own.
 *
 * `--radius-*` is here rather than shadcn's `calc(var(--radius) - 4px)`
 * ladder: our scale is a designed scale, not four offsets from one number.
 */
const REFERENCE_GROUPS: readonly (readonly [string, readonly string[]])[] = [
  ["--font-", Object.keys(fontFamily)],
  ["--text-", Object.keys(fontSize)],
  ["--radius-", Object.keys(radius)],
  ["--shadow-", Object.keys(shadow)],
];

/**
 * The roles shadcn has no name for, kept under their own. `bg-warn`,
 * `text-fg-danger` and `bg-primary-soft` are how a screen reaches them.
 */
const EXTRA_UTILITIES: readonly (readonly [string, string])[] = [
  ["--color-primary-hover", "--color-primary-hover"],
  ["--color-primary-soft", "--color-primary-soft"],
  ["--color-money-hover", "--color-money-hover"],
  ["--color-danger-soft", "--color-danger-soft"],
  ["--color-warn", "--color-warn"],
  ["--color-warn-soft", "--color-warn-soft"],
  ["--color-info", "--color-info"],
  ["--color-info-soft", "--color-info-soft"],
  ["--color-success", "--color-success"],
  ["--color-hover", "--surface-hover"],
  ["--color-scrim", "--surface-scrim"],
  ["--color-inverse", "--surface-inverse"],
  ["--color-on-inverse", "--text-on-inverse"],
  ["--color-faint", "--text-tertiary"],
  ["--color-fg-disabled", "--text-disabled"],
  ["--color-fg-danger", "--text-danger"],
  ["--color-fg-success", "--text-success"],
  ["--color-line-strong", "--border-strong"],
  ["--color-line-danger", "--border-danger"],
];

const line = (name: string, value: string): string => `  ${name}: ${value};`;

const shadcnLines = (): string[] => SHADCN.map(([name, role]) => line(name, `var(${role})`));

const referenceLines = (): string[] =>
  REFERENCE_GROUPS.flatMap(([prefix, keys]) =>
    keys.map((key) => line(`${prefix}${key}`, `var(${prefix}${key})`)),
  ).concat(EXTRA_UTILITIES.filter(isIdentity).map(([name]) => line(name, `var(${name})`)));

/** Every shadcn name becomes a Tailwind colour key, except the two lengths. */
const NOT_A_COLOUR: ReadonlySet<string> = new Set(["--radius"]);

/**
 * An extra whose key already equals the variable it points at is a reference,
 * not an alias: put it in `@theme inline` and Tailwind emits
 * `--color-warn: var(--color-warn)` into its own layer, a declaration that
 * refers to itself. The unlayered token block still wins the cascade, so it
 * renders, but it is a cycle waiting for the day the layering moves.
 */
const isIdentity = ([name, role]: readonly [string, string]): boolean => name === role;

const inlineLines = (): string[] => [
  ...SHADCN.filter(([name]) => !NOT_A_COLOUR.has(name)).map(([name]) =>
    line(`--color-${name.slice(2)}`, `var(${name})`),
  ),
  ...EXTRA_UTILITIES.filter((entry) => !isIdentity(entry)).map(([name, role]) =>
    line(name, `var(${role})`),
  ),
];

const HEADER = `/* Generated from packages/design/src by \`just theme\`. Do not edit:
   packages/design/src/themeCss.test.ts fails the gates when this file and the
   token source disagree, the way just types-check does for the DTOs.

   Four parts: our tokens for Comptoir and Registre, the shadcn/ui variable
   set pointing at them, the Tailwind v4 theme map, and the document itself.

   The shadcn block is written once. Every name in it is an indirection, so
   [data-theme="registre"] redefining --surface-bg is enough for --background
   to follow. That holds only while data-theme is on the same element as
   :root, which is why the app sets it on <html>.

   The faces are not here: apps/desktop/src/styles.css imports them from the
   @fontsource packages, and nothing is fetched over the network. */`;

/**
 * The three declarations a themed app cannot start without. Everything else
 * a screen wears is a utility class; these are on the document itself, so a
 * theme switch repaints the page behind the app rather than a box inside it.
 */
const BASE = `/* ---- Base. The document, not a component. ---- */
html {
  font-family: var(--font-sans);
  background: var(--background);
  color: var(--foreground);
}`;

export const toThemeCss = (): string =>
  [
    HEADER,
    "",
    "/* ---- Tokens. The source is packages/design/src/semantic.ts. ---- */",
    toCss().trimEnd(),
    "",
    "/* ---- shadcn/ui, on our roles. ---- */",
    `:root {\n${shadcnLines().join("\n")}\n}`,
    "",
    "/* ---- Tailwind v4. ---- */",
    `@theme reference {\n${referenceLines().join("\n")}\n}`,
    "",
    `@theme inline {\n${inlineLines().join("\n")}\n}`,
    "",
    BASE,
    "",
  ].join("\n");
