// `modules.config.mjs` decides which route files a build carries (C6 of
// `the-first-clinic-module-patients-queue-appointments`). Two things break
// silently if this drifts: a module added to `MODULE_ROUTES` with the wrong
// name never gets excluded, and a route file added under `src/routes` and
// left out of every list here builds into every trade whether it should or
// not. The classification test below is what catches the second one; it
// reads the real directory, so it goes red the moment a file is added
// without a line in this page.

import { readdirSync } from "node:fs";
import path from "node:path";

import { describe, expect, test } from "vitest";

import {
  ALL_MODULES,
  MODULE_ROUTES,
  SHARED_ROUTES,
  excludedRouteFiles,
  resolveModules,
  routeIgnorePatternSource,
} from "../modules.config.mjs";

// `vitest.config.ts` carries no `root` override, so the working directory
// is `apps/desktop` for every run, `just gates` included.
const routesDir = path.join(process.cwd(), "src", "routes");

/** Every top-level route file the generator can see: not a `-`-prefixed
 *  folder or file (the plugin's own ignore prefix already drops those), not
 *  a directory, not the generated tree itself. */
function topLevelRouteFiles(): string[] {
  return readdirSync(routesDir, { withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => entry.name)
    .filter((name) => !name.startsWith("-") && !name.startsWith("."))
    .filter((name) => name !== "routeTree.gen.ts");
}

describe("classification: every route file belongs to exactly one place", () => {
  test("no file under src/routes is unclassified or double-classified", () => {
    const owned = new Map<string, string>();
    for (const name of SHARED_ROUTES) owned.set(name, "shared");
    for (const module of ALL_MODULES) {
      for (const name of MODULE_ROUTES[module]) {
        const already = owned.get(name);
        expect(already, `"${name}" is claimed by both ${already} and ${module}`).toBeUndefined();
        owned.set(name, module);
      }
    }

    for (const file of topLevelRouteFiles()) {
      expect(owned.has(file), `"${file}" is in neither SHARED_ROUTES nor MODULE_ROUTES`).toBe(
        true,
      );
    }
  });

  test("every name MODULE_ROUTES and SHARED_ROUTES list is a file that exists", () => {
    const present = new Set(topLevelRouteFiles());
    for (const name of SHARED_ROUTES) {
      expect(present.has(name), `SHARED_ROUTES names "${name}", which is not on disk`).toBe(true);
    }
    for (const module of ALL_MODULES) {
      for (const name of MODULE_ROUTES[module]) {
        expect(present.has(name), `${module} names "${name}", which is not on disk`).toBe(true);
      }
    }
  });
});

describe("resolveModules: VITE_DINAR_MODULES to the list a build carries", () => {
  test.each<[string | undefined, readonly string[]]>([
    [undefined, ["retail"]],
    ["", ["retail"]],
    ["retail", ["retail"]],
    ["clinic", ["clinic"]],
    ["retail,clinic", ["retail", "clinic"]],
    [" retail , clinic ", ["retail", "clinic"]],
    ["yoga-studio", ["retail"]],
  ])("resolveModules(%o) -> %o", (raw, expected) => {
    expect(resolveModules(raw)).toEqual(expected);
  });
});

describe("excludedRouteFiles: the other trade's files, none of its own", () => {
  test("retail alone excludes the clinic's two files and none of its own", () => {
    const excluded = excludedRouteFiles(["retail"]);
    expect(excluded).toEqual(expect.arrayContaining(["patients.tsx", "queue.tsx"]));
    expect(excluded).not.toEqual(expect.arrayContaining(["customers.tsx", "till.tsx"]));
  });

  test("clinic alone excludes every retail file and none of its own", () => {
    const excluded = excludedRouteFiles(["clinic"]);
    expect(excluded).toEqual(expect.arrayContaining([...MODULE_ROUTES.retail]));
    expect(excluded).not.toEqual(expect.arrayContaining(["patients.tsx", "queue.tsx"]));
  });

  test("both trades built excludes nothing", () => {
    expect(excludedRouteFiles(["retail", "clinic"])).toEqual([]);
  });
});

describe("routeIgnorePatternSource: the string the router-generator matches a basename against", () => {
  test("null when nothing is excluded", () => {
    expect(routeIgnorePatternSource(["retail", "clinic"])).toBeNull();
  });

  test("matches an excluded basename and nothing else", () => {
    const source = routeIgnorePatternSource(["retail"]);
    expect(source).not.toBeNull();
    const pattern = new RegExp(source ?? "");
    expect("patients.tsx").toMatch(pattern);
    expect("queue.tsx").toMatch(pattern);
    expect("customers.tsx").not.toMatch(pattern);
    // A basename containing an excluded one as a substring must not match:
    // the pattern anchors start and end, or "queue.tsx" would also catch a
    // hypothetical "queue.test.tsx".
    expect("not-queue.tsx").not.toMatch(pattern);
    // The other side of the same anchor: dropping the trailing `$` would
    // let "queue.tsx" match the start of "queue.tsx.bak" too, excluding a
    // file that was never in `MODULE_ROUTES`.
    expect("queue.tsx.bak").not.toMatch(pattern);
  });
});
