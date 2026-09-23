import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

import { describe, expect, it } from "vitest";

/**
 * The same shape as `role.test.ts` and `theme.test.ts`: a rule the whole of
 * `src/` is held to, checked by reading the source rather than by hoping
 * the next person remembers.
 *
 * The rule: a question the shop has to answer is asked in a dialog of ours,
 * never in the box the browser draws. `window.confirm` puts the sentence
 * through the dictionary and leaves the two buttons and the frame to
 * Windows, which writes them in the language Windows is in and lays them
 * out left to right on an Arabic till. `suppliers.tsx` wrote that down in
 * 2026 and answered it with a dialog of its own; the till's credit override
 * and the customer ledger's correction kept asking the old way until
 * 2026-09-12, and `components/ConfirmDialog.tsx` is the one answer now.
 *
 * `alert` and `prompt` are banned with it. Neither is used today and both
 * are the same box.
 */

// process.cwd(), not import.meta.url: these run under jsdom, where
// import.meta.url is an http:// URL and fileURLToPath refuses it.
const SRC = join(process.cwd(), "src");

/** This file spells what it bans, so it is the one exemption. */
const SELF = "confirm.test.ts";

const sources = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) return sources(full);
    return /\.(ts|tsx)$/.test(entry.name) ? [full] : [];
  });

/** Every line that matches, named by file and line so a failure says where. */
const offenders = (pattern: RegExp): string[] =>
  sources(SRC)
    .filter((file) => relative(SRC, file) !== SELF)
    .flatMap((file) =>
      readFileSync(file, "utf8")
        .split("\n")
        .map((text, index) => ({ file: relative(SRC, file), line: index + 1, text }))
        .filter((row) => pattern.test(row.text))
        .map((row) => `${row.file}:${row.line}: ${row.text.trim()}`),
    );

describe("the shop is asked in the app's own words", () => {
  it("calls no browser confirm box", () => {
    expect(offenders(/\bwindow\.confirm\s*\(/)).toEqual([]);
  });

  it("calls no browser alert or prompt either", () => {
    expect(offenders(/\bwindow\.(alert|prompt)\s*\(/)).toEqual([]);
  });
});
