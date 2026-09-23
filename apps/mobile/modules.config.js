// Which trade a mobile build carries, in one place (C7 of
// `the-first-clinic-module-patients-queue-appointments`): `app.config.js`
// reads `routerRootFor` to pick the Expo Router root directory, and
// `tests/lib/modules.test.ts` reads the rest to prove every top-level route
// file under both `app/` and `app-clinic/` is accounted for.
//
// Unlike the desktop switch (`apps/desktop/modules.config.mjs`), which
// drops a route file from one shared `src/routes` tree with a router-
// generator ignore pattern, Expo Router's own file discovery
// (`expo-router/_ctx.js`) hardcodes its match regex, and Metro's
// `require.context` needs that regex to be a literal `RegExp` node
// (`metro/src/ModuleGraph/worker/collectDependencies.js`'s
// `getRequireContextArgs`, which only accepts an `argNode.type ===
// "RegExpLiteral"` for the filter argument) — so a per-file exclusion list
// like the desktop's cannot drive it directly. What Expo Router does expose
// is a whole-directory swap: the `root` option the `expo-router` config
// plugin validates (`expo-router/plugin/options.json`, "changes the routes
// directory from `app` to another value") and `@expo/cli` reads back as
// `exp.extra.router.root`
// (`@expo/cli/build/src/start/server/metro/router.js`) to compute the
// directory Metro's `require.context` walks. So a clinic build points the
// whole router at `app-clinic/` instead of filtering `app/`: one directory
// per module that owns a screen the other does not, sharing everything
// else through thin re-exports of `screens/`.
//
// The catch this leaves open: the two roots do not compose. Retail wins
// when both are named (`DINAR_MOBILE_MODULES=retail,clinic` builds
// `app/`, same tie-break as the desktop switch's `homeRoute`), because
// there is only "the shop's root" and "the cabinet's root," not a third
// root carrying both trades' screens at once. A phone that needs both
// would need that third root built by hand, the same way a third module
// on the desktop needs a line in `MODULE_ROUTES`.

/** @typedef {"retail" | "clinic"} Module */

/** The Expo Router root each module builds from.
 *  @type {Readonly<Record<Module, string>>} */
const MODULE_ROOTS = {
  retail: "./app",
  clinic: "./app-clinic",
};

/** Every top-level route basename (posix-relative to its own root) a
 *  module's root carries that no other root does.
 *  @type {Readonly<Record<Module, readonly string[]>>} */
const MODULE_ROUTES = {
  retail: ["(signed-in)/till.tsx"],
  clinic: [],
};

/** Present in every root, though not always byte-identical: `index.tsx`
 *  and `sign-in.tsx` differ by the one literal each root's own copy passes
 *  as the signed-in landing (`screens/Index.tsx`, `screens/SignIn.tsx`);
 *  the rest re-export a `screens/` file unchanged.
 *  @type {readonly string[]} */
const SHARED_ROUTES = [
  "_layout.tsx",
  "index.tsx",
  "pair.tsx",
  "sign-in.tsx",
  "(signed-in)/_layout.tsx",
  "(signed-in)/settings.tsx",
];

/** @type {readonly Module[]} */
const ALL_MODULES = /** @type {readonly Module[]} */ (Object.keys(MODULE_ROOTS));

const DEFAULT_MODULES = "retail";

/** `DINAR_MOBILE_MODULES=clinic` -> `["clinic"]`. Blank, unset or a name
 *  this file has never heard of falls back to `DEFAULT_MODULES` alone, the
 *  same shape as the desktop's `resolveModules`.
 *  @param {string | undefined} raw
 *  @returns {readonly Module[]} */
function resolveModules(raw) {
  const wanted = (raw ?? "")
    .split(",")
    .map((name) => name.trim())
    .filter((name) => name !== "");
  const known = wanted.filter((name) => ALL_MODULES.includes(/** @type {Module} */ (name)));
  return known.length === 0 ? [DEFAULT_MODULES] : /** @type {readonly Module[]} */ (known);
}

/** The router root a build carrying `active` points at. Retail wins when
 *  both are named, the way the shop's own home always has.
 *  @param {readonly Module[]} active
 *  @returns {string} */
function routerRootFor(active) {
  return active.includes("retail") ? MODULE_ROOTS.retail : MODULE_ROOTS.clinic;
}

module.exports = {
  MODULE_ROOTS,
  MODULE_ROUTES,
  SHARED_ROUTES,
  ALL_MODULES,
  DEFAULT_MODULES,
  resolveModules,
  routerRootFor,
};
