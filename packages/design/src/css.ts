// The CSS custom properties, emitted from the same semantic entries theme.ts
// resolves. src/css.test.ts asserts this output carries exactly the variable
// names and values in design/shared/tokens.css, so the mockups and the apps
// cannot drift while both read tokens.
//
// One block per theme. `:root` is the primitives plus Comptoir, the light
// theme and the default; every other theme is a `[data-theme="<name>"]` block
// re-declaring the whole themed set. `[dir="rtl"]` swaps the font stack.
//
// That is the entire theme mechanism. A component reads a role name and never
// learns which theme it is in: no branch, no per-theme component, nothing in
// TypeScript at all beyond the attribute the switcher writes on <html>.

import { families, rampOf } from "./primitives";
import { THEMES, fontFamily, fontSize, layout, rtl, space, themeTokensOf } from "./semantic";
import type { ThemeName, ThemeTokens, Token, TokenGroup } from "./semantic";

/** A group whose values change with the theme; `role` picks it out of one. */
interface ThemedSection {
  readonly kind: "themed";
  /** Prepended to every key in the group. */
  readonly prefix: string;
  readonly role: keyof ThemeTokens;
}

/** A group the theme axis does not touch: a scale, a font stack, a length. */
interface FixedSection {
  readonly kind: "fixed";
  /** Empty where the key is the whole name. */
  readonly prefix: string;
  readonly group: TokenGroup;
}

type Section = ThemedSection | FixedSection;

/** Order matches design/shared/tokens.css so a diff of the two reads straight. */
const SECTIONS: readonly Section[] = [
  { kind: "themed", prefix: "color-", role: "color" },
  { kind: "themed", prefix: "surface-", role: "surface" },
  { kind: "themed", prefix: "text-", role: "text" },
  { kind: "themed", prefix: "border-", role: "border" },
  { kind: "fixed", prefix: "space-", group: space },
  { kind: "themed", prefix: "radius-", role: "radius" },
  { kind: "themed", prefix: "shadow-", role: "shadow" },
  { kind: "fixed", prefix: "font-", group: fontFamily },
  { kind: "fixed", prefix: "text-", group: fontSize },
  { kind: "fixed", prefix: "", group: layout },
];

export const primitiveVarName = (family: string, step: string): string => `--p-${family}-${step}`;

/** The selector a theme's declarations live under. Comptoir is the default. */
export const themeSelector = (name: ThemeName): string =>
  name === "comptoir" ? ":root" : `[data-theme="${name}"]`;

const cssValue = (token: Token): string => {
  switch (token.kind) {
    case "ref":
      return `var(${primitiveVarName(token.family, String(token.step))})`;
    case "literal":
      return token.value;
    case "px":
      return `${token.value}px`;
  }
};

const declaration = (name: string, value: string): string => `  ${name}: ${value};`;

const groupLines = (prefix: string, group: TokenGroup): string[] =>
  Object.entries(group).map(([key, token]) => declaration(`--${prefix}${key}`, cssValue(token)));

const primitiveLines = (): string[] =>
  families.flatMap((family) =>
    Object.entries(rampOf(family)).map(([step, hex]) =>
      declaration(primitiveVarName(family, step), hex),
    ),
  );

/** Every section for `:root`; only the themed ones for an override block. */
const semanticLines = (theme: ThemeName, only: "themed" | "all"): string[] => {
  const colors = themeTokensOf(theme);
  return SECTIONS.flatMap((section) => {
    if (section.kind === "themed") {
      return groupLines(section.prefix, colors[section.role]);
    }
    return only === "all" ? groupLines(section.prefix, section.group) : [];
  });
};

const block = (selector: string, lines: readonly string[]): string =>
  `${selector} {\n${lines.join("\n")}\n}`;

/** The full stylesheet: `:root`, one block per other theme, the RTL swap. */
export const toCss = (): string => {
  const root = block(":root", [
    "  /* Tier 1: primitives. No component reads these. */",
    ...primitiveLines(),
    "",
    "  /* Tier 2: semantic. This is the API. */",
    ...semanticLines("comptoir", "all"),
  ]);
  // Every theme but Comptoir, in declaration order. Comptoir is `:root`.
  const overrides = THEMES.slice(1).map((theme) =>
    block(themeSelector(theme), semanticLines(theme, "themed")),
  );
  const rtlLines = Object.entries(rtl).map(([key, token]) =>
    declaration(`--font-${key}`, cssValue(token)),
  );
  return `${[root, ...overrides].join("\n\n")}\n\n${block('[dir="rtl"]', rtlLines)}\n`;
};
