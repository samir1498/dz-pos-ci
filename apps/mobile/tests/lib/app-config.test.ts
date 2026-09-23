// `app.config.js` is the one place `DINAR_MOBILE_MODULES` actually decides
// which Expo Router root a build gets (C7 of
// `the-first-clinic-module-patients-queue-appointments`): `modules.config.js`'s
// own tests (`modules.test.ts`) pin `routerRootFor` in isolation, but
// nothing before this file called `app.config.js` itself, so a typo
// wiring `routerRootFor(active)` into the wrong plugin option, or a
// hardcoded `root` that never reads `active` at all, would have shipped
// unnoticed. Checked red first: hardcoding `root: "./app"` in
// `app.config.js` (ignoring `routerRootFor`) failed every case below
// except the one it happens to agree with.
//
// `app.config.js` reads `process.env.DINAR_MOBILE_MODULES` inside the
// function Expo calls, not at module load time, so `vi.stubEnv` before
// each call is enough — no module cache to bust, unlike a value Vite
// inlines at build time (`src/lib/modules.ts` on desktop).

import { afterEach, describe, expect, test, vi } from "vitest";

import loadConfig from "../../app.config";

afterEach(() => {
  vi.unstubAllEnvs();
});

/** The `["expo-router", { root }]` tuple `app.config.js` builds, found by
 *  name rather than by position: `plugins` also carries `expo-camera` and
 *  `expo-splash-screen` tuples, and a reorder should not break this test. */
function expoRouterPlugin(config: ReturnType<typeof loadConfig>): unknown {
  return config.expo.plugins.find((plugin) => Array.isArray(plugin) && plugin[0] === "expo-router");
}

describe("app.config.js: DINAR_MOBILE_MODULES to the expo-router plugin's root", () => {
  test("unset falls back to the retail root", () => {
    vi.stubEnv("DINAR_MOBILE_MODULES", undefined);
    expect(expoRouterPlugin(loadConfig())).toEqual(["expo-router", { root: "./app" }]);
  });

  test("clinic alone points at the clinic root", () => {
    vi.stubEnv("DINAR_MOBILE_MODULES", "clinic");
    expect(expoRouterPlugin(loadConfig())).toEqual(["expo-router", { root: "./app-clinic" }]);
  });

  test("both named: retail wins, the retail root", () => {
    vi.stubEnv("DINAR_MOBILE_MODULES", "retail,clinic");
    expect(expoRouterPlugin(loadConfig())).toEqual(["expo-router", { root: "./app" }]);
  });
});
