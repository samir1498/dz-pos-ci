import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { THEME_CSS_PATH, toThemeCss } from "./themeCss";

/**
 * apps/desktop/src/theme.css is generated and checked in, so the browser
 * reads a plain stylesheet and nothing runs a build step to get one. That
 * only holds while the committed file is the one this source produces, which
 * is what the first test asserts: a token changed without regenerating fails
 * the gates instead of shipping a screen the tokens no longer describe.
 *
 * `just theme` runs this file with DZPOS_THEME_CSS_WRITE set and writes
 * instead of comparing, the same shape as `just types` beside
 * `just types-check`.
 */

const OUT = fileURLToPath(new URL(`../../../${THEME_CSS_PATH}`, import.meta.url));
const STYLES = fileURLToPath(new URL("../../../apps/desktop/src/styles.css", import.meta.url));
const INDEX = fileURLToPath(new URL("../../../apps/desktop/index.html", import.meta.url));

const generated = toThemeCss();

if (process.env.DZPOS_THEME_CSS_WRITE === "1") {
  mkdirSync(dirname(OUT), { recursive: true });
  writeFileSync(OUT, generated, "utf8");
}

/** The weights the brief asks for, per family and per subset kept. */
const EXPECTED_FONT_IMPORTS: readonly string[] = [
  ...[400, 500, 600, 700].map((w) => `@fontsource/ibm-plex-sans/latin-${w}.css`),
  ...[400, 500, 600, 700].map((w) => `@fontsource/ibm-plex-sans/latin-ext-${w}.css`),
  ...[400, 500, 600, 700].map((w) => `@fontsource/ibm-plex-sans-arabic/arabic-${w}.css`),
  ...[500, 600].map((w) => `@fontsource/jetbrains-mono/latin-${w}.css`),
  ...[500, 600].map((w) => `@fontsource/jetbrains-mono/latin-ext-${w}.css`),
];

describe("theme.css", () => {
  it("is the file the token source produces", () => {
    expect(readFileSync(OUT, "utf8")).toBe(generated);
  });

  /**
   * The shop counter has no internet. A face fetched from a CDN would look
   * fine on the box that built it and fall back to system-ui on the machine
   * that matters, so the rule is checked rather than remembered.
   *
   * Three files, not two. The generated stylesheet and styles.css are where
   * an @import would go, but the easiest way to reintroduce a Google Fonts
   * face is the one every tutorial shows: two preconnects and a stylesheet
   * link pasted into the document head. That head is
   * `apps/desktop/index.html`, which neither of the other two checks reach,
   * so it is read here as text and held to the same rule.
   */
  it.each([
    ["the generated stylesheet", () => generated],
    ["styles.css", () => readFileSync(STYLES, "utf8")],
    ["index.html", () => readFileSync(INDEX, "utf8")],
  ])("names no external host in %s", (_where, read) => {
    const text = read();
    expect(text).not.toMatch(/https?:/);
    expect(text).not.toMatch(/\/\/fonts\./);
  });

  it("imports exactly the weights and subsets the brief lists", () => {
    const imports = [...readFileSync(STYLES, "utf8").matchAll(/@import "(@fontsource\/[^"]+)"/g)]
      .map((match) => match[1])
      .sort();
    expect(imports).toEqual([...EXPECTED_FONT_IMPORTS].sort());
  });
});

describe("the shadcn variable set", () => {
  // Only the `:root { ... }` block that follows the marker: the `@theme`
  // blocks below it name the same variables the other way round, and taking
  // them in would read `--color-background: var(--background)` as a role.
  const from = generated.indexOf("shadcn/ui, on our roles");
  const shadcn = generated.slice(from, generated.indexOf("}", from) + 1);

  /**
   * The names a shadcn component installed by the CLI in D3 will reach for.
   * Missing one means that component renders with Tailwind's default palette
   * on an ink screen and nothing fails, so the list is pinned here.
   */
  it.each([
    "--background",
    "--foreground",
    "--card",
    "--card-foreground",
    "--popover",
    "--popover-foreground",
    "--primary",
    "--primary-foreground",
    "--secondary",
    "--secondary-foreground",
    "--muted",
    "--muted-foreground",
    "--accent",
    "--accent-foreground",
    "--destructive",
    "--destructive-foreground",
    "--border",
    "--input",
    "--ring",
    "--radius",
    "--sidebar",
    "--sidebar-foreground",
    "--sidebar-primary",
    "--sidebar-primary-foreground",
    "--sidebar-accent",
    "--sidebar-accent-foreground",
    "--sidebar-border",
    "--sidebar-ring",
    "--money",
    "--money-foreground",
  ])("declares %s", (name) => {
    expect(shadcn).toMatch(new RegExp(`^\\s*${name}: var\\(--[a-z0-9-]+\\);$`, "m"));
  });

  /**
   * Every shadcn name points at one of our roles, never at a literal. That
   * is what makes the block theme-proof: [data-theme="registre"] redefines
   * the role and the shadcn name follows without being re-emitted.
   */
  it("points every name at a role this file declares", () => {
    const tokens = generated.slice(0, generated.indexOf("shadcn/ui, on our roles"));
    for (const [, role] of shadcn.matchAll(/^\s*--[a-z0-9-]+: var\((--[a-z0-9-]+)\);$/gm)) {
      expect(tokens).toMatch(new RegExp(`^\\s*${role}:`, "m"));
    }
  });

  /**
   * Tailwind's `--text-*` namespace means font size. Declaring a colour role
   * there would compile `text-primary` to `font-size: <a colour>`, so only
   * the sizes carry that prefix inside the theme blocks.
   */
  it("keeps colour roles out of Tailwind's font-size namespace", () => {
    const theme = generated.slice(generated.indexOf("@theme"));
    for (const role of ["primary", "secondary", "tertiary", "disabled", "danger", "success"]) {
      expect(theme).not.toMatch(new RegExp(`^\\s*--text-${role}:`, "m"));
    }
    expect(theme).toMatch(/^\s*--text-md: var\(--text-md\);$/m);
    expect(theme).toMatch(/^\s*--color-background: var\(--background\);$/m);
    expect(theme).toMatch(/^\s*--color-muted-foreground: var\(--muted-foreground\);$/m);
    expect(theme).toMatch(/^\s*--color-money: var\(--money\);$/m);
  });

  /** A key declared in both theme blocks would resolve to whichever came last. */
  it("declares no Tailwind key twice", () => {
    const theme = generated.slice(generated.indexOf("@theme"));
    const keys = [...theme.matchAll(/^\s*(--[a-z0-9-]+):/gm)].map((match) => match[1]);
    expect(keys.length).toBe(new Set(keys).size);
  });
});
