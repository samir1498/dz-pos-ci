import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { THEME_CSS_PATH, fontFaces, toThemeCss } from "./themeCss";

/**
 * apps/desktop/src/theme.css is generated and checked in, so the browser
 * reads a plain stylesheet and nothing runs a build step to get one. That
 * only holds while the committed file is the one this source produces, which
 * is what the first test asserts: a token changed without regenerating fails
 * the gates instead of shipping a screen the tokens no longer describe.
 *
 * `pnpm --filter @dzpos/design gen:theme` runs this file with
 * DZPOS_THEME_CSS_WRITE set and writes the file instead of comparing, the
 * same shape as `just types` beside `just types-check`.
 */

const OUT = fileURLToPath(new URL(`../../../${THEME_CSS_PATH}`, import.meta.url));

const generated = toThemeCss();

if (process.env.DZPOS_THEME_CSS_WRITE === "1") {
  mkdirSync(dirname(OUT), { recursive: true });
  writeFileSync(OUT, generated, "utf8");
}

describe("theme.css", () => {
  it("is the file the token source produces", () => {
    expect(readFileSync(OUT, "utf8")).toBe(generated);
  });

  /**
   * The shop counter has no internet. A face fetched from a CDN would look
   * fine on the box that built it and fall back to system-ui on the machine
   * that matters, so the rule is checked rather than remembered.
   */
  it("names no external host", () => {
    expect(generated).not.toMatch(/https?:/);
    expect(generated).not.toMatch(/\/\/fonts\./);
  });

  it("points every face at a vendored woff2", () => {
    const sources = [...generated.matchAll(/url\("([^"]+)"\)/g)].map((m) => m[1]);
    expect(sources.length).toBe(fontFaces().length);
    for (const source of sources) {
      expect(source).toMatch(/^\.\.\/\.\.\/\.\.\/packages\/design\/assets\/fonts\/[\w.-]+\.woff2$/);
    }
  });

  /** The three families the token font stacks name, and nothing else. */
  it("carries a face for every family the tokens name", () => {
    const families = new Set(fontFaces().map((face) => face.family));
    expect([...families].sort()).toEqual([
      "IBM Plex Sans",
      "IBM Plex Sans Arabic",
      "JetBrains Mono",
    ]);
  });

  /**
   * `--text-primary` is a colour and `--text-md` is a font size, and
   * Tailwind's `--text-*` namespace means font size. Declaring the colour
   * there would compile `text-primary` to `font-size: <a colour>`, so the
   * colour roles are aliased into `--color-*` instead and only sizes keep
   * the prefix.
   */
  it("keeps colour roles out of Tailwind's font-size namespace", () => {
    const theme = generated.slice(generated.indexOf("@theme"));
    for (const role of ["primary", "secondary", "tertiary", "disabled", "danger", "success"]) {
      expect(theme).not.toMatch(new RegExp(`^\\s*--text-${role}:`, "m"));
    }
    expect(theme).toMatch(/^\s*--text-md: var\(--text-md\);$/m);
    expect(theme).toMatch(/^\s*--color-fg: var\(--text-primary\);$/m);
    expect(theme).toMatch(/^\s*--color-muted: var\(--text-secondary\);$/m);
  });
});
