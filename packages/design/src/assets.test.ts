import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { families, hexOf, rampOf, type PrimitiveFamily } from "./primitives";

/**
 * The logo is the one drawing in the product, so its colours are the one
 * place a hex is typed by hand instead of pointing at a token. That is fine
 * as far as it goes: an SVG cannot read a custom property that is not on the
 * page, and the mark has to render inside a ticket, a favicon and a Tauri
 * window icon where no stylesheet is loaded at all.
 *
 * What is not fine is the pair drifting silently. Nudge the brass a step in
 * `primitives.ts` and the coin in the sidebar keeps the old value; nobody
 * sees it, because the two are never next to each other on a screen, until
 * Samir puts a lockup on a slide beside a pay button. So every hex in every
 * asset is read back and has to be a value the ramps declare.
 */

const ASSETS = fileURLToPath(new URL("../assets", import.meta.url));

const files = readdirSync(ASSETS).filter((name) => name.endsWith(".svg"));

const hexesIn = (file: string): string[] => {
  const svg = readFileSync(join(ASSETS, file), "utf8");
  return [...new Set([...svg.matchAll(/#[0-9a-fA-F]{3,8}/g)].map((hit) => hit[0].toLowerCase()))];
};

/** Every value any ramp declares, as one set to look a hex up in. */
const KNOWN: ReadonlySet<string> = new Set(
  families.flatMap((family) => Object.values(rampOf(family))),
);

/**
 * The one exception, and it is a printing fact rather than a design choice:
 * the ticket header goes to an 80 mm thermal printer, which has one ink and
 * no grey. Pure black is what the driver expects; the ink 800 the screen uses
 * would be dithered into a smudge at 203 dpi.
 */
const BLACK = "#000000";

describe("the logo assets", () => {
  it("has the files the desktop and the ticket reach for", () => {
    expect(files.sort()).toEqual([
      "favicon.svg",
      "lockup-dark.svg",
      "lockup-light.svg",
      "mark-mono.svg",
      "mark.svg",
      "ticket-header.svg",
      "wordmark-arabic.svg",
      "wordmark-latin.svg",
    ]);
  });

  it.each(files)("paints %s only in colours the ramps declare", (file) => {
    const allowed = file === "ticket-header.svg" ? new Set([...KNOWN, BLACK]) : KNOWN;
    expect(hexesIn(file).filter((hex) => !allowed.has(hex))).toEqual([]);
  });

  /**
   * The two named ones, pinned to the exact step rather than to "some value
   * in some ramp", because these are the identity: the coin is brass and the
   * stroke through it is the ink the sidebar is made of. A change to either
   * is a brand change and has to be made in both places on purpose.
   */
  it.each([
    ["mark.svg", "brass", 500],
    ["mark.svg", "ink", 800],
    ["favicon.svg", "brass", 500],
    ["favicon.svg", "ink", 800],
    ["lockup-dark.svg", "brass", 500],
    ["lockup-light.svg", "brass", 500],
  ] satisfies readonly (readonly [string, PrimitiveFamily, number])[])(
    "sets %s in %s %i",
    (file, family, step) => {
      expect(hexesIn(file)).toContain(hexOf(family, step));
    },
  );

  /** One colour, so it takes the ink of whatever it is dropped into. */
  it("keeps the mono mark on currentColor and off any hex", () => {
    expect(readFileSync(join(ASSETS, "mark-mono.svg"), "utf8")).toContain("currentColor");
    expect(hexesIn("mark-mono.svg")).toEqual([]);
  });

  /** The thermal printer has one ink; a grey there is a smudge, not a tint. */
  it("keeps the ticket header in pure black", () => {
    expect(hexesIn("ticket-header.svg")).toEqual([BLACK]);
  });
});
