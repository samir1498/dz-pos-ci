// Assembly. Every reference resolved. This is what TypeScript imports:
// `import { theme } from "@dzpos/design"`. A plain object, not a hook, so it
// works at module scope inside a StyleSheet.create or a style helper.
//
// The desktop reads the CSS custom properties instead, and reaches for this
// only where a value has to be computed in JS.

import { hexOf } from "./primitives";
import {
  borderColor,
  color,
  fontFamily,
  fontSize,
  layout,
  radius,
  registreBorder,
  registreColor,
  registreShadow,
  registreSurface,
  registreText,
  shadow,
  space,
  surface,
  textColor,
} from "./semantic";
import type { PxGroup, ThemeName, Token, TokenGroup, TokenPx } from "./semantic";

const stringValue = (token: Token): string => {
  switch (token.kind) {
    case "ref":
      return hexOf(token.family, token.step);
    case "literal":
      return token.value;
    case "px":
      return `${token.value}px`;
  }
};

// The overload keeps the caller's key union; the implementation signature is
// the loose one TypeScript checks the body against. That is what lets this
// stay cast-free, and `as` is banned repo-wide.
export function resolveStrings<K extends string>(
  group: Readonly<Record<K, Token>>,
): Readonly<Record<K, string>>;
export function resolveStrings(group: TokenGroup): Readonly<Record<string, string>> {
  const out: Record<string, string> = {};
  for (const [key, token] of Object.entries(group)) {
    out[key] = stringValue(token);
  }
  return out;
}

export function resolveLengths<K extends string>(
  group: Readonly<Record<K, TokenPx>>,
): Readonly<Record<K, number>>;
export function resolveLengths(group: PxGroup): Readonly<Record<string, number>> {
  const out: Record<string, number> = {};
  for (const [key, token] of Object.entries(group)) {
    out[key] = token.value;
  }
  return out;
}

export const theme = {
  colors: {
    ...resolveStrings(color),
    surface: resolveStrings(surface),
    text: resolveStrings(textColor),
    border: resolveStrings(borderColor),
  },
  space: resolveLengths(space),
  radius: resolveLengths(radius),
  shadow: resolveStrings(shadow),
  fontFamily: resolveStrings(fontFamily),
  fontSize: resolveLengths(fontSize),
  layout: resolveLengths(layout),
};

export type Theme = typeof theme;

/**
 * Both themes resolved. The desktop never reads this: it switches with the
 * `data-theme` attribute and the cascade does the work. React Native has no
 * cascade, so mobile picks the object and re-renders. `theme` above stays
 * Comptoir so an existing import keeps meaning what it meant.
 *
 * The Registre groups carry the same keys as the Comptoir ones (the parity
 * test in css.test.ts is what holds that), which is why this typechecks as
 * a `Theme` without a cast.
 */
export const themes: Readonly<Record<ThemeName, Theme>> = {
  comptoir: theme,
  registre: {
    ...theme,
    colors: {
      ...resolveStrings(registreColor),
      surface: resolveStrings(registreSurface),
      text: resolveStrings(registreText),
      border: resolveStrings(registreBorder),
    },
    shadow: resolveStrings(registreShadow),
  },
};
