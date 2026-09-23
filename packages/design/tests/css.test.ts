import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { themeSelector, toCss } from "../src/css";
import { THEMES } from "../src/semantic";
import type { ThemeName } from "../src/semantic";

/**
 * design/shared/tokens.css is the hand-written source of the token values.
 * The mockups in design/ load it; the apps read what packages/design emits.
 * These assertions fail the moment the two disagree on one variable name or
 * one value, in either direction, so neither side can drift alone.
 */

const SOURCE = fileURLToPath(new URL("../../../design/shared/tokens.css", import.meta.url));

/** Collapse runs of whitespace so `rgb(26 24 21 / 0.45)` compares by content. */
const normalise = (value: string): string => value.trim().replace(/\s+/g, " ");

/** Every `--name: value;` declaration inside the first block with this selector. */
const declarations = (css: string, selector: string): Record<string, string> => {
  const withoutComments = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const block = new RegExp(`${escaped}\\s*\\{([^}]*)\\}`).exec(withoutComments);
  if (block === null) {
    throw new Error(`no ${selector} block found`);
  }
  const body = block[1];
  if (body === undefined) {
    throw new Error(`empty ${selector} block`);
  }
  const found: Record<string, string> = {};
  for (const line of body.split(";")) {
    const trimmed = line.trim();
    if (trimmed === "") {
      continue;
    }
    const colon = trimmed.indexOf(":");
    if (colon === -1) {
      throw new Error(`declaration without a colon in ${selector}: ${trimmed}`);
    }
    found[normalise(trimmed.slice(0, colon))] = normalise(trimmed.slice(colon + 1));
  }
  return found;
};

const source = readFileSync(SOURCE, "utf8");
const emitted = toCss();

/** Every theme but Comptoir, which is `:root`. */
const OVERRIDES: readonly ThemeName[] = THEMES.slice(1);

/** `--text-` is two groups: these are the colours, the rest are font sizes. */
const TEXT_COLOUR_ROLES: ReadonlySet<string> = new Set([
  "--text-primary",
  "--text-secondary",
  "--text-tertiary",
  "--text-disabled",
  "--text-on-inverse",
  "--text-on-sidebar",
  "--text-on-sidebar-muted",
  "--text-danger",
  "--text-success",
]);

/** A name the theme axis owns, so every theme block has to redeclare it. */
const themed = (name: string): boolean =>
  name.startsWith("--color-") ||
  name.startsWith("--surface-") ||
  name.startsWith("--border-") ||
  name.startsWith("--shadow-") ||
  name.startsWith("--radius-") ||
  TEXT_COLOUR_ROLES.has(name);

describe("toCss", () => {
  it("emits the same :root variables as design/shared/tokens.css", () => {
    expect(declarations(emitted, ":root")).toEqual(declarations(source, ":root"));
  });

  it('emits the same [dir="rtl"] overrides as design/shared/tokens.css', () => {
    expect(declarations(emitted, '[dir="rtl"]')).toEqual(declarations(source, '[dir="rtl"]'));
  });

  it.each(OVERRIDES)("emits the same %s block as design/shared/tokens.css", (theme) => {
    const selector = themeSelector(theme);
    expect(declarations(emitted, selector)).toEqual(declarations(source, selector));
  });

  it("resolves every var() reference to a variable declared in the same block", () => {
    const root = declarations(emitted, ":root");
    for (const [name, value] of Object.entries(root)) {
      const reference = /^var\((--[a-z0-9-]+)\)$/.exec(value);
      if (reference === null) {
        continue;
      }
      const target = reference[1];
      if (target === undefined) {
        throw new Error(`unparsable reference on ${name}`);
      }
      expect(Object.keys(root)).toContain(target);
    }
  });

  /**
   * A theme block carries no primitives of its own; it points at the ones
   * `:root` declares. A typo there would resolve to nothing and the property
   * would fall back to the Comptoir value, which is the one failure mode a
   * dark theme cannot show loudly.
   */
  it.each(OVERRIDES)("resolves every %s reference to a primitive in :root", (theme) => {
    const root = declarations(emitted, ":root");
    for (const [name, value] of Object.entries(declarations(emitted, themeSelector(theme)))) {
      const reference = /^var\((--[a-z0-9-]+)\)$/.exec(value);
      if (reference === null) {
        continue;
      }
      const target = reference[1];
      if (target === undefined) {
        throw new Error(`unparsable reference on ${name}`);
      }
      expect(Object.keys(root)).toContain(target);
    }
  });

  /**
   * Every theme names every role Comptoir does, so none falls through. A role
   * left out of a dark block is a light colour on a dark surface, and that
   * miss only shows on the one screen nobody opened.
   */
  it.each(OVERRIDES)("%s overrides every themed role Comptoir declares", (theme) => {
    const light = Object.keys(declarations(emitted, ":root")).filter(themed);
    const dark = Object.keys(declarations(emitted, themeSelector(theme)));
    expect([...dark].sort()).toEqual([...light].sort());
  });
});

