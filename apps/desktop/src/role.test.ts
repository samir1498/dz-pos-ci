import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

import { describe, expect, it } from "vitest";

import { ROLE_LABEL } from "@/components/UserMenu";
import ar from "@/i18n/ar.json";
import en from "@/i18n/en.json";
import fr from "@/i18n/fr.json";

/**
 * The same rule `theme.test.ts` holds themes to, held here for roles: a
 * role is a single value that reaches a screen once (`me.role`, or its
 * label) and never a branch. `UserMenu.tsx` is the one screen that reads
 * it at all today, through `ROLE_LABEL[me.role]`, a lookup and not a
 * comparison; the milestone's own text (`m4-common.md`) asks for a grep
 * test that holds every other screen to the same thing, the way
 * `theme.test.ts` already does for a theme name. `RoleDto` (generated,
 * `packages/shared/src/generated/RoleDto.ts`) carries no runtime export the
 * way `@dzpos/design` exports `THEMES`, so the three names are spelled
 * once here instead.
 */
const ROLES = ["owner", "manager", "cashier"] as const;

// process.cwd(), not import.meta.url: these run under jsdom, where
// import.meta.url is an http:// URL and fileURLToPath refuses it.
const SRC = join(process.cwd(), "src");

/**
 * The files that name a role on purpose: the switcher that carries the
 * label, this test, which has to spell what it bans, and `session.test.tsx`,
 * whose `MeDto` fixtures need a real `role` value the same way a fixture
 * needs a real `name` — data, never a comparison (`session.test.tsx` builds
 * two different people to prove the query cache does not survive the
 * change between them; which two roles they hold is incidental to that).
 * Named one by one rather than "any test file", so a branch cannot hide in
 * a test either.
 */
const ALLOWED: ReadonlySet<string> = new Set([
  join("components", "UserMenu.tsx"),
  join("lib", "session.test.tsx"),
  "role.test.ts",
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

describe("a role is a label, not a branch", () => {
  it("names no role outside the topbar's lookup", () => {
    const names = ROLES.join("|");
    expect(offenders(new RegExp(`["'\`](${names})["'\`]`))).toEqual([]);
  });

  /**
   * The shape a branch takes even when the name is held in a constant: a
   * ternary or a comparison against `me.role` (or any variable simply
   * called `role`).
   */
  it("compares nothing against a role", () => {
    expect(offenders(/\brole\s*(===|!==)\s*["'`]/)).toEqual([]);
  });

  it("switches on nothing called role either", () => {
    expect(offenders(/\bswitch\s*\([^)]*\brole\b[^)]*\)/)).toEqual([]);
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
