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
 * lists, and staff have roles. `test/session.ts` (M4 T5) is the one
 * `MeDto` fixture every other screen test's `SessionProvider` renders
 * behind; the two roles on it are the same kind of data.
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
  join("test", "session.ts"),
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
 * M4 T5's own rule, the same shape as the role one above: a screen asks
 * `useHasPermission` (or, for the one place that has to check several
 * permissions against one `me` without breaking the rule that a hook runs
 * the same number of times on every render, the plain `hasPermission`
 * function) and never reads `me.permissions` itself. Both live in
 * `lib/session.tsx`, which is the one place this pattern is data rather
 * than a second way to ask. `AppShell.tsx`'s sidebar used to read
 * `me.permissions.includes(...)` inline before it took `hasPermission`,
 * which is exactly what this test would have caught.
 */
describe("a permission is asked once", () => {
  it("reads `.permissions` only in lib/session.tsx", () => {
    expect(
      offenders(/\.permissions\b/, new Set([join("lib", "session.tsx"), SELF])),
    ).toEqual([]);
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

/**
 * The server sends a code and the UI owns the wording, which only works if
 * there is one place that turns a code into wording. There were two until
 * 2026-09-11: `lib/fields.tsx` and a private copy inside `routes/till.tsx`,
 * and they had drifted in both directions. The till's knew about a credit
 * limit and a barcode already taken; the shared one did not. The shared one
 * knew about a refused permission; the till's did not. So a cashier refused
 * a discount at the till, which is the exact refusal this milestone exists
 * to produce, was shown "something went wrong".
 *
 * The first cashier spec in the browser suite is what found it, and this is
 * what stops the second copy coming back. The same shape as the role rule
 * above and for the same reason: a second copy of a table is not a bug on
 * the day it is written, it is a bug on the day one of them is updated.
 */
describe("a server error code becomes wording in one place", () => {
  it("declares no second error map", () => {
    const found = offenders(/\bconst ERROR_KEY\s*:\s*Record<string,\s*Key>/, new Set([SELF, "lib/fields.tsx"]));
    expect(found).toEqual([]);
  });

  it("writes no second errorKey function", () => {
    const found = offenders(/function\s+errorKey\s*\(/, new Set([SELF, "lib/fields.tsx"]));
    expect(found).toEqual([]);
  });
});
