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

describe("no raw colours outside the design package", () => {
  it.each(BANNED)("finds no $why", ({ pattern }) => {
    const offenders = files(SRC)
      .flatMap((file) =>
        readFileSync(file, "utf8")
          .split("\n")
          .map((text, index) => ({ file: relative(SRC, file), line: index + 1, text }))
          .filter((row) => pattern.test(row.text)),
      )
      .map((row) => `${row.file}:${row.line}: ${row.text.trim()}`);
    expect(offenders).toEqual([]);
  });
});
