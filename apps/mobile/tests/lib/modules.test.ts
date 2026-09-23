// `modules.config.js` decides which Expo Router root a build points at
// (C7 of `the-first-clinic-module-patients-queue-appointments`). Two things
// break silently if this drifts: a module whose root name is misspelled
// never gets picked, and a route file added under `app/` or `app-clinic/`
// and left out of `SHARED_ROUTES`/`MODULE_ROUTES` builds unnoticed into a
// root it should not be in (or is missing from a root it should). The
// classification test below reads both real directories, so it goes red
// the moment a file is added without a line here — checked by hand before
// this file existed: adding a stray `app-clinic/(signed-in)/till.tsx` red
// against "no file in any root is unclassified", and removing
// `(signed-in)/till.tsx` from `MODULE_ROUTES.retail` red against "a
// module's own route is present only in its own root", exactly as each
// should.

import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";

import ts from "typescript";
import { describe, expect, test } from "vitest";

import {
  ALL_MODULES,
  MODULE_ROOTS,
  MODULE_ROUTES,
  SHARED_ROUTES,
  resolveModules,
  routerRootFor,
} from "../../modules.config";

// `vitest.config.ts` carries no `root` override, so the working directory
// is `apps/mobile` for every run, `just gates` included.
const projectRoot = process.cwd();

/** Every route file a root carries, as a posix path relative to that root
 *  (`_layout.tsx`, `(signed-in)/settings.tsx`): recurse into real
 *  directories (Expo Router nests routes in folders, unlike the desktop's
 *  flat `src/routes`), skip `node_modules` and any dotfile, keep only
 *  `.ts`/`.tsx`. */
function routeFiles(root: string): string[] {
  const absolute = path.join(projectRoot, root);
  const out: string[] = [];
  const walk = (dir: string, prefix: string) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (entry.name.startsWith(".") || entry.name === "node_modules") continue;
      const relative = prefix === "" ? entry.name : `${prefix}/${entry.name}`;
      if (entry.isDirectory()) {
        walk(path.join(dir, entry.name), relative);
      } else if (/\.tsx?$/.test(entry.name)) {
        out.push(relative);
      }
    }
  };
  walk(absolute, "");
  return out;
}

/** Every `.ts`/`.tsx` file under `root`, absolute, recursing into real
 *  directories. Shared with the import-graph walk below and its positive
 *  control, which both need to start from every file a root carries rather
 *  than a hand-picked list. */
function tsxFilesUnder(root: string): string[] {
  const absolute = path.join(projectRoot, root);
  const out: string[] = [];
  const walk = (dir: string) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (entry.name.startsWith(".") || entry.name === "node_modules") continue;
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (/\.tsx?$/.test(entry.name)) out.push(full);
    }
  };
  walk(absolute);
  return out;
}

/** Posix-relative to the project root, so a path reads the same as the
 *  literals this file compares it against regardless of platform. */
function relPosix(file: string): string {
  return path.relative(projectRoot, file).split(path.sep).join("/");
}

describe("classification: every route file in every root belongs to exactly one place", () => {
  test("a shared name is present in every root", () => {
    for (const module of ALL_MODULES) {
      const present = new Set(routeFiles(MODULE_ROOTS[module]));
      for (const name of SHARED_ROUTES) {
        expect(present.has(name), `${module}'s root is missing shared route "${name}"`).toBe(
          true,
        );
      }
    }
  });

  test("a module's own route is present only in its own root", () => {
    for (const owner of ALL_MODULES) {
      for (const name of MODULE_ROUTES[owner]) {
        for (const module of ALL_MODULES) {
          const present = routeFiles(MODULE_ROOTS[module]).includes(name);
          if (module === owner) {
            expect(present, `${owner} names "${name}", which is not in its own root`).toBe(true);
          } else {
            expect(present, `"${name}" is ${owner}'s but is also in ${module}'s root`).toBe(
              false,
            );
          }
        }
      }
    }
  });

  test("no file in any root is unclassified", () => {
    const owned = new Map<string, string>();
    for (const name of SHARED_ROUTES) owned.set(name, "shared");
    for (const module of ALL_MODULES) {
      for (const name of MODULE_ROUTES[module]) owned.set(name, module);
    }

    for (const module of ALL_MODULES) {
      for (const file of routeFiles(MODULE_ROOTS[module])) {
        expect(
          owned.has(file),
          `"${file}" in ${module}'s root (${MODULE_ROOTS[module]}) is in neither SHARED_ROUTES nor MODULE_ROUTES`,
        ).toBe(true);
      }
    }
  });
});

