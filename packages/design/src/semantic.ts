// Tier 2. Roles, and the scales that carry no role. This is the layer that
// gets edited when a colour changes meaning.
//
// A colour token keeps its primitive reference instead of a resolved hex, so
// src/css.ts can emit `var(--p-teal-600)` the way design/shared/tokens.css
// does, and src/theme.ts can resolve the same entry to a hex for React
// Native. One entry, two outputs, no second table to keep in step.

import type { PrimitiveFamily } from "./primitives";

export interface TokenRef {
  readonly kind: "ref";
  readonly family: PrimitiveFamily;
  readonly step: number;
}

/** A value the ramps cannot express: an alpha colour, a shadow, a font stack. */
export interface TokenLiteral {
  readonly kind: "literal";
  readonly value: string;
}

/** A length. A number in TypeScript, `px` once it reaches CSS. */
export interface TokenPx {
  readonly kind: "px";
  readonly value: number;
}

export type Token = TokenRef | TokenLiteral | TokenPx;

export type TokenGroup = Readonly<Record<string, Token>>;
export type PxGroup = Readonly<Record<string, TokenPx>>;

const ref = (family: PrimitiveFamily, step: number): TokenRef => ({ kind: "ref", family, step });
const literal = (value: string): TokenLiteral => ({ kind: "literal", value });
const px = (value: number): TokenPx => ({ kind: "px", value });

/**
 * The two themes. Comptoir is the light one and the default, so its groups
 * are the plain exports below and the ones `theme.ts` resolves; Registre
 * overrides every colour role, never a subset (see `themes` at the bottom of
 * this file and the parity test in `css.test.ts`).
 */
export type ThemeName = "comptoir" | "registre";

/** Emission order. Comptoir first: it is what `:root` carries. */
export const THEMES: readonly ThemeName[] = ["comptoir", "registre"];

export const color = {
  primary: ref("teal", 600),
  "primary-hover": ref("teal", 700),
  "primary-soft": ref("teal", 50),
  "on-primary": ref("stone", 0),
  // Brass. The one action that moves money wears it and nothing else does,
  // in either theme: a second brass button and the first one stops meaning
  // anything.
  money: ref("brass", 500),
  "money-hover": ref("brass", 700),
  "on-money": ref("ink", 800),
  danger: ref("red", 600),
  "danger-soft": ref("red", 50),
  warn: ref("amber", 500),
  "warn-soft": ref("amber", 50),
  info: ref("blue", 500),
  "info-soft": ref("blue", 50),
  success: ref("teal", 500),
} satisfies TokenGroup;

export const surface = {
  bg: ref("stone", 50),
  card: ref("stone", 0),
  raised: ref("stone", 100),
  hover: ref("stone", 100),
  selected: ref("teal", 50),
  scrim: literal("rgb(26 24 21 / 0.45)"),
  // Ink green, not stone: the sidebar is the one place Comptoir carries
  // Registre's face (the approved blend page), and the two themes then
  // share one chrome colour instead of two near-blacks.
  inverse: ref("ink", 800),
  // The chrome. Ink in both themes: the sidebar is the one surface the two
  // themes agree on, which is what lets a shop switch theme without the
  // shape of the app moving.
  sidebar: ref("ink", 800),
  "sidebar-active": ref("ink", 600),
} satisfies TokenGroup;

export const textColor = {
  primary: ref("stone", 900),
  secondary: ref("stone", 600),
  tertiary: ref("stone", 400),
  disabled: ref("stone", 300),
  "on-inverse": ref("paper", 50),
  "on-sidebar": ref("paper", 50),
  danger: ref("red", 600),
  success: ref("teal", 600),
} satisfies TokenGroup;

export const borderColor = {
  default: ref("stone", 200),
  strong: ref("stone", 300),
  focus: ref("teal", 500),
  danger: ref("red", 500),
  sidebar: ref("ink", 500),
} satisfies TokenGroup;

export const space = {
  1: px(4),
  2: px(8),
  3: px(12),
  4: px(16),
  5: px(20),
  6: px(24),
  8: px(32),
  10: px(40),
  12: px(48),
} satisfies PxGroup;

export const radius = {
  sm: px(6),
  md: px(10),
  lg: px(14),
  full: px(999),
} satisfies PxGroup;

export const shadow = {
  sm: literal("0 1px 2px rgb(26 24 21 / 0.06)"),
  md: literal("0 4px 12px rgb(26 24 21 / 0.08)"),
  lg: literal("0 12px 32px rgb(26 24 21 / 0.14)"),
} satisfies TokenGroup;

