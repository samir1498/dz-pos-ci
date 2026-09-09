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
});
