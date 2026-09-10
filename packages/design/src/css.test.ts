import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { toCss } from "./css";

/**
 * design/shared/tokens.css is the hand-written source of the token values.
 * The mockups in design/ load it; the apps will read what packages/design
 * emits. These assertions fail the moment the two disagree on one variable
 * name or one value, in either direction, so neither side can drift alone.
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

describe("toCss", () => {
  it("emits the same :root variables as design/shared/tokens.css", () => {
    expect(declarations(emitted, ":root")).toEqual(declarations(source, ":root"));
  });

  it('emits the same [dir="rtl"] overrides as design/shared/tokens.css', () => {
    expect(declarations(emitted, '[dir="rtl"]')).toEqual(declarations(source, '[dir="rtl"]'));
  });

  it('emits the same [data-theme="registre"] overrides as design/shared/tokens.css', () => {
    expect(declarations(emitted, '[data-theme="registre"]')).toEqual(
      declarations(source, '[data-theme="registre"]'),
    );
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
   * The theme block carries no primitives of its own; it points at the ones
   * `:root` declares. A typo there would resolve to nothing and the property
   * would fall back to the Comptoir value, which is the one failure mode a
   * dark theme cannot show loudly.
   */
  it("resolves every registre reference to a primitive declared in :root", () => {
    const root = declarations(emitted, ":root");
    const dark = declarations(emitted, '[data-theme="registre"]');
    for (const [name, value] of Object.entries(dark)) {
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

  /** Registre names every colour role Comptoir does, so none falls through. */
  it("overrides every colour role Comptoir declares", () => {
    const colourish = (name: string): boolean =>
      name.startsWith("--color-") ||
      name.startsWith("--surface-") ||
      name.startsWith("--border-") ||
      name.startsWith("--shadow-") ||
      TEXT_COLOUR_ROLES.has(name);
    const light = Object.keys(declarations(emitted, ":root")).filter(colourish);
    const dark = Object.keys(declarations(emitted, '[data-theme="registre"]'));
    expect([...dark].sort()).toEqual([...light].sort());
  });
});

/** `--text-` is two groups: these are the colours, the rest are font sizes. */
const TEXT_COLOUR_ROLES: ReadonlySet<string> = new Set([
  "--text-primary",
  "--text-secondary",
  "--text-tertiary",
  "--text-disabled",
  "--text-on-inverse",
  "--text-danger",
  "--text-success",
]);

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
 * "red, amber and blue kept legible" is the whole reason the dark theme has
 * its own steps rather than reusing the light ones, so it is asserted rather
 * than eyeballed. Text a person reads to act clears 4.5:1 on the theme's own
 * background; tertiary and disabled are the dimmed tiers and clear 3:1.
 */
describe("registre contrast", () => {
  const root = declarations(emitted, ":root");
  const dark = declarations(emitted, '[data-theme="registre"]');

  const hex = (role: string): string => {
    const value = dark[role] ?? root[role];
    if (value === undefined) {
      throw new Error(`no role ${role}`);
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

  it.each([
    "--text-primary",
    "--text-secondary",
    "--text-danger",
    "--text-success",
    "--color-danger",
    "--color-warn",
    "--color-info",
    "--color-money",
  ])("%s reads at 4.5:1 on the ink background", (role) => {
    expect(contrast(hex(role), bg())).toBeGreaterThanOrEqual(4.5);
  });

  it.each(["--text-tertiary", "--text-disabled"])("%s reads at 3:1", (role) => {
    expect(contrast(hex(role), bg())).toBeGreaterThanOrEqual(3);
  });

  it("puts readable text on the primary and money buttons", () => {
    expect(contrast(hex("--color-on-primary"), hex("--color-primary"))).toBeGreaterThanOrEqual(4.5);
    expect(contrast(hex("--color-on-money"), hex("--color-money"))).toBeGreaterThanOrEqual(4.5);
  });
});