export const fontFamily = {
  sans: literal('"IBM Plex Sans", "IBM Plex Sans Arabic", system-ui, sans-serif'),
  mono: literal('"IBM Plex Mono", ui-monospace, monospace'),
  // Every amount, on screen and on paper, in both themes: a monospace with
  // tabular figures is what makes a column of totals line up on the digit
  // rather than on the glyph width.
  numeric: literal('"JetBrains Mono", ui-monospace, monospace'),
} satisfies TokenGroup;

export const fontSize = {
  xs: px(12),
  sm: px(13),
  md: px(15),
  lg: px(17),
  xl: px(20),
  "2xl": px(26),
  "3xl": px(34),
} satisfies PxGroup;

/** Touch target and control heights. Their CSS names carry no group prefix. */
export const layout = {
  "touch-min": px(44),
  "control-h": px(40),
  "control-h-lg": px(52),
} satisfies PxGroup;

/**
 * Arabic mirrors the layout and leads with the Arabic face. The desktop gets
 * this as a `[dir="rtl"]` override; mobile picks the family by locale, since
 * React Native has no cascade.
 */
export const rtl = {
  sans: literal('"IBM Plex Sans Arabic", "IBM Plex Sans", system-ui, sans-serif'),
} satisfies TokenGroup;

// ---- Registre, the dark theme ----
//
// Every role above, again. Nothing inherits: a role left out would fall
// through to the Comptoir value, which on an ink surface is a light colour
// on a light colour, and that miss only shows on the screen nobody opened.
// `css.test.ts` fails when the two key sets differ.
//
// The scales (space, radius, font, size, layout) are not part of the theme
// axis. A theme changes what a thing is made of, never how big it is.

export const registreColor = {
  // A dark theme inverts the button: the accent is the light half and the
  // ink is the text, so a teal 600 fill (4.3:1 under white) does not have to
  // be the thing a cashier hits all day.
  primary: ref("teal", 300),
  "primary-hover": ref("teal", 100),
  "primary-soft": ref("teal", 800),
  "on-primary": ref("ink", 800),
  money: ref("brass", 500),
  "money-hover": ref("brass", 300),
  "on-money": ref("ink", 800),
  danger: ref("red", 300),
  "danger-soft": ref("red", 900),
  warn: ref("amber", 300),
  "warn-soft": ref("amber", 900),
  info: ref("blue", 300),
  "info-soft": ref("blue", 900),
  success: ref("teal", 300),
} satisfies TokenGroup;

export const registreSurface = {
  bg: ref("ink", 800),
  card: ref("ink", 700),
  raised: ref("ink", 600),
  hover: ref("ink", 600),
  selected: ref("teal", 800),
  scrim: literal("rgb(0 0 0 / 0.6)"),
  inverse: ref("paper", 50),
  // The same ink as Comptoir. On this theme it matches the page behind it,
  // so the sidebar is told apart by its border rather than by its colour.
  sidebar: ref("ink", 800),
  "sidebar-active": ref("ink", 600),
} satisfies TokenGroup;

export const registreText = {
  primary: ref("paper", 50),
  secondary: ref("paper", 100),
  tertiary: ref("paper", 200),
  disabled: ref("ink", 200),
  "on-inverse": ref("ink", 800),
  "on-sidebar": ref("paper", 50),
  danger: ref("red", 300),
  success: ref("teal", 300),
} satisfies TokenGroup;

export const registreBorder = {
  default: ref("ink", 500),
  strong: ref("ink", 400),
  focus: ref("teal", 300),
  danger: ref("red", 300),
  sidebar: ref("ink", 500),
} satisfies TokenGroup;

/**
 * Black rather than the warm stone the light theme casts, and heavier: a
 * shadow on an ink surface is only read as depth when it is darker than the
 * surface it falls on.
 */
export const registreShadow = {
  sm: literal("0 1px 2px rgb(0 0 0 / 0.4)"),
  md: literal("0 4px 12px rgb(0 0 0 / 0.5)"),
  lg: literal("0 12px 32px rgb(0 0 0 / 0.6)"),
} satisfies TokenGroup;

/** The five colour groups a theme owns, one entry per theme. */
export interface ThemeColors {
  readonly color: TokenGroup;
  readonly surface: TokenGroup;
  readonly text: TokenGroup;
  readonly border: TokenGroup;
  readonly shadow: TokenGroup;
}

export const themes: Readonly<Record<ThemeName, ThemeColors>> = {
  comptoir: { color, surface, text: textColor, border: borderColor, shadow },
  registre: {
    color: registreColor,
    surface: registreSurface,
    text: registreText,
    border: registreBorder,
    shadow: registreShadow,
  },
};

export const themeColorsOf = (name: ThemeName): ThemeColors => {
  const found = themes[name];
  if (found === undefined) {
    throw new Error(`unknown theme: ${name}`);
  }
  return found;
};
