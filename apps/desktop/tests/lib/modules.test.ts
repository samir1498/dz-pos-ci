// `src/lib/modules.ts` is the bundle's own copy of the default resolution
// `modules.config.mjs` makes for the router plugin (it cannot import that
// file: nothing under `src/` reaches outside it, and vitest never loads
// `vite.config.ts`). This pins the one thing that must never drift between
// the two: unset, blank or an unknown name falls back to "retail" alone.

import { afterEach, describe, expect, test, vi } from "vitest";

import { builtModules, isModuleBuilt } from "@/lib/modules";

afterEach(() => {
  vi.unstubAllEnvs();
});

describe("builtModules: VITE_DINAR_MODULES to the trades this bundle carries", () => {
  test("unset falls back to retail alone", () => {
    vi.stubEnv("VITE_DINAR_MODULES", undefined);
    expect(builtModules()).toEqual(["retail"]);
  });

  test("blank falls back to retail alone", () => {
    vi.stubEnv("VITE_DINAR_MODULES", "");
    expect(builtModules()).toEqual(["retail"]);
  });

  test("a name this app has never heard of falls back to retail alone", () => {
    vi.stubEnv("VITE_DINAR_MODULES", "yoga-studio");
    expect(builtModules()).toEqual(["retail"]);
  });

  test("clinic alone carries clinic and not retail", () => {
    vi.stubEnv("VITE_DINAR_MODULES", "clinic");
    expect(builtModules()).toEqual(["clinic"]);
  });

  test("both trades, comma separated with spaces", () => {
    vi.stubEnv("VITE_DINAR_MODULES", " retail , clinic ");
    expect(builtModules()).toEqual(["retail", "clinic"]);
  });
});

describe("isModuleBuilt: one module asked for at a time", () => {
  test("true for a module in the built list, false for one left out", () => {
    vi.stubEnv("VITE_DINAR_MODULES", "clinic");
    expect(isModuleBuilt("clinic")).toBe(true);
    expect(isModuleBuilt("retail")).toBe(false);
  });
});
