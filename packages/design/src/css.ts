// The CSS custom properties, emitted from the same semantic entries theme.ts
// resolves. src/css.test.ts asserts this output carries exactly the variable
// names and values in design/shared/tokens.css, so the mockups and the apps
// cannot drift while both read tokens.

import { families, rampOf } from "./primitives";
import {
  borderColor,
  color,
  fontFamily,
  fontSize,
  layout,
  radius,
  rtl,
  shadow,
  space,
  surface,
  textColor,
} from "./semantic";
import type { Token, TokenGroup } from "./semantic";

interface Section {
  /** Prepended to every key in the group. Empty where the key is the whole name. */
  readonly prefix: string;
  readonly group: TokenGroup;
}

/** Order matches design/shared/tokens.css so a diff of the two reads straight. */
const SECTIONS: readonly Section[] = [
  { prefix: "color-", group: color },
  { prefix: "surface-", group: surface },
  { prefix: "text-", group: textColor },
  { prefix: "border-", group: borderColor },
  { prefix: "space-", group: space },
  { prefix: "radius-", group: radius },
  { prefix: "shadow-", group: shadow },
  { prefix: "font-", group: fontFamily },
  { prefix: "text-", group: fontSize },
  { prefix: "", group: layout },
];

export const primitiveVarName = (family: string, step: string): string => `--p-${family}-${step}`;

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

const primitiveLines = (): string[] =>
  families.flatMap((family) =>
    Object.entries(rampOf(family)).map(([step, hex]) =>
      declaration(primitiveVarName(family, step), hex),
    ),
  );

const semanticLines = (): string[] =>
  SECTIONS.flatMap(({ prefix, group }) =>
    Object.entries(group).map(([key, token]) =>
      declaration(`--${prefix}${key}`, cssValue(token)),
    ),
  );

const block = (selector: string, lines: readonly string[]): string =>
  `${selector} {\n${lines.join("\n")}\n}`;

/** The full stylesheet: the `:root` block and the `[dir="rtl"]` overrides. */
export const toCss = (): string => {
  const root = block(":root", [
    "  /* Tier 1: primitives. No component reads these. */",
    ...primitiveLines(),
    "",
    "  /* Tier 2: semantic. This is the API. */",
    ...semanticLines(),
  ]);
  const rtlLines = Object.entries(rtl).map(([key, token]) =>
    declaration(`--font-${key}`, cssValue(token)),
  );
  return `${root}\n\n${block('[dir="rtl"]', rtlLines)}\n`;
};
