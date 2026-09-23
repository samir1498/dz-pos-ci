// Which trades this running window's build actually carries, read from
// `VITE_DINAR_MODULES` at build time (`vite.config.ts` computes the same
// value for the router plugin, from `modules.config.mjs`; this is the
// bundle's own copy, since nothing under `src/` may import a file outside
// it, and vitest never loads `vite.config.ts` at all).
//
// The nav (`AppShell`), the settings rail and the index redirect all read
// this rather than assuming the shop is always retail, so a screen a build
// left out is a screen this app never points at.

export type Module = "retail" | "clinic";

const KNOWN: readonly Module[] = ["retail", "clinic"];

function isModule(name: string): name is Module {
  return KNOWN.some((known) => known === name);
}

/** `VITE_DINAR_MODULES` unset, blank or carrying nothing this app knows
 *  falls back to `["retail"]` alone: the build every shop already has. */
export function builtModules(): readonly Module[] {
  const raw = import.meta.env.VITE_DINAR_MODULES;
  const wanted = (typeof raw === "string" ? raw : "")
    .split(",")
    .map((name) => name.trim())
    .filter(isModule);
  return wanted.length === 0 ? ["retail"] : wanted;
}

export function isModuleBuilt(module: Module): boolean {
  return builtModules().includes(module);
}
