import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

import { describe, expect, it } from "vitest";

/**
 * A colour in this app comes from a token or it is a mistake. Tailwind's
 * arbitrary-value syntax (`bg-[#14211c]`, `text-[13px]`) and a bare hex both
 * walk straight past the design package, and neither shows up as a failure:
 * the screen renders, it just renders a colour the other theme never heard
 * of, so the dark themes leave it light on an ink surface.
 *
 * No lint rule reads intent, but these three shapes are mechanical, so they
 * are a test rather than a convention. `theme.css` is generated from the
 * token source and is the one file allowed to spell a hex; `routeTree.gen.ts`
 * is generated too.
 */

// process.cwd(), not import.meta.url: these run under jsdom, where
// import.meta.url is an http:// URL and fileURLToPath refuses it.
const SRC = join(process.cwd(), "src");

/** Generated files, and this test, which has to name the patterns it bans. */
const EXEMPT: ReadonlySet<string> = new Set(["theme.css", "routeTree.gen.ts", "tokens.test.ts"]);

const BANNED: readonly { readonly pattern: RegExp; readonly why: string }[] = [
  { pattern: /\bbg-\[/, why: "an arbitrary background; use a token utility (bg-card, bg-primary)" },
  { pattern: /\btext-\[/, why: "an arbitrary colour or size; use a token utility (text-muted, text-md)" },
  { pattern: /#[0-9a-fA-F]{3,8}\b/, why: "a literal colour; add a role in packages/design instead" },
];

const files = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) return files(full);
    if (EXEMPT.has(entry.name)) return [];
    return /\.(ts|tsx|css)$/.test(entry.name) ? [full] : [];
  });

const hits = (pattern: RegExp, sources: readonly string[]): string[] =>
  sources
    .flatMap((file) =>
      readFileSync(file, "utf8")
        .split("\n")
        .map((text, index) => ({ file: relative(SRC, file), line: index + 1, text }))
        .filter((row) => pattern.test(row.text)),
    )
    .map((row) => `${row.file}:${row.line}: ${row.text.trim()}`);

describe("no raw colours outside the design package", () => {
  it.each(BANNED)("finds no $why", ({ pattern }) => {
    expect(hits(pattern, files(SRC))).toEqual([]);
  });
});

/**
 * The same idea one step further, and only on the screens.
 *
 * A number in a class list is the other half of the leak the three patterns
 * above catch: `p-[13px]`, `w-[240px]`, `gap-[0.375rem]` all render, none of
 * them is on the scale, and nothing fails. Screens are held to it and the kit
 * is not, because the kit is where a size that the scale has no name for is
 * allowed to exist once, with a comment, rather than a hundred times across
 * the screens.
 *
 * `dark:` is banned everywhere. Tailwind's own dark variant answers to the
 * machine's preference, so a `dark:` utility fires on a dark laptop whose
 * shop chose the light theme and paints one element from the wrong theme.
 * A theme here is a block of variables and the switch is one attribute; a
 * second way to say "dark" is the theme-conditional code `theme.test.ts`
 * already refuses in its other forms.
 */
const SCREEN_BANNED: readonly { readonly pattern: RegExp; readonly why: string }[] = [
  {
    pattern: /-\[[0-9.]+(px|rem|em)\]/,
    why: "a hardcoded size; widen the scale in packages/design or use a spacing utility",
  },
];

describe("no hardcoded sizes on a screen", () => {
  const screens = files(join(SRC, "routes"));

  it.each(SCREEN_BANNED)("finds no $why", ({ pattern }) => {
    expect(hits(pattern, screens)).toEqual([]);
  });
});

describe("no second way to say dark", () => {
  it("uses no dark: variant anywhere", () => {
    // Preceded by a quote, a space or a backtick so the word "dark" in a
    // sentence cannot report, and followed by the start of a utility.
    expect(hits(/["'`\s]dark:[a-z[-]/, files(SRC))).toEqual([]);
  });
});
