import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

import { describe, expect, it } from "vitest";

import { ROLE_LABEL, ROLES } from "@/lib/roles";
import ar from "@/i18n/ar.json";
import en from "@/i18n/en.json";
import fr from "@/i18n/fr.json";

/**
 * The same rule `theme.test.ts` holds themes to, held here for roles: a
 * role is a single value that reaches a screen once (`me.role`, or its
 * label) and never a branch. Two screens read one at all: the topbar shows
 * its label, and the staff screen offers the list when a person is added.
 * Both go through `lib/roles.ts`, a lookup and not a comparison, and that
 * is also where the three names are written down, so this test reads them
 * rather than spelling a second copy that could drift from the first.
 */

// process.cwd(), not import.meta.url: these run under jsdom, where
// import.meta.url is an http:// URL and fileURLToPath refuses it.
const SRC = join(process.cwd(), "src");

/**
 * The files that name a role on purpose: the switcher that carries the
 * label, this test, which has to spell what it bans, and the tests whose
 * fixtures need a real `role` value the same way a fixture needs a real
 * `name` — data, never a comparison.
 *
 * `lib/roles.ts` is where the three names are written down, once, for the
 * whole app. `session.test.tsx` builds two different people to prove the
 * query cache does not survive the change between them; which two roles
 * they hold is incidental. `AppShell.test.tsx` answers `/auth/me` with
 * somebody, and a person has a role. `audit.test.tsx` spells one inside the
 * `after` of an audit row, which is the log recording that a person's role
 * was changed: the name there is the thing being logged, not a decision the
 * screen takes. `settings_.users.test.tsx` builds the staff a staff screen
 * lists, and staff have roles.
 *
 * Named one by one rather than "any test file", so a branch cannot hide in
 * a test either. Adding a file here is a claim that its role name is data;
 * the comparison test below still covers every file, this one included.
 */
const ALLOWED: ReadonlySet<string> = new Set([
  join("lib", "roles.ts"),
  join("components", "AppShell.test.tsx"),
  join("lib", "session.test.tsx"),
  join("routes", "audit.test.tsx"),
  join("routes", "settings_.users.test.tsx"),
  "role.test.ts",
]);

const sources = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) return sources(full);
    return /\.(ts|tsx)$/.test(entry.name) ? [full] : [];
  });

/** This file spells every pattern it bans, so it is the one exemption the
 *  comparison rules also take. */
const SELF = "role.test.ts";

const offenders = (pattern: RegExp, exempt: ReadonlySet<string>): string[] =>
  sources(SRC)
    .filter((file) => !exempt.has(relative(SRC, file)))
    .flatMap((file) =>
      readFileSync(file, "utf8")
        .split("\n")
        .map((text, index) => ({ file: relative(SRC, file), line: index + 1, text }))
        .filter((row) => pattern.test(row.text))
        .map((row) => `${row.file}:${row.line}: ${row.text.trim()}`),
    );

describe("a role is a label, not a branch", () => {
  it("names no role outside the topbar's lookup", () => {
    const names = ROLES.join("|");
    expect(offenders(new RegExp(`["'\`](${names})["'\`]`), ALLOWED)).toEqual([]);
  });

  /**
   * The shape a branch takes even when the name is held in a constant: a
   * ternary or a comparison against `me.role` (or any variable simply
   * called `role`).
   */
  it("compares nothing against a role", () => {
    expect(offenders(/\brole\s*(===|!==)\s*["'`]/, new Set([SELF]))).toEqual([]);
  });

  it("switches on nothing called role either", () => {
    expect(offenders(/\bswitch\s*\([^)]*\brole\b[^)]*\)/, new Set([SELF]))).toEqual([]);
  });
});

/**
 * The one place allowed to say "Propriétaire" or "Caissier" out loud, so
 * the labels are checked where the names already live, the same shape as
 * `theme.test.ts`'s label test: every role `ROLE_LABEL` can be asked to
 * show has something to show, in all three dictionaries.
 */
describe("every role's label", () => {
  it.each(ROLES)("is in fr, en and ar for %s", (name) => {
    const key = ROLE_LABEL[name];
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
