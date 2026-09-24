// Which trade a build carries, in one place: the router-generator's
// `routeFileIgnorePattern` (vite.config.ts) and the tsc pass that has to
// agree with it (scripts/build.mjs) both read this file rather than each
// keeping their own list, which is how the two drifted apart the first time
// this was tried by hand.
//
// Plain JS, not TypeScript: vite.config.ts loads it as an ordinary ESM
// import (Vite transpiles its own config either way) and scripts/build.mjs
// runs it under plain Node with no loader, so a `.ts` file here would need
// one tool or the other to gain a dependency it does not otherwise need.
//
// `src/lib/modules.ts` is the runtime's own, separate copy of the *default*
// resolution (env unset -> "retail"): it runs in the browser bundle, which
// cannot import a file from outside `src/`, and vitest never loads this
// file at all (`vitest.config.ts` carries no router plugin). One default,
// spelled twice, each pinned by its own test.

/** @typedef {"retail" | "clinic"} Module */

/** Every module the desktop knows, and the top-level file under
 *  `src/routes` that belongs to each one. A file left out of every list
 *  below is `SHARED_ROUTES`, and `tests/modules.test.ts` fails the moment a
 *  new file under `src/routes` is in neither, so an added screen cannot
 *  forget to say which trade it is. `-`-prefixed folders (`-patients/`,
 *  `-till/`, ...) never appear here: the router-generator already skips
 *  them by name (`routeFileIgnorePrefix`, the default `-`), and nothing
 *  outside the module that owns a route file imports its `-folder`, so
 *  excluding the route file already drops the whole subtree by ordinary
 *  tree-shaking.
 *
 * @type {Readonly<Record<Module, readonly string[]>>}
 */
export const MODULE_ROUTES = {
  retail: [
    "customers.tsx",
    "customers_.$id.tsx",
    "products.tsx",
    "purchases.tsx",
    "purchases_.$id.tsx",
    "purchases_.new.tsx",
    "suppliers.tsx",
    "suppliers_.$id.tsx",
    "till.tsx",
    "till_.shifts.tsx",
    "dashboard.tsx",
    "documents.tsx",
    "expenses.tsx",
    "settings.regime.tsx",
    "settings.printing.tsx",
    "settings.data.tsx",
  ],
  clinic: ["patients.tsx", "queue.tsx", "book.tsx", "settings.book.tsx"],
};

/** Every route file no build ever drops: the shell, the redirect, the dev
 *  kit page, the audit log and every settings room a trade does not own.
 *  `tests/modules.test.ts` also pins this list, so a route moved out of it
 *  by mistake fails there rather than as a silent 404 in one build only.
 *
 * @type {readonly string[]}
 */
export const SHARED_ROUTES = [
  "__root.tsx",
  "index.tsx",
  "kit.tsx",
  "audit.tsx",
  "settings.tsx",
  "settings.index.tsx",
  "settings.about.tsx",
  "settings.appearance.tsx",
  "settings.backups.tsx",
  "settings.phones.tsx",
  "settings.shop.tsx",
  "settings.users.tsx",
];

/** @type {readonly Module[]} */
export const ALL_MODULES = /** @type {readonly Module[]} */ (Object.keys(MODULE_ROUTES));

export const DEFAULT_MODULES = "retail";

/**
 * `VITE_DINAR_MODULES=retail,clinic` -> `["retail", "clinic"]`. Blank,
 * unset or a name this file has never heard of falls back to
 * `DEFAULT_MODULES` alone, so a typo builds the shop everyone already has
 * rather than a build with nothing in it.
 *
 * @param {string | undefined} raw
 * @returns {readonly Module[]}
 */
export function resolveModules(raw) {
  const wanted = (raw ?? "")
    .split(",")
    .map((name) => name.trim())
    .filter((name) => name !== "");
  const known = wanted.filter((name) => ALL_MODULES.includes(/** @type {Module} */ (name)));
  return known.length === 0 ? [DEFAULT_MODULES] : /** @type {readonly Module[]} */ (known);
}

/**
 * The basenames a build with exactly `active` in it must not compile: every
 * file owned by a module that is not in `active`. Shared files are never in
 * this list.
 *
 * @param {readonly Module[]} active
 * @returns {readonly string[]}
 */
export function excludedRouteFiles(active) {
  return ALL_MODULES.filter((module) => !active.includes(module)).flatMap(
    (module) => MODULE_ROUTES[module],
  );
}

/** A basename regexp source for one `d.name` per directory level, the shape
 *  `@tanstack/router-generator` tests `routeFileIgnorePattern` against
 *  (`getRouteNodes.js`: `d.name.match(new RegExp(pattern, "g"))`, run at
 *  every directory the walk visits, not the whole relative path). `null`
 *  when nothing needs excluding, so a superset build passes `undefined` to
 *  the plugin rather than a pattern that matches nothing but still runs.
 *
 * @param {readonly Module[]} active
 * @returns {string | null}
 */
export function routeIgnorePatternSource(active) {
  const excluded = excludedRouteFiles(active);
  if (excluded.length === 0) return null;
  const escaped = excluded.map((name) => name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  return `^(${escaped.join("|")})$`;
}

/**
 * The cargo-side arguments a Tauri run needs so the Rust binary it launches
 * carries exactly `active`'s modules, no more (whole-loop review: an
 * installed doctor's build had no clinic routes because nothing ever told
 * cargo to build them). `dzpos-desktop`'s own `retail`/`clinic` features
 * forward to `dzpos-api` and `dzpos-core` (`apps/desktop/src-tauri/Cargo.toml`),
 * but the Tauri CLI's own `--features`/`-f` only *adds* to cargo's default
 * set (`tauri build --help`: "list of features to activate"), so a
 * clinic-alone build still needs `--no-default-features` or the shop's
 * `retail` default rides along uninvited. `tauri dev`/`build` do not read
 * either flag as their own -- the pair after `--` is what those commands
 * pass straight through to the `cargo run`/`cargo build` underneath
 * ("[ARGS]... Command line arguments passed to the runner").
 *
 * @param {readonly Module[]} active
 * @returns {readonly string[]}
 */
export function cargoFeatureArgs(active) {
  return ["--", "--no-default-features", "--features", active.join(",")];
}