/** WCAG relative luminance of an `#rrggbb`. */
const luminance = (hex: string): number => {
  const channel = (offset: number): number => {
    const raw = Number.parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return raw <= 0.03928 ? raw / 12.92 : ((raw + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
};

const contrast = (a: string, b: string): number => {
  const [high, low] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  if (high === undefined || low === undefined) {
    throw new Error("two colours are needed");
  }
  return (high + 0.05) / (low + 0.05);
};

/**
 * "red, amber and blue kept legible" is the whole reason a dark theme has its
 * own steps rather than reusing the light ones, so it is asserted rather than
 * eyeballed, and on every theme rather than only the dark pair. Text a person
 * reads to act clears 4.5:1 on its own page background; the two dim tiers
 * clear 3:1.
 */
describe.each(THEMES)("%s contrast", (theme) => {
  const root = declarations(emitted, ":root");
  const own = theme === "comptoir" ? {} : declarations(emitted, themeSelector(theme));

  const hex = (role: string): string => {
    const value = own[role] ?? root[role];
    if (value === undefined) {
      throw new Error(`no role ${role} on ${theme}`);
    }
    const reference = /^var\((--[a-z0-9-]+)\)$/.exec(value);
    if (reference === null) {
      return value;
    }
    const target = reference[1];
    const resolved = target === undefined ? undefined : root[target];
    if (resolved === undefined) {
      throw new Error(`${role} points at ${value}, which :root does not declare`);
    }
    return resolved;
  };

  const bg = () => hex("--surface-bg");

  /** Sentence text on the page. WCAG AA for body copy. */
  it.each(["--text-primary", "--text-secondary", "--text-danger"])(
    "%s reads at 4.5:1 on the page background",
    (role) => {
      expect(contrast(hex(role), bg())).toBeGreaterThanOrEqual(4.5);
    },
  );

  /**
   * A status colour and the text on a filled control, at WCAG AA's 3:1 for a
   * user interface component rather than the 4.5:1 for body copy. Both always
   * come with a word beside them, and holding them to the body bar would mean
   * refusing the brand colours on two of the four approved directions.
   */
  it.each(["--text-success", "--color-danger", "--color-info"])(
    "%s reads at 3:1 on the page background",
    (role) => {
      expect(contrast(hex(role), bg())).toBeGreaterThanOrEqual(3);
    },
  );

  it("puts readable text on the sidebar", () => {
    expect(contrast(hex("--text-on-sidebar"), hex("--surface-sidebar"))).toBeGreaterThanOrEqual(4.5);
  });

  it("puts readable text on the primary and money buttons", () => {
    expect(contrast(hex("--color-on-primary"), hex("--color-primary"))).toBeGreaterThanOrEqual(3);
    expect(contrast(hex("--color-on-money"), hex("--color-money"))).toBeGreaterThanOrEqual(3);
  });
});
