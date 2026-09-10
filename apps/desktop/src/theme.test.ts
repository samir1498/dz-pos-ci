import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

import { THEMES } from "@dzpos/design";
import { describe, expect, it } from "vitest";

/**
 * A theme is a block of CSS variables and nothing else. The switch is one
 * attribute on `<html>`; a component wears `bg-background` and
 * `text-muted-foreground` and never learns which theme is on.
 *
 * That rule is worth a test rather than a paragraph because breaking it is
 * cheap and invisible: one `theme === "registre" ? "bg-black" : "bg-white"`
 * renders correctly on the two themes its author had open and wrongly on the
 * other two, and adding a fifth theme then means finding every such branch
 * instead of writing one CSS block. So the theme names and the attribute may
 * appear in exactly two files, the provider that writes the attribute and the
 * switcher that carries the labels.
 */

// process.cwd(), not import.meta.url: these run under jsdom, where
// import.meta.url is an http:// URL and fileURLToPath refuses it.
const SRC = join(process.cwd(), "src");

/**
 * The files that name a theme on purpose: the provider that writes the
 * attribute, the switcher that carries the labels, their own test, the
 * generated stylesheet that declares the blocks, and this test, which has to
 * spell what it bans. Named one by one rather than "any test file", so a
 * branch cannot hide in a test either.
 */
const ALLOWED: ReadonlySet<string> = new Set([
  join("lib", "theme.tsx"),
  join("lib", "theme.test.tsx"),
  join("components", "ThemeSwitcher.tsx"),
  "theme.css",
  "theme.test.ts",
]);

const sources = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) return sources(full);
    return /\.(ts|tsx)$/.test(entry.name) ? [full] : [];
  });

const offenders = (pattern: RegExp): string[] =>
  sources(SRC)
    .filter((file) => !ALLOWED.has(relative(SRC, file)))
    .flatMap((file) =>
      readFileSync(file, "utf8")
        .split("\n")
        .map((text, index) => ({ file: relative(SRC, file), line: index + 1, text }))
        .filter((row) => pattern.test(row.text))
        .map((row) => `${row.file}:${row.line}: ${row.text.trim()}`),
    );

describe("the theme is an attribute, not a branch", () => {
  it("names no theme outside the provider and the switcher", () => {
    // Word boundaries so `observe-dark` does not also report as `observe`,
    // and so a variable called `observer` is not a hit.
    const names = THEMES.map((name) => name.replace("-", "\\-")).join("|");
    expect(offenders(new RegExp(`["'\`](${names})["'\`]`))).toEqual([]);
  });

  it("writes data-theme in one place", () => {
    expect(offenders(/data-theme|dataset\.theme/)).toEqual([]);
  });

  /**
   * The shape a branch takes even when the name is held in a constant: a
   * ternary or a comparison against the resolved theme.
   */
  it("compares nothing against the current theme", () => {
    expect(offenders(/\b(resolved|theme)\s*===\s*["'`]/)).toEqual([]);
  });

  it("keeps every theme reachable through the same variable names", () => {
    // Four blocks in the generated stylesheet, one per theme, and Comptoir is
    // `:root`. A theme added to the package without a block would leave the
    // switcher offering a name that changes nothing.
    const css = readFileSync(join(SRC, "theme.css"), "utf8");
    for (const name of THEMES.slice(1)) {
      expect(css).toContain(`[data-theme="${name}"]`);
    }
  });
});
