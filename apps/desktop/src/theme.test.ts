import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

import { OS_THEME, THEMES } from "@dzpos/design";
import { describe, expect, it } from "vitest";

import { THEME_LABEL } from "@/components/ThemeSwitcher";
import ar from "@/i18n/ar.json";
import en from "@/i18n/en.json";
import fr from "@/i18n/fr.json";

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

/**
 * The switcher is the one component allowed to say "Comptoir" out loud, so
 * the labels are checked where the names already live. `keys.test.ts` proves
 * the three dictionaries carry the same key set; this proves that set holds
 * the four keys the select reaches for, in each file, because a theme whose
 * label is missing renders an option the shop cannot tell from the others.
 */
describe("every theme's label", () => {
  it.each(THEMES)("is in fr, en and ar for %s", (name) => {
    const key = THEME_LABEL[name];
    for (const [lang, dict] of [
      ["fr", fr],
      ["en", en],
      ["ar", ar],
    ] satisfies readonly (readonly [string, Readonly<Record<string, string>>])[]) {
      expect({ lang, key, label: dict[key] }).toEqual({ lang, key, label: expect.any(String) });
      expect(dict[key]?.trim()).not.toBe("");
    }
  });
});

/**
 * The one place outside the provider and the switcher that has to name the
 * themes, and it cannot import them: index.html's anti-flash script runs
 * before any module is fetched, so it carries its own copy of the list and
 * its own copy of the OS fallback. A third theme was added to the package on
 * 2026-09-10 and this script kept two names for an afternoon, which is
 * exactly the failure a hand-copied list produces: nothing breaks, the fresh
 * machine just lands on the wrong theme and the provider corrects it a second
 * later, so the flash the script exists to remove comes back and no test
 * says so.
 *
 * So the copy stays (it must) and is checked against the source here.
 * `apps/desktop/e2e/theme.spec.ts` asserts its own list against the same
 * file, which chains the three together.
 */
describe("the anti-flash script in index.html", () => {
  const html = readFileSync(join(process.cwd(), "index.html"), "utf8");

  it("lists exactly the themes the design package declares", () => {
    const match = /var names = \[([^\]]*)\];/.exec(html);
    expect(match).not.toBeNull();
    const names = [...(match?.[1] ?? "").matchAll(/"([^"]+)"/g)].map((hit) => hit[1]);
    expect(names).toEqual([...THEMES]);
  });

  /**
   * The ternary the script falls back on when nothing is stored. Read as a
   * pair rather than a single name: swapping the two arms would leave a dark
   * machine on the light theme and still match a one-sided assertion.
   */
  it("falls back to the same two OS themes as the provider", () => {
    const match = /\.matches\s*\?\s*"([^"]+)"\s*:\s*"([^"]+)";/.exec(html);
    expect(match).not.toBeNull();
    expect({ dark: match?.[1], light: match?.[2] }).toEqual({
      dark: OS_THEME.dark,
      light: OS_THEME.light,
    });
  });
});
