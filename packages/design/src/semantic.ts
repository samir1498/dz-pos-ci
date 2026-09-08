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

export const color = {
  primary: ref("teal", 600),
  "primary-hover": ref("teal", 700),
  "primary-soft": ref("teal", 50),
  "on-primary": ref("stone", 0),
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
  inverse: ref("stone", 800),
} satisfies TokenGroup;

export const textColor = {
  primary: ref("stone", 900),
  secondary: ref("stone", 600),
  tertiary: ref("stone", 400),
  disabled: ref("stone", 300),
  "on-inverse": ref("stone", 50),
  danger: ref("red", 600),
  success: ref("teal", 600),
} satisfies TokenGroup;

export const borderColor = {
  default: ref("stone", 200),
  strong: ref("stone", 300),
  focus: ref("teal", 500),
  danger: ref("red", 500),
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
  numeric: literal('"IBM Plex Sans", system-ui, sans-serif'),
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