describe("resolveModules: DINAR_MOBILE_MODULES to the list a build carries", () => {
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

// A first version of the leak check listed `screens/**` minus `Till.tsx`
// plus `app-clinic/**` and grepped each file's own imports for
// `features/till`/`lib/basket`. It missed a real path: `Settings.tsx`
// (shared, so in `app-clinic/`'s tree by way of `screens/RootLayout.tsx`
// mounting `CartProvider`) could re-export the till's code under a name
// that never mentions "till" (`export { priceBasket as computeTotal } from
// "../lib/basket"`), and a screen that imports that name would carry the
// money math into a clinic build without ever importing `lib/basket`
// itself. What actually decides whether a clinic build carries the till's
// code is the transitive closure of what `app-clinic/`'s own files import,
// followed through every re-export, not each file read in isolation.
//
// Walked with the TypeScript compiler rather than matched with a regular
// expression, the same reason `no-literal-strings.test.ts` parses instead
// of scanning text: a regex over source text cannot tell an import
// specifier from a comment that happens to name the same path, and this
// file's own comments do (`features/till`, `lib/basket`, several times
// over) — a plain substring match flagged its own file for saying so
// before this walk was written to read only import and export specifiers.
//
// `import type`/`export type` and an all-type named clause are skipped
// because they are erased before the bundle exists: `CartProvider.tsx`
// imports `CartLine`/`Product` from `../lib/basket` as types only, and
// following that edge would fail every build, retail included, on a line
// that compiles to nothing.
describe("the till's own code is unreachable from a clinic build", () => {
  /** Whether an import clause carries a binding still in the code once
   *  types are erased: a bare `import "x"` has no clause and is always
   *  runtime, `import type … from "x"` is never, a default or namespace
   *  binding always is, and a named-imports clause is runtime the moment
   *  one of its elements is not itself marked `type`. */
  function importClauseIsRuntime(clause: ts.ImportClause | undefined): boolean {
    if (clause === undefined) return true;
    if (clause.isTypeOnly) return false;
    if (clause.name !== undefined) return true;
    const bindings = clause.namedBindings;
    if (bindings === undefined) return false;
    if (ts.isNamespaceImport(bindings)) return true;
    return bindings.elements.some((element) => !element.isTypeOnly);
  }

  /** Same question for a re-export: `export type { … } from "x"` and a
   *  named-exports clause that is every element `type` do not run;
   *  `export * from "x"`, `export * as ns from "x"` and a mixed named
   *  clause do. */
  function exportClauseIsRuntime(node: ts.ExportDeclaration): boolean {
    if (node.isTypeOnly) return false;
    const clause = node.exportClause;
    if (clause === undefined) return true;
    if (ts.isNamespaceExport(clause)) return true;
    return clause.elements.some((element) => !element.isTypeOnly);
  }

  /** The specifiers a file's own imports and re-exports name, plus
   *  whether a dynamic `import()`/`require()` in it named something other
   *  than a plain string literal — a walk cannot follow that, and would
   *  rather fail loudly on it than silently skip it. */
  function specifiersOf(file: string): { specifiers: string[]; opaqueCall: boolean } {
    const source = ts.createSourceFile(
      file,
      readFileSync(file, "utf8"),
      ts.ScriptTarget.Latest,
      true,
    );
    const specifiers: string[] = [];
    let opaqueCall = false;
    const visit = (node: ts.Node) => {
      if (ts.isImportDeclaration(node) && ts.isStringLiteral(node.moduleSpecifier)) {
        if (importClauseIsRuntime(node.importClause)) specifiers.push(node.moduleSpecifier.text);
      } else if (
        ts.isExportDeclaration(node) &&
        node.moduleSpecifier !== undefined &&
        ts.isStringLiteral(node.moduleSpecifier)
      ) {
        if (exportClauseIsRuntime(node)) specifiers.push(node.moduleSpecifier.text);
      } else if (
        ts.isCallExpression(node) &&
        (node.expression.kind === ts.SyntaxKind.ImportKeyword ||
          (ts.isIdentifier(node.expression) && node.expression.text === "require"))
      ) {
        const argument = node.arguments[0];
        if (argument !== undefined && ts.isStringLiteral(argument)) {
          specifiers.push(argument.text);
        } else if (argument !== undefined) {
          opaqueCall = true;
        }
      }
      ts.forEachChild(node, visit);
    };
    visit(source);
    return { specifiers, opaqueCall };
  }

  /** A relative or `@/`-aliased specifier resolved to the file on disk it
   *  names, trying the bare path, `.ts`, `.tsx`, `/index.ts` and
   *  `/index.tsx` in that order (`tsconfig.json`'s `@/*` -> `./*`, the
   *  same resolution `tsc` itself does). `null` for a bare specifier
   *  (`"react"`, `"@dzpos/shared"`, `"expo-router"`): a package this walk
   *  does not own and is not asked to follow. `undefined` for a
   *  relative/`@/` specifier that resolves to nothing — a caller treats
   *  that as a failure, not a skip, since a silent skip here is exactly
   *  the hole this walk exists to close. */
  function resolveSpecifier(fromFile: string, specifier: string): string | null | undefined {
    let base: string;
    if (specifier.startsWith(".")) {
      base = path.resolve(path.dirname(fromFile), specifier);
    } else if (specifier.startsWith("@/")) {
      base = path.join(projectRoot, specifier.slice(2));
    } else {
      return null;
    }
    const candidates = [base, `${base}.ts`, `${base}.tsx`, `${base}/index.ts`, `${base}/index.tsx`];
    for (const candidate of candidates) {
      if (existsSync(candidate) && statSync(candidate).isFile()) return candidate;
    }
    return undefined;
  }

  /** Every file reachable from `startFiles` by a runtime import or
   *  re-export, transitively. Throws rather than returning a partial set
   *  on an unresolvable specifier or an opaque dynamic call, since either
   *  one means the walk cannot say what is and is not reachable. */
  function walkImportGraph(startFiles: readonly string[]): Set<string> {
    const visited = new Set<string>();
    const queue = [...startFiles];
    while (queue.length > 0) {
      const file = queue.shift();
      if (file === undefined || visited.has(file)) continue;
      visited.add(file);
      const { specifiers, opaqueCall } = specifiersOf(file);
      if (opaqueCall) {
        throw new Error(
          `"${relPosix(file)}" has a dynamic import() or require() whose argument is not a string literal; the walk cannot follow it`,
        );
      }
      for (const specifier of specifiers) {
        const resolved = resolveSpecifier(file, specifier);
        if (resolved === null) continue;
        if (resolved === undefined) {
          throw new Error(
            `"${relPosix(file)}" imports "${specifier}", which does not resolve to a file`,
          );
        }
        if (!visited.has(resolved)) queue.push(resolved);
      }
    }
    return visited;
  }

  function isTillCode(relative: string): boolean {
    return relative.startsWith("features/till/") || /^lib\/basket(?:\.tsx?)?$/.test(relative);
  }

  describe("from app-clinic/", () => {
    const walked = walkImportGraph(tsxFilesUnder(MODULE_ROOTS.clinic));
    const relatives = [...walked].map(relPosix);

    // An empty walk would pass every check below by finding nothing to
    // check, the same trap `check-clinic-bundle`'s comment names for its
    // own positive grep: assert the walk actually walked somewhere, and
    // reached the two files it is certain to (the shared root layout every
    // route re-exports, and the cart provider that layout mounts), before
    // trusting it to say nothing else is there.
    test("the walk is not empty and reaches the shared root layout and the cart provider", () => {
      expect(relatives.length).toBeGreaterThan(0);
      expect(relatives).toContain("screens/RootLayout.tsx");
      expect(relatives).toContain("providers/CartProvider.tsx");
    });

    test.each(relatives)("%s is not the till's own code", (relative) => {
      expect(isTillCode(relative), `"${relative}" is reachable from app-clinic/`).toBe(false);
    });
  });

  // The positive control: the identical walk from the retail root must
  // reach the till's code, or the walk itself — not the absence of a
  // leak — could be why the clinic side above found nothing. Mirrors
  // `check-clinic-bundle`'s own default-build assertion (the marker string
  // must still be found in an ordinary build) for the same reason.
  describe("from app/, the positive control", () => {
    const walked = walkImportGraph(tsxFilesUnder(MODULE_ROOTS.retail));
    const relatives = [...walked].map(relPosix);

    test("the walk reaches features/till and lib/basket", () => {
      expect(relatives.some((relative) => relative.startsWith("features/till/"))).toBe(true);
      expect(relatives).toContain("lib/basket.ts");
    });
  });
});

describe("routerRootFor: the directory a build's Expo Router points at", () => {
  test("retail alone is the retail root", () => {
    expect(routerRootFor(["retail"])).toBe("./app");
  });

  test("clinic alone, with no retail, is the clinic root", () => {
    expect(routerRootFor(["clinic"])).toBe("./app-clinic");
  });

  test("both named: retail wins, the same tie-break as the desktop's home", () => {
    expect(routerRootFor(["retail", "clinic"])).toBe("./app");
  });
});
